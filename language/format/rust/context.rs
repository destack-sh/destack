use std::num::NonZeroU32;
use std::sync::atomic::Ordering;

use dyst_language_source::Source;

use crate::{GroupId, IndentStyle, PrintOptions};

/// This structure stores the state that is relevant for the formatting of the whole document.
///
/// This structure is different from [`crate::Formatter`] in that the formatting infrastructure
/// creates a new [`crate::Formatter`] for every [`crate::write`!] call, whereas this structure stays alive
/// for the whole process of formatting a root with [`crate::format`!].
#[derive(Debug, Default)]
pub struct FormatState<Context> {
    context: Context,
    next_group_id: std::sync::atomic::AtomicU32,
}

impl<Context> FormatState<Context> {
    /// Creates a new state with the given language specific context
    pub fn new(context: Context) -> Self {
        Self {
            context,
            next_group_id: std::sync::atomic::AtomicU32::new(1),
        }
    }

    pub fn into_context(self) -> Context {
        self.context
    }

    /// Returns the context specifying how to format the current CST
    pub fn context(&self) -> &Context {
        &self.context
    }

    /// Returns a mutable reference to the context
    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    /// Creates a new group id that is unique to this document. The passed debug name is used in the
    /// [`std::fmt::Debug`] of the document if this is a debug build.
    /// The name is unused for production builds and has no meaning on the equality of two group ids.
    pub fn group_id(&self, debug_name: &'static str) -> GroupId {
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
    fn source(&self) -> &Source;
}

pub trait FormatOptions {
    /// The indent style.
    fn indent_style(&self) -> IndentStyle;

    /// The visual width of an indent
    fn indent_width(&self) -> u8;

    /// What's the max width of a line. Defaults to 80.
    fn line_width(&self) -> u8;

    /// Derives the print options from these format options
    fn as_print_options(&self) -> PrintOptions;
}

#[derive(Debug, Default, Eq, PartialEq, Clone)]
pub struct SimpleFormatOptions {
    pub indent_style: IndentStyle,
    pub indent_width: u8,
    pub line_width: u8,
}

impl FormatOptions for SimpleFormatOptions {
    fn indent_style(&self) -> IndentStyle {
        self.indent_style
    }

    fn indent_width(&self) -> u8 {
        self.indent_width
    }

    fn line_width(&self) -> u8 {
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

#[derive(Debug, Default, PartialEq)]
pub struct SimpleFormatContext {
    options: SimpleFormatOptions,
    source: Source,
}

impl SimpleFormatContext {
    pub fn new(options: SimpleFormatOptions, source: Source) -> Self {
        Self { options, source }
    }
}

impl FormatContext for SimpleFormatContext {
    type Options = SimpleFormatOptions;

    fn options(&self) -> &Self::Options {
        &self.options
    }

    fn source(&self) -> &Source {
        &self.source
    }
}
