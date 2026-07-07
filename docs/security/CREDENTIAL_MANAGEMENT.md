# Credential Management — velox-terminal

Almacenamiento seguro de credenciales de broker.

---

## Storage

**Nunca** almacenar credenciales en:
- Archivos de configuración en texto plano ❌
- Variables de entorno ❌ (aunque es mejor que texto plano, no es suficiente)
- El registro de Windows ❌
- `~/.config/velox-terminal/config.toml` ❌

**Siempre** usar el almacenamiento seguro del sistema operativo:

| OS | Backend | Crate |
|----|---------|-------|
| Linux | Secret Service (libsecret) / GNOME Keyring / KDE Wallet | `keyring` |
| macOS | Keychain | `keyring` |
| Windows | Credential Manager | `keyring` |

## Keyring Integration

```rust
use keyring::Entry;

fn store_credentials(api_key: &str, api_secret: &str) -> Result<(), Box<dyn std::error::Error>> {
    let entry = Entry::new("velox-terminal", "interactive-brokers")?;
    entry.set_password(&format!("{}:{}", api_key, api_secret))?;
    Ok(())
}

fn get_credentials() -> Result<(String, String), Box<dyn std::error::Error>> {
    let entry = Entry::new("velox-terminal", "interactive-brokers")?;
    let creds = entry.get_password()?;
    let mut parts = creds.splitn(2, ':');
    let api_key = parts.next().unwrap_or("").to_string();
    let api_secret = parts.next().unwrap_or("").to_string();
    Ok((api_key, api_secret))
}
```

## Runtime Security

- Las credenciales se mantienen en memoria como `SecretString` (zeroize on drop)
- Nunca se loguean en ningún nivel
- Nunca se serializan en dumps de crash
- Se borran explícitamente al hacer logout o cerrar sesión

## Credential Rotation

- El usuario puede rotar credenciales desde la UI sin reiniciar
- Al rotar, las viejas credenciales se sobrescriben en el keyring
- Las conexiones activas se reconectan con las nuevas credenciales
