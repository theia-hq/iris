//! Persisted node identity: a 32-byte ed25519 secret key stored on disk, so this node keeps the same
//! address across runs. Override the location with `--key` / `IRIS_KEY`; otherwise
//! `~/.config/iris/identity.key`.

use std::path::{Path, PathBuf};

use eyre::eyre;

/// Load the persisted secret key, creating and saving a fresh one on first run.
///
/// An explicit path (`--key`, which also backs `IRIS_KEY`) overrides the default location; with none, the
/// default `~/.config/iris/identity.key` applies.
pub async fn load_or_create(explicit: Option<&Path>) -> eyre::Result<[u8; 32]> {
    let path = match explicit {
        Some(path) => path.to_owned(),
        None => default_path()?,
    };
    if let Ok(bytes) = tokio::fs::read(&path).await
        && let Ok(secret) = <[u8; 32]>::try_from(bytes.as_slice())
    {
        return Ok(secret);
    }

    let secret: [u8; 32] = rand::random();
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&path, &secret).await?;
    restrict(&path).await?;
    Ok(secret)
}

/// The default persisted key location, `~/.config/iris/identity.key`.
fn default_path() -> eyre::Result<PathBuf> {
    let home = std::env::var_os("HOME").ok_or_else(|| eyre!("HOME is not set; pass --key"))?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("iris")
        .join("identity.key"))
}

#[cfg(unix)]
async fn restrict(path: &Path) -> eyre::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;

    tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).await?;
    Ok(())
}

#[cfg(not(unix))]
async fn restrict(_path: &Path) -> eyre::Result<()> {
    Ok(())
}
