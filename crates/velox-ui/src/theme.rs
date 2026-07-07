//! Dark trading terminal theme for egui.
//!
//! Provides a professional dark color scheme inspired by NinjaTrader/ATAS.

use egui::{Color32, CornerRadius, Stroke, Visuals};

/// Apply the dark trading theme to an egui context.
pub fn configure(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.visuals = Visuals {
        override_text_color: Some(Color32::from_gray(200)),
        window_stroke: Stroke::new(1.0, Color32::from_gray(60)),
        window_fill: Color32::from_rgb(18, 18, 24),
        panel_fill: Color32::from_rgb(18, 18, 24),
        faint_bg_color: Color32::from_rgb(24, 24, 32),
        extreme_bg_color: Color32::from_rgb(12, 12, 16),
        code_bg_color: Color32::from_rgb(24, 24, 32),
        warn_fg_color: Color32::from_rgb(255, 180, 0),
        error_fg_color: Color32::from_rgb(255, 80, 80),
        hyperlink_color: Color32::from_rgb(80, 160, 255),
        selection: egui::style::Selection {
            bg_fill: Color32::from_rgba_premultiplied(60, 120, 255, 80),
            stroke: Stroke::new(1.0, Color32::from_rgb(80, 160, 255)),
        },
        widgets: egui::style::Widgets {
            noninteractive: egui::style::WidgetVisuals {
                bg_fill: Color32::from_rgb(24, 24, 32),
                weak_bg_fill: Color32::from_rgb(30, 30, 40),
                bg_stroke: Stroke::new(1.0, Color32::from_rgb(50, 50, 60)),
                corner_radius: CornerRadius::same(2),
                fg_stroke: Stroke::new(1.0, Color32::from_gray(160)),
                expansion: 0.0,
            },
            inactive: egui::style::WidgetVisuals {
                bg_fill: Color32::from_rgb(30, 30, 40),
                weak_bg_fill: Color32::from_rgb(24, 24, 32),
                bg_stroke: Stroke::new(1.0, Color32::from_rgb(60, 60, 72)),
                corner_radius: CornerRadius::same(2),
                fg_stroke: Stroke::new(1.0, Color32::from_gray(180)),
                expansion: 0.0,
            },
            hovered: egui::style::WidgetVisuals {
                bg_fill: Color32::from_rgb(40, 40, 52),
                weak_bg_fill: Color32::from_rgb(35, 35, 45),
                bg_stroke: Stroke::new(1.0, Color32::from_rgb(80, 80, 100)),
                corner_radius: CornerRadius::same(2),
                fg_stroke: Stroke::new(1.5, Color32::from_gray(220)),
                expansion: 1.0,
            },
            active: egui::style::WidgetVisuals {
                bg_fill: Color32::from_rgb(50, 50, 65),
                weak_bg_fill: Color32::from_rgb(45, 45, 55),
                bg_stroke: Stroke::new(1.0, Color32::from_rgb(100, 100, 120)),
                corner_radius: CornerRadius::same(2),
                fg_stroke: Stroke::new(2.0, Color32::from_gray(255)),
                expansion: 1.0,
            },
            open: egui::style::WidgetVisuals {
                bg_fill: Color32::from_rgb(35, 35, 45),
                weak_bg_fill: Color32::from_rgb(30, 30, 40),
                bg_stroke: Stroke::new(1.0, Color32::from_rgb(70, 70, 85)),
                corner_radius: CornerRadius::same(2),
                fg_stroke: Stroke::new(1.5, Color32::from_gray(200)),
                expansion: 0.0,
            },
        },
        ..Default::default()
    };

    ctx.set_style(style);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_applies_without_panic() {
        let ctx = egui::Context::default();
        configure(&ctx);
        // Smoke test: just verify style was applied without panicking
        let _active_style = ctx.style();
    }
}
