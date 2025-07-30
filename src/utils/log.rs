use std::fs::File;
use std::io::stderr;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

use crate::board::zobrist::ZOBRIST;
use chrono::Local;
use miette::{Context, IntoDiagnostic};
use tracing::level_filters::LevelFilter;
use tracing::{Level, info};
use tracing_appender::non_blocking;
use tracing_subscriber::reload;
use tracing_subscriber::{
    EnvFilter, Layer, fmt, layer::SubscriberExt, reload::Handle, util::SubscriberInitExt,
};

pub trait LogHandle: Send + Sync {
    fn set_filter(&self, new_filter: EnvFilter) -> miette::Result<()>;
}

impl<S> LogHandle for Handle<EnvFilter, S>
where
    S: tracing::Subscriber + Send + Sync + 'static,
{
    fn set_filter(&self, new_filter: EnvFilter) -> miette::Result<()> {
        self.modify(|current| *current = new_filter)
            .into_diagnostic()
    }
}

pub struct LogHandles {
    console_handle: Mutex<Box<dyn LogHandle>>,
    file_handle: Mutex<Box<dyn LogHandle>>,
}

static LOG_HANDLES: LazyLock<LogHandles> = LazyLock::new(|| {
    color_backtrace::install();

    // Console Layer with its own reloadable filter
    let console_filter = EnvFilter::builder()
        .with_default_directive(Level::INFO.into())
        .from_env_lossy();
    let (console_filter, console_handle) = reload::Layer::new(console_filter);
    let console_layer = fmt::layer()
        .without_time()
        .with_writer(stderr)
        .with_filter(console_filter);

    // File Layer with its own reloadable filter (initially off)
    let file_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::OFF.into())
        .from_env_lossy();
    let (file_filter, file_handle) = reload::Layer::new(file_filter);

    let log_dir = Path::new("/tmp/eschec_logs");
    if !log_dir.exists() {
        std::fs::create_dir(log_dir).expect("Failed to create log directory");
    }

    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let log_filename = format!("/tmp/eschec_logs/eschec_{timestamp}.log");
    let log_file = File::create(&log_filename)
        .unwrap_or_else(|_| panic!("Failed to create log file: {log_filename}"));

    let (non_blocking_writer, _guard) = non_blocking(log_file);
    std::mem::forget(_guard); // Keep the guard alive.

    let file_layer = fmt::layer()
        .with_writer(non_blocking_writer)
        .with_ansi(false) // No colors in file
        .with_filter(file_filter);

    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    LogHandles {
        console_handle: Mutex::new(Box::new(console_handle)),
        file_handle: Mutex::new(Box::new(file_handle)),
    }
});

pub fn set_log_level(level: Level) -> miette::Result<()> {
    let new_filter = EnvFilter::new(level.to_string());

    LOG_HANDLES
        .console_handle
        .lock()
        .unwrap()
        .set_filter(new_filter)
        .with_context(|| format!("Failed to modify log filter to level: {level}"))
}

pub fn toggle_file_logging(enable: bool) -> miette::Result<()> {
    let new_filter = if enable {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("off")
    };

    LOG_HANDLES
        .file_handle
        .lock()
        .unwrap()
        .set_filter(new_filter)
        .context("Failed to modify log filter")
}

/// Initialize tracing and backtrace
pub fn init() {
    LazyLock::force(&LOG_HANDLES);
    LazyLock::force(&ZOBRIST);
    #[cfg(feature = "simd")]
    {
        info!("Simd Enabled, but nothing for now");
    }
    #[cfg(not(feature = "simd"))]
    {
        info!("Not using Simd");
    }
}
