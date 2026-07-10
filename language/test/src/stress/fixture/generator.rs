use std::fmt;
use std::fmt::Write;

/// Shared source generator for stress fixtures.
#[derive(Debug)]
pub(super) struct Generator {
    /// The generated source text.
    source: String,
    /// The requested fixture scale.
    scale: usize,
    /// The requested line width.
    width: usize,
}

impl Generator {
    /// Create a source generator for one stress fixture variant.
    pub(super) fn new(scale: usize, width: usize, capacity: usize) -> Self {
        Self {
            source: String::with_capacity(capacity),
            scale,
            width,
        }
    }

    /// Return the requested fixture scale.
    pub(super) fn scale(&self) -> usize {
        self.scale
    }

    /// Return the requested line width.
    pub(super) fn width(&self) -> usize {
        self.width
    }

    /// Append source text.
    pub(super) fn emit(&mut self, text: &str) {
        self.source.push_str(text);
    }

    /// Append formatted source text.
    pub(super) fn write(&mut self, arguments: fmt::Arguments<'_>) {
        let _ = self.source.write_fmt(arguments);
    }

    /// Finish the generated source text.
    pub(super) fn finish(self) -> String {
        self.source
    }
}

/// Append formatted source text.
macro_rules! emit {
    ($generator:expr, $($argument:tt)*) => {{
        $generator.write(format_args!($($argument)*));
    }};
}

pub(super) use emit;
