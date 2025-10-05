use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::SubscriberBuilder;

use crate::errors::Result;

/// Initializes the default tracing subscriber used across services.
pub fn init_tracing(level: Option<&str>) -> Result<()> {
    let default_level = level.unwrap_or("info");
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    SubscriberBuilder::default()
        .with_env_filter(filter)
        .with_target(true)
        .with_ansi(atty::is(atty::Stream::Stdout))
        .try_init()
        .map_err(|err| crate::errors::LogLineError::GeneralError(err.to_string()))?;

    Ok(())
}
