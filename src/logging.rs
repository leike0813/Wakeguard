use crate::error::WakeguardError;
use tracing_subscriber::EnvFilter;

pub fn init_logging() -> Result<(), WakeguardError> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("wakeguard=info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .try_init()
        .map_err(|e| WakeguardError::InvalidConfig(format!("logging init failed: {e}")))?;

    Ok(())
}
