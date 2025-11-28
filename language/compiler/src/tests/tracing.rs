use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::FormatFields;

/// Initialize tracing for tests.
/// Set RUST_LOG=trace (or debug, info, etc.) to see output.
pub fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_test_writer()
        .without_time()
        .with_target(false)
        .fmt_fields(TestFieldFormatter)
        .try_init();
}

struct TestFieldFormatter;

impl<'w> FormatFields<'w> for TestFieldFormatter {
    fn format_fields<R: tracing_subscriber::field::RecordFields>(
        &self,
        mut writer: Writer<'w>,
        fields: R,
    ) -> std::fmt::Result {
        let mut visitor = TestFieldVisitor {
            writer: &mut writer,
            first: true,
        };
        fields.record(&mut visitor);
        Ok(())
    }
}

struct TestFieldVisitor<'a, 'w> {
    writer: &'a mut Writer<'w>,
    first: bool,
}

impl<'a, 'w> tracing::field::Visit for TestFieldVisitor<'a, 'w> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if !self.first {
            write!(self.writer, " ").ok();
        }
        self.first = false;

        let name = field.name();
        // unwrap dynamic event name and inline args/kwargs directly
        if name == "message" || name == "event" || name == "args" || name == "kwargs" {
            write!(self.writer, "{value:?}").ok();
        }
        // regular dynamic fields
        else {
            write!(self.writer, "{name}={value:?}").ok();
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if !self.first {
            write!(self.writer, " ").ok();
        }
        self.first = false;

        let name = field.name();
        // unwrap dynamic event name and inline args/kwargs directly
        if name == "message" || name == "event" || name == "args" || name == "kwargs" {
            write!(self.writer, "{value}").ok();
        }
        // regular dynamic fields
        else {
            write!(self.writer, "{name}={value}").ok();
        }
    }
}
