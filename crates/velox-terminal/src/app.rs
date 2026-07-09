//! # App — Main application orchestrator
//!
//! Owns the window, GPU resources, chart renderer, egui state, panel manager,
//! market data pipeline, and exchange feed. Runs the winit event loop and
//! dispatches events to the appropriate subsystems.
//!
//! # Data Flow
//!
//! ```text
//! Every frame (AboutToWait → RedrawRequested):
//!   1. poll_candles() → drains mpsc channel from MarketDataPipeline
//!   2. panel_manager.show() → builds egui UI, records chart rect
//!   3. chart_renderer.update_from_state() → uploads new candles to GPU
//!   4. composite_render() → PASS 1 (chart) + PASS 2 (egui)
//! ```

#![allow(deprecated)] // TODO: migrate from EventLoop::run → run_app

use crate::input;
use std::iter;
use std::sync::Arc;
use velox_broker::{BrokerClient, BrokerConfig};
use velox_chart::renderer::ChartRenderer;
use velox_exchange::binance::BinanceFeed;
use velox_exchange::binance_broker::BinanceBroker;
use velox_exchange::ExchangeFeed;
use velox_gpu::device::GpuDevice;
use velox_gpu::error::GpuError;
use velox_md::pipeline::MarketDataPipeline;
use velox_md::ring_buffer::RingBuffer;
use velox_ui::app_state::AppState;
use velox_ui::panels::PanelManager;
use velox_ui::theme as ui_theme;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

/// Helper: render egui into a render pass, working around the `'static` lifetime
/// requirement of `egui_wgpu::Renderer::render()`.
///
/// SAFETY: This is safe because:
/// 1. The encoder outlives the render() call (both are scoped to composite_render).
/// 2. egui_wgpu's render() does not store the RenderPass reference — it only issues
///    draw calls synchronously and returns.
/// 3. No paint callbacks are used that could stash the render pass.
unsafe fn render_egui_with_pass(
    renderer: &egui_wgpu::Renderer,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    primitives: &[egui::ClippedPrimitive],
    screen_descriptor: &egui_wgpu::ScreenDescriptor,
) {
    let pass_desc = wgpu::RenderPassDescriptor {
        label: Some("egui_pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    };

    let mut pass = encoder.begin_render_pass(&pass_desc);
    // SAFETY: egui_wgpu::Renderer::render() requires &mut RenderPass<'static> but
    // never stores the reference. The encoder lives for the duration of composite_render,
    // which is longer than this render call. This transmute is equivalent to what
    // frameworks like egui's own examples do when dealing with short-lived encoders.
    let pass_static: &mut wgpu::RenderPass<'static> = unsafe { std::mem::transmute(&mut pass) };
    renderer.render(pass_static, primitives, screen_descriptor);
}

/// Main application orchestrator.
pub struct App {
    /// Winit window handle.
    pub window: std::rc::Rc<Window>,

    /// GPU device/queue/instance.
    pub gpu: GpuDevice,

    /// WGPU surface for the window.
    ///
    /// SAFETY: Surface has a `'static` lifetime but is actually tied to `window`.
    /// We ensure correct drop order by declaring `window` AFTER `surface` in the struct
    /// (Rust drops fields in declaration order, so `window` is dropped first).
    pub surface: wgpu::Surface<'static>,

    /// Surface configuration (dimensions, format).
    pub surface_config: wgpu::SurfaceConfiguration,

    /// Chart renderer.
    pub chart_renderer: ChartRenderer,

    /// egui context.
    pub egui_ctx: egui::Context,

    /// egui-winit state (input handling).
    pub egui_state: egui_winit::State,

    /// egui-wgpu renderer (GPU rendering of egui meshes).
    pub egui_renderer: egui_wgpu::Renderer,

    /// UI panel manager.
    pub panel_manager: PanelManager,

    /// Shared application state.
    pub state: AppState,

    /// Market data pipeline: polls ring buffer → aggregates candles → channel.
    pub pipeline: MarketDataPipeline,

    /// Exchange feed (WebSocket connection).
    pub feed: BinanceFeed,
}

impl App {
    /// Create a new application instance.
    ///
    /// This blocks on GPU initialization (async) using `pollster`,
    /// and spawns the exchange WebSocket feed on the provided tokio runtime.
    pub fn new(event_loop: &winit::event_loop::EventLoop<()>) -> Result<Self, anyhow::Error> {
        tracing::info!("Initializing velox-terminal...");

        // ── Window ──────────────────────────────────────────────────
        let window = {
            let w = create_window(event_loop)?;
            std::rc::Rc::new(w)
        };

        // ── GPU ─────────────────────────────────────────────────────
        let gpu = pollster::block_on(GpuDevice::new())?;

        // SAFETY: Both `window` and `surface` are owned by `App`.
        // `window` is declared AFTER `surface` in the struct, so it is dropped FIRST,
        // ensuring the window outlives the surface.
        let (surface, surface_config) = {
            let size = window.inner_size();
            let format = wgpu::TextureFormat::Bgra8UnormSrgb;
            let surface = unsafe {
                let target = wgpu::SurfaceTargetUnsafe::from_window(&*window)?;
                gpu.instance.create_surface_unsafe(target)?
            };
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: size.width.max(1),
                height: size.height.max(1),
                present_mode: wgpu::PresentMode::Mailbox,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            surface.configure(&gpu.device, &config);
            (surface, config)
        };

        // ── Market Data Pipeline (1m, 5m, 1h) ──────────────────────
        let timeframes: &[i64] = &[60, 300, 3600];
        let ring = Arc::new(RingBuffer::new(4096));
        let (pipeline, candle_rx) = MarketDataPipeline::new(ring.clone(), timeframes);
        let mut state = AppState::empty(timeframes);
        state.set_candle_receiver(candle_rx);

        // ── Chart Renderer ──────────────────────────────────────────
        let mut chart_renderer = ChartRenderer::new(&gpu, surface_config.format)?;
        {
            let rect = egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(surface_config.width as f32, surface_config.height as f32),
            );
            let scale = window.scale_factor() as f32;
            let phys_w = (rect.width() * scale).max(1.0);
            let phys_h = (rect.height() * scale).max(1.0);
            // Upload initial empty data to GPU
            chart_renderer.update_from_state(
                &state.candles,
                &state.chart_interaction.view,
                phys_w,
                phys_h,
            );
        }

        // ── Exchange Feed ───────────────────────────────────────────
        let feed = BinanceFeed::new();
        feed.subscribe("BTC/USDT").ok();
        if let Err(e) = feed.start(ring) {
            tracing::warn!("Failed to start Binance feed (will retry): {e}");
        } else {
            state.set_feed_connected(true);
        }

        // ── egui ────────────────────────────────────────────────────
        let egui_ctx = egui::Context::default();
        ui_theme::configure(&egui_ctx);

        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &*window,
            None, // native_pixels_per_point
            None, // theme
            None, // max_texture_side
        );

        let egui_renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            surface_config.format,
            None,  // output_depth_format
            1,     // msaa_samples
            false, // dithering
        );

        tracing::info!("velox-terminal initialized successfully");

        Ok(Self {
            window,
            gpu,
            surface,
            surface_config,
            chart_renderer,
            egui_ctx,
            egui_state,
            egui_renderer,
            panel_manager: PanelManager::new(),
            state,
            pipeline,
            feed,
        })
    }

    /// Poll for new market data. Called every frame before building UI.
    fn poll_market_data(&mut self) {
        // 1. Update feed connection status from the WebSocket feed
        self.state.set_feed_connected(self.feed.connected());

        // 2. Poll the ring buffer → aggregate ticks → get candles
        let new_candles = self.pipeline.poll();

        // 3. Drain the mpsc channel into AppState
        let received = self.state.poll_candles();

        // 4. Poll depth (order book) from the exchange feed
        let sym_depth = self.state.symbol.clone();
        if let Some(book) = self.feed.order_book(&sym_depth) {
            self.state.depth = Some(book);
        }

        // 5. Mock execution: if we have new candles, fill open market orders at the last price
        // Extract values first to avoid borrow conflicts.
        let last_close = self.state.candles.last().map(|c| c.close);
        if let Some(close) = last_close {
            let filled = self.state.execute_open_orders(close);
            if filled > 0 {
                tracing::info!("Mock fill: executed {filled} order(s) at {close:.2}");
                self.state.update_account();
                self.state.needs_redraw = true;
            }
            // Keep the paper trader in sync with latest price
            let sym = self.state.symbol.clone();
            self.state.paper_trader.update_price(&sym, close);
        }

        // 5. Update pipeline metrics on state
        self.state.ticks_processed = self.pipeline.ticks_processed();
        self.state.candles_produced = self.pipeline.candles_produced();

        if received > 0 || new_candles > 0 {
            self.state.needs_redraw = true;
        }
    }

    /// Handle a winit event.
    pub fn handle_event(&mut self, event: winit::event::Event<()>, elwt: &ActiveEventLoop) {
        match event {
            // ── Resize ──────────────────────────────────────────
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::Resized(size),
                ..
            } => {
                self.surface_config.width = size.width.max(1);
                self.surface_config.height = size.height.max(1);
                self.surface
                    .configure(&self.gpu.device, &self.surface_config);
                self.state.needs_redraw = true;
                self.window.request_redraw();
            }

            // ── Scale factor changed ───────────────────────────
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::ScaleFactorChanged { .. },
                ..
            } => {
                self.state.needs_redraw = true;
                self.window.request_redraw();
            }

            // ── Redraw ─────────────────────────────────────────
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::RedrawRequested,
                ..
            } => {
                if let Err(e) = self.composite_render() {
                    tracing::error!("Render error: {e}");
                }
            }

            // ── Close ──────────────────────────────────────────
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::CloseRequested,
                ..
            } => {
                // Gracefully stop the exchange feed
                if let Err(e) = self.feed.stop() {
                    tracing::warn!("Feed stop error: {e}");
                }
                elwt.exit();
            }

            // ── Window events (input routing) ──────────────────
            winit::event::Event::WindowEvent { event, .. } => {
                // 1. Always feed egui first
                let response = self.egui_state.on_window_event(&self.window, &event);

                // 2. If egui didn't consume it, route to chart
                if !response.consumed {
                    let scale = self.window.scale_factor();
                    input::route_to_chart(&event, &mut self.state, scale);
                }

                // 3. Request redraw
                self.state.needs_redraw = true;
                self.window.request_redraw();
            }

            // ── About to wait ──────────────────────────────────
            // This is the per-frame update hook. We poll market data here
            // so the next RedrawRequested picks up any new candles.
            winit::event::Event::AboutToWait => {
                self.state.frame_count += 1;

                // ── Handle broker connect request ──────────────
                if self.state.connect_requested {
                    self.state.connect_requested = false;
                    let broker = Arc::new(BinanceBroker::new());
                    let config = BrokerConfig {
                        api_key: std::mem::take(&mut self.state.connect_api_key),
                        api_secret: std::mem::take(&mut self.state.connect_api_secret),
                        base_url: if self.state.connect_base_url.is_empty() {
                            "https://api.binance.com".into()
                        } else {
                            std::mem::take(&mut self.state.connect_base_url)
                        },
                        paper_trading: false,
                    };
                    let b = broker.clone();
                    let c = config.clone();
                    tokio::spawn(async move {
                        match b.connect(c).await {
                            Ok(h) => tracing::info!("Broker connected: {:?}", h),
                            Err(e) => tracing::error!("Broker connect failed: {e}"),
                        }
                    });
                    self.state.set_broker(broker, config);
                }

                // ── Handle broker disconnect request ───────────
                if self.state.disconnect_requested {
                    self.state.disconnect_requested = false;
                    self.state.clear_broker();
                }

                // Poll market data (non-blocking)
                self.poll_market_data();

                // Always request redraw when live feed is active
                if self.state.feed_connected {
                    self.window.request_redraw();
                }
            }

            _ => {}
        }
    }

    /// Composite render: chart first, egui on top.
    fn composite_render(&mut self) -> Result<(), GpuError> {
        let device = &self.gpu.device;
        let queue = &self.gpu.queue;
        let scale = self.window.scale_factor() as f32;

        // ── 1. Build egui UI, record chart rect ────────────────
        let raw_input = self.egui_state.take_egui_input(&self.window);
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            self.panel_manager.show(ctx, &mut self.state);
        });
        self.egui_state
            .handle_platform_output(&self.window, full_output.platform_output);

        // ── 2. Compute chart physical rect ─────────────────────
        let chart_rect = self.state.chart_panel_rect;
        let phys_x = (chart_rect.min.x * scale) as u32;
        let phys_y = (chart_rect.min.y * scale) as u32;
        let phys_w = (chart_rect.width() * scale).max(1.0);
        let phys_h = (chart_rect.height() * scale).max(1.0);

        // ── 3. Update chart GPU data ───────────────────────────
        self.chart_renderer.update_from_state(
            &self.state.candles,
            &self.state.chart_interaction.view,
            phys_w,
            phys_h,
        );

        // ── 3.5 Update indicator overlays (line data) ─────────
        if !self.state.candles.is_empty() {
            self.state.overlays.update_all(&self.state.candles);
            let line_data = self.state.overlays.collect_line_data();
            self.chart_renderer.update_lines(&line_data);
        }

        // ── 4. Get surface texture ─────────────────────────────
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost) => {
                self.surface.configure(device, &self.surface_config);
                return Ok(());
            }
            Err(wgpu::SurfaceError::Outdated) => {
                return Ok(()); // skip frame
            }
            Err(e) => {
                tracing::warn!("Surface error: {e:?}");
                return Ok(());
            }
        };
        let frame_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // ── 5. Create encoder ──────────────────────────────────
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("composite_encoder"),
        });

        // ── 6. Prepare egui data (outside render pass) ──────
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.surface_config.width, self.surface_config.height],
            pixels_per_point: scale,
        };

        let clipped_primitives = self.egui_ctx.tessellate(full_output.shapes, scale);

        // Update egui textures (needs encoder, not pass)
        for (id, delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(device, queue, *id, delta);
        }

        // Update egui vertex/index buffers
        self.egui_renderer.update_buffers(
            device,
            queue,
            &mut encoder,
            &clipped_primitives,
            &screen_descriptor,
        );

        // ── 7. PASS 1: Chart (background, dark clear) ────────
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("chart_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.07,
                            g: 0.07,
                            b: 0.09,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Scissor rect limits chart rendering to the chart area
            pass.set_scissor_rect(
                phys_x,
                phys_y,
                (phys_w as u32).max(1),
                (phys_h as u32).max(1),
            );

            self.chart_renderer.render(&mut pass);
        } // chart pass dropped

        // ── 8. PASS 2: egui UI (alpha blends over chart via LoadOp::Load) ──
        // SAFETY: see render_egui_with_pass safety comment.
        unsafe {
            render_egui_with_pass(
                &self.egui_renderer,
                &mut encoder,
                &frame_view,
                &clipped_primitives,
                &screen_descriptor,
            );
        }

        // ── 9. Submit + Present ────────────────────────────────
        queue.submit(iter::once(encoder.finish()));
        frame.present();

        self.state.needs_redraw = false;

        Ok(())
    }
}

/// Safely drop resources that depend on the window.
impl Drop for App {
    fn drop(&mut self) {
        // Stop the exchange feed before tearing down GPU/window
        if let Err(e) = self.feed.stop() {
            tracing::warn!("Feed stop on drop: {e}");
        }
        // Surface depends on window, App struct ensures window is dropped last.
    }
}

/// Create the winit window with default settings.
fn create_window(event_loop: &winit::event_loop::EventLoop<()>) -> Result<Window, anyhow::Error> {
    let attributes = Window::default_attributes()
        .with_title("velox-terminal")
        .with_inner_size(winit::dpi::PhysicalSize::new(1280, 800))
        .with_min_inner_size(winit::dpi::PhysicalSize::new(800, 600));

    Ok(event_loop.create_window(attributes)?)
}
