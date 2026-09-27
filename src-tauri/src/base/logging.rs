//! Structured logging to `%LOCALAPPDATA%\ALXS-RL-Mod\logs` (daily rotation)
//! and to stderr in debug builds. `RUST_LOG` overrides the default filter.

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::base::paths;

const DEFAULT_FILTER: &str = "info,alxs_rl_mod_lib=debug";

/// Keep the returned guard alive for the whole process, otherwise buffered
/// log lines are dropped on exit.
pub fn init() -> Option<WorkerGuard> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
    let (file_layer, guard) = match paths::logs_dir() {
        Ok(dir) => {
            let appender = tracing_appender::rolling::daily(dir, "alxs-rl-mod.log");
            let (writer, guard) = tracing_appender::non_blocking(appender);
            (
                Some(fmt::layer().with_ansi(false).with_writer(writer)),
                Some(guard),
            )
        }
        Err(_) => (None, None),
    };
    let stderr_layer = cfg!(debug_assertions).then(fmt::layer);
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stderr_layer)
        .try_init();
    guard
}
