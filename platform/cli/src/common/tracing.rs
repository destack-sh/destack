use std::fmt::Write;
use std::time::SystemTime;

use clap::{Args, ValueEnum};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{EnvFilter, fmt};

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

/// Arguments for configuring tracing.
#[derive(Args, Debug, Clone, Default)]
pub struct TracingArgs {
    /// Set log level (error|warn|info|debug|trace, default: off).
    #[arg(long = "log", value_enum, global = true)]
    pub log_level: Option<LogLevelArg>,

    /// Quiet mode (minimal output).
    #[arg(short = 'q', long, global = true)]
    pub quiet: bool,

    /// Verbose mode (detailed output).
    #[arg(short = 'v', long, global = true)]
    pub verbose: bool,
}

impl TracingArgs {
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
            .with_ansi(true)
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
fn format_level(level: tracing::Level) -> &'static str {
    match level {
        tracing::Level::ERROR => "\x1b[1;31mERROR\x1b[0m",
        tracing::Level::WARN => "\x1b[1;33mWARN\x1b[0m ",
        tracing::Level::INFO => "\x1b[1;32mINFO\x1b[0m ",
        tracing::Level::DEBUG => "\x1b[1;34mDEBUG\x1b[0m",
        tracing::Level::TRACE => "\x1b[1;35mTRACE\x1b[0m",
    }
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
        write!(
            writer,
            "\x1b[2m{hours:02}:{minutes:02}:{seconds:02}.{millis:03}\x1b[0m "
        )?;

        // level with color
        let level = *event.metadata().level();
        write!(writer, "{} ", format_level(level))?;

        // thread id (dim)
        let thread_id = std::thread::current().id().as_u64().get();
        write!(writer, "\x1b[2mT{thread_id:02}\x1b[0m ")?;

        // message (first string field)
        let mut message_visitor = MessageVisitor::default();
        event.record(&mut message_visitor);
        if let Some(msg) = &message_visitor.message {
            write!(writer, "{msg}")?;
        }

        // event fields (excluding message)
        if !message_visitor.fields.is_empty() {
            write!(writer, " \x1b[2m{}\x1b[0m", message_visitor.fields)?;
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
