use std::fmt::Write;
use std::time::SystemTime;

use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{EnvFilter, fmt};

/// Initialize tracing for tests.
/// Set RUST_LOG=trace (or debug, info, etc.) to see output.
pub fn init_tracing() {
    let _ = tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(
            fmt::layer()
                .with_test_writer()
                .event_format(TestFormat)
                .fmt_fields(TestFieldFormat),
        )
        .try_init();
}

/// Custom event format for tests: [level] [message] [fields]
struct TestFormat;

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

impl<S, N> FormatEvent<S, N> for TestFormat
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
        // timestamp (dim)
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs();
        let millis = now.subsec_millis();
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

        // collect message and fields
        let mut visitor = TestFieldVisitor {
            message: None,
            fields: String::new(),
        };
        event.record(&mut visitor);

        // message
        if let Some(msg) = &visitor.message {
            write!(writer, "{msg}")?;
        }

        // fields (dim)
        if !visitor.fields.is_empty() {
            write!(writer, " \x1b[2m{}\x1b[0m", visitor.fields)?;
        }

        writeln!(writer)
    }
}

struct TestFieldFormat;

impl<'w> FormatFields<'w> for TestFieldFormat {
    fn format_fields<R: tracing_subscriber::field::RecordFields>(
        &self,
        mut writer: Writer<'w>,
        fields: R,
    ) -> std::fmt::Result {
        let mut visitor = TestFieldVisitor {
            message: None,
            fields: String::new(),
        };
        fields.record(&mut visitor);
        if let Some(msg) = &visitor.message {
            write!(writer, "{msg}")?;
        }
        if !visitor.fields.is_empty() {
            if visitor.message.is_some() {
                write!(writer, " ")?;
            }
            write!(writer, "{}", visitor.fields)?;
        }
        Ok(())
    }
}

struct TestFieldVisitor {
    message: Option<String>,
    fields: String,
}

impl tracing::field::Visit for TestFieldVisitor {
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
            write!(self.fields, "{name}={value}").ok();
        }
    }
}
