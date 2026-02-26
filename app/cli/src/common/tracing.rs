use std::fmt::Write;
use std::time::SystemTime;

use clap::{Args, ValueEnum};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{EnvFilter, fmt};

use crate::console;

/// The log level to use.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum LogLevelArg {
    /// Error level.
    Error,
    /// Warn level.
    Warn,
    /// Info level.
    #[default]
    Info,
    /// Debug level.
    Debug,
    /// Trace level.
    Trace,
}

impl From<LogLevelArg> for LevelFilter {
    fn from(level: LogLevelArg) -> Self {
        match level {
            LogLevelArg::Error => LevelFilter::ERROR,
            LogLevelArg::Warn => LevelFilter::WARN,
            LogLevelArg::Info => LevelFilter::INFO,
            LogLevelArg::Debug => LevelFilter::DEBUG,
            LogLevelArg::Trace => LevelFilter::TRACE,
        }
    }
}

/// Color mode for CLI output.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum ColorModeArg {
    /// Enable colors when the output supports it.
    #[default]
    Auto,
    /// Always emit ANSI colors.
    Always,
    /// Never emit ANSI colors.
    Never,
}

impl From<ColorModeArg> for console::ColorMode {
    fn from(mode: ColorModeArg) -> Self {
        match mode {
            ColorModeArg::Auto => console::ColorMode::Auto,
            ColorModeArg::Always => console::ColorMode::Always,
            ColorModeArg::Never => console::ColorMode::Never,
        }
    }
}

/// Arguments for configuring tracing.
#[derive(Args, Debug, Clone, Default)]
pub struct TracingArgs {
    /// Set log level (error|warn|info|debug|trace, default: off).
    #[arg(long = "log", value_enum, hide_possible_values = true, global = true)]
    pub log_level: Option<LogLevelArg>,

    /// Color mode (auto|always|never, default: auto).
    #[arg(long = "color", value_enum, hide_possible_values = true, global = true)]
    pub color: Option<ColorModeArg>,

    /// Quiet mode (minimal output).
    #[arg(short = 'q', long, global = true)]
    pub quiet: bool,

    /// Verbose mode (detailed output).
    #[arg(short = 'v', long, global = true)]
    pub verbose: bool,
}

impl TracingArgs {
    /// Apply console settings for color output.
    pub fn apply_console_settings(&self) {
        if let Some(mode) = self.color {
            console::set_color_mode(mode.into());
        }
    }

    /// Initialize the tracing subscriber with the configured log level.
    pub fn init(&self) {
        let Some(level) = self.log_level else {
            return;
        };
        let level_filter: LevelFilter = level.into();

        // allow RUST_LOG to override
        let env_filter = EnvFilter::builder()
            .with_default_directive(level_filter.into())
            .from_env_lossy();

        let fmt_layer = fmt::layer()
            .with_writer(std::io::stderr)
            .with_ansi(console::color_enabled(console::Stream::Stderr))
            .event_format(CustomFormat::new());

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();
    }
}

struct CustomFormat;

impl CustomFormat {
    fn new() -> Self {
        Self
    }
}

/// Format the level with ANSI color.
fn format_level(level: tracing::Level) -> String {
    let (label, codes) = match level {
        tracing::Level::ERROR => ("ERROR", &["1", "31"][..]),
        tracing::Level::WARN => ("WARN", &["1", "33"][..]),
        tracing::Level::INFO => ("INFO", &["1", "32"][..]),
        tracing::Level::DEBUG => ("DEBUG", &["1", "34"][..]),
        tracing::Level::TRACE => ("TRACE", &["1", "35"][..]),
    };

    console::style_for_stream(label, codes, console::Stream::Stderr)
}

/// Render dimmed text for stderr output.
fn dim_for_stderr(text: &str) -> String {
    console::style_for_stream(text, &["2"], console::Stream::Stderr)
}

impl<S, N> FormatEvent<S, N> for CustomFormat
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: fmt::format::Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> std::fmt::Result {
        // timestamp
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs();
        let millis = now.subsec_millis();

        // convert to time components (UTC)
        let secs_of_day = secs % 86400;
        let hours = secs_of_day / 3600;
        let minutes = (secs_of_day % 3600) / 60;
        let seconds = secs_of_day % 60;
        let timestamp = format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}");
        write!(writer, "{} ", dim_for_stderr(&timestamp))?;

        // level with color
        let level = *event.metadata().level();
        write!(writer, "{} ", format_level(level))?;

        // thread id (dim)
        let thread_id = std::thread::current().id().as_u64().get();
        write!(writer, "{} ", dim_for_stderr(&format!("T{thread_id:02}")))?;

        // message (first string field)
        let mut message_visitor = MessageVisitor::default();
        event.record(&mut message_visitor);
        if let Some(msg) = &message_visitor.message {
            write!(writer, "{msg}")?;
        }

        // event fields (excluding message)
        if !message_visitor.fields.is_empty() {
            write!(writer, " {}", dim_for_stderr(&message_visitor.fields))?;
        }

        writeln!(writer)
    }
}

/// Visitor to extract message and other fields from an event.
#[derive(Default)]
struct MessageVisitor {
    message: Option<String>,
    fields: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let name = field.name();
        // unwrap dynamic event name
        if name == "message" || name == "event" {
            self.message = Some(format!("{value:?}"));
        }
        // inline args/kwargs directly
        else if name == "args" || name == "kwargs" {
            if !self.fields.is_empty() {
                self.fields.push(' ');
            }
            write!(self.fields, "{value:?}").ok();
        }
        // regular dynamic fields
        else {
            if !self.fields.is_empty() {
                self.fields.push(' ');
            }
            write!(self.fields, "{name}={value:?}").ok();
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        let name = field.name();
        // unwrap dynamic event name
        if name == "message" || name == "event" {
            self.message = Some(value.to_string());
        }
        // inline args/kwargs directly
        else if name == "args" || name == "kwargs" {
            if !self.fields.is_empty() {
                self.fields.push(' ');
            }
            write!(self.fields, "{value}").ok();
        }
        // regular dynamic fields
        else {
            if !self.fields.is_empty() {
                self.fields.push(' ');
            }
            write!(self.fields, "{name}=\"{value}\"").ok();
        }
    }
}
