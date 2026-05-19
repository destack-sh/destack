use std::fmt;
use std::fmt::Write;

use super::StressMode;

/// Shared source generator for stress cases.
#[derive(Debug)]
pub(super) struct Generator {
    /// The generated source text.
    source: String,
    /// The generated file mode.
    mode: StressMode,
    /// The requested case scale.
    scale: usize,
    /// The requested line width.
    width: usize,
}

impl Generator {
    /// Create a source generator for one stress case variant.
    pub(super) fn new(mode: StressMode, scale: usize, width: usize, capacity: usize) -> Self {
        Self {
            source: String::with_capacity(capacity),
            mode,
            scale,
            width,
        }
    }

    /// Return the generated file mode.
    pub(super) fn mode(&self) -> StressMode {
        self.mode
    }

    /// Return the requested case scale.
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
