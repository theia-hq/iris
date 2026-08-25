//! Persisted node identity: a 32-byte ed25519 secret key stored on disk, so this node keeps the same
//! address across runs. Override the location with `IRIS_KEY`; otherwise `~/.config/iris/identity.key`.

use std::path::{Path, PathBuf};

use eyre::eyre;

/// Load the persisted secret key, creating and saving a fresh one on first run.
pub async fn load_or_create() -> eyre::Result<[u8; 32]> {
    let path = key_path()?;
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

fn key_path() -> eyre::Result<PathBuf> {
    if let Some(path) = std::env::var_os("IRIS_KEY") {
        return Ok(PathBuf::from(path));
    }
    let home = std::env::var_os("HOME").ok_or_else(|| eyre!("HOME is not set; set IRIS_KEY"))?;
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
