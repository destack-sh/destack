use std::sync::atomic::Ordering;

use crate::{FormatOptions, GroupId};

/// This structure stores the state that is relevant for the formatting of the whole document.
///
/// This structure is different from [`crate::Formatter`] in that the formatting infrastructure
/// creates a new [`crate::Formatter`] for every [`crate::write`!] call, whereas this structure stays alive
/// for the whole process of formatting a root with [`crate::format`!].
pub struct FormatState<Context> {
    context: Context,
    next_group_id: std::sync::atomic::AtomicU32,
}

impl<Context> FormatState<Context> {
    /// Creates a new FormatState with the given context.
    pub fn new(context: Context) -> Self {
        Self {
            context,
            // Start with 1 because `GroupId` wraps a `NonZeroU32` to reduce memory usage.
            next_group_id: std::sync::atomic::AtomicU32::new(1),
        }
    }

    /// Gets a reference to the context.
    pub fn context(&self) -> &Context {
        &self.context
    }

    /// Creates a new unique group id with the given debug name.
    pub(crate) fn acquire_group_id(&self, debug_name: &'static str) -> GroupId {
        let id = self.next_group_id.fetch_add(1, Ordering::Relaxed);
        let id = NonZeroU32::new(id).unwrap_or_else(|| panic!("ID overflowed"));
        GroupId::new(id, debug_name)
    }
}

/// Context object storing data relevant when formatting an object.
pub trait FormatContext {
    type Options: FormatOptions;

    /// Returns the formatting options
    fn options(&self) -> &Self::Options;

    /// Returns the source code from the document that gets formatted.
    fn source_code(&self) -> SourceCode<'_>;
}

#[derive(Debug, Default, Eq, PartialEq, Clone)]
pub struct SimpleFormatOptions {
    pub indent_style: IndentStyle,
    pub indent_width: IndentWidth,
    pub line_width: LineWidth,
}

impl FormatOptions for SimpleFormatOptions {
    fn indent_style(&self) -> IndentStyle {
        self.indent_style
    }

    fn indent_width(&self) -> IndentWidth {
        self.indent_width
    }

    fn line_width(&self) -> LineWidth {
        self.line_width
    }

    fn as_print_options(&self) -> PrintOptions {
        PrintOptions {
            line_width: self.line_width,
            indent_style: self.indent_style,
            indent_width: self.indent_width,
            ..PrintOptions::default()
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct SimpleFormatContext {
    options: SimpleFormatOptions,
    source_code: String,
}

impl SimpleFormatContext {
    pub fn new(options: FormatOptions) -> Self {
        Self {
            options,
            source_code: String::new(),
        }
    }

    #[must_use]
    pub fn with_source_code(mut self, code: &str) -> Self {
        self.source_code = String::from(code);
        self
    }
}

impl FormatContext for SimpleFormatContext {
    type Options = SimpleFormatOptions;

    fn options(&self) -> &Self::Options {
        &self.options
    }

    fn source_code(&self) -> SourceCode<'_> {
        SourceCode::new(&self.source_code)
    }
}
