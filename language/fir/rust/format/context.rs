use std::num::NonZeroU32;
use std::sync::atomic::Ordering;

use dyst_source::{Source, SourceFormat};

use crate::format::{FormatOptions, GroupId, SimpleFormatOptions};

/// Stores the state that is relevant for the formatting of the whole document.
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
    /// Create a new state with the given language specific context.
    pub fn new(context: Context) -> Self {
        Self {
            context,
            next_group_id: std::sync::atomic::AtomicU32::new(1),
        }
    }

    /// Convert this state into its inner context.
    pub fn into_context(self) -> Context {
        self.context
    }

    /// Get the context specifying how to format the current AST.
    pub fn context(&self) -> &Context {
        &self.context
    }

    /// Get a mutable reference to the context.
    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    /// Create a new group id that is unique to this document.
    ///
    /// The passed debug name is used in the [`std::fmt::Debug`] of the document if this is a debug build.
    /// The name is unused for production builds and has no meaning on the equality of two group ids.
    pub fn group_id(&self, debug_name: &'static str) -> GroupId {
        let id = self.next_group_id.fetch_add(1, Ordering::Relaxed);
        let id = NonZeroU32::new(id).expect("ID overflowed");
        GroupId::new(id, debug_name)
    }
}

/// Context object storing data relevant when formatting an object.
pub trait FormatContext {
    type Options: FormatOptions;

    /// Get the formatting options.
    fn options(&self) -> &Self::Options;

    /// Get the source code from the document that gets formatted.
    fn source(&self) -> &Source;
}

#[derive(Debug, PartialEq, Clone)]
pub struct SimpleFormatContext {
    options: SimpleFormatOptions,
    source: Source,
}

impl SimpleFormatContext {
    /// Create a new SimpleFormatContext with the given options and source.
    pub fn new(options: SimpleFormatOptions, source: Source) -> Self {
        Self { options, source }
    }

    /// Create an empty SimpleFormatContext.
    pub fn empty_dyst() -> Self {
        Self {
            options: SimpleFormatOptions::default(),
            source: Source::empty(SourceFormat::Dyst),
        }
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
