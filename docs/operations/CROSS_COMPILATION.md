# Cross Compilation — velox-terminal

Compilación cruzada para Windows/macOS/Linux.

---

## Targets

| Platform | Target Triple | Backend wgpu | Installer |
|----------|--------------|--------------|-----------|
| Windows x64 | `x86_64-pc-windows-msvc` | DirectX 12 | MSI (WiX Toolset) |
| macOS Intel | `x86_64-apple-darwin` | Metal | DMG |
| macOS Apple Silicon | `aarch64-apple-darwin` | Metal | DMG (universal) |
| Linux x64 | `x86_64-unknown-linux-gnu` | Vulkan | AppImage |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | Vulkan | AppImage |

## Toolchain Setup

```bash
# Install targets
rustup target add x86_64-pc-windows-msvc
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
rustup target add x86_64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-gnu

# Install cross-compilation tools
cargo install cross  # For Docker-based cross-compilation
```

## Cross-compilation with cargo-cross

```bash
# Linux → Windows
cross build --target x86_64-pc-windows-msvc --release

# Linux → macOS (Intel)
cross build --target x86_64-apple-darwin --release

# Linux → macOS (Apple Silicon)
cross build --target aarch64-apple-darwin --release

# Native builds
cargo build --target x86_64-unknown-linux-gnu --release
```

## Platform-Specific Notes

### Windows
- Usar `x86_64-pc-windows-msvc` (MSVC toolchain) para mejor compatibilidad
- El backend DirectX 12 requiere Windows 10+
- Fallback a Vulkan en Windows si DX12 no está disponible

### macOS
- Universal binary: `lipo -create -output velox-terminal x86_64-apple-darwin/release/velox-terminal aarch64-apple-darwin/release/velox-terminal`
- Notarización obligatoria para distribución fuera de la App Store

### Linux
- AppImage: empaqueta el binario + dependencias en un solo archivo portable
- deb: para distribución en Debian/Ubuntu
- wgpu requiere Vulkan (drivers: amdvlk o mesa-vulkan-drivers)
