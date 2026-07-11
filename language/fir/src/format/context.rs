use std::num::NonZeroU32;

use destack_source::{File, FileType};

use crate::format::{Allocator, FormatOptions, GroupId, SimpleFormatOptions};

/// Shared state for one formatting pass.
pub struct FormatState<'a, Context> {
    context: Context,
    allocator: &'a Allocator,
    next_group_id: u32,
}

impl<Context> std::fmt::Debug for FormatState<'_, Context>
where
    Context: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormatState")
            .field("context", &self.context)
            .field("next_group_id", &self.next_group_id)
            .finish()
    }
}

impl<'a, Context> FormatState<'a, Context> {
    /// Create formatter state over one context and arena.
    pub fn new(context: Context, allocator: &'a Allocator) -> Self {
        Self {
            context,
            allocator,
            next_group_id: 1,
        }
    }

    /// Return the formatter arena.
    pub fn allocator(&self) -> &'a Allocator {
        self.allocator
    }

    /// Return the context and discard the remaining state.
    pub fn into_context(self) -> Context {
        self.context
    }

    /// Return the formatting context.
    pub fn context(&self) -> &Context {
        &self.context
    }

    /// Return the formatting context mutably.
    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    /// Create a group ID unique within this document.
    ///
    /// The passed debug name is used in the [`std::fmt::Debug`] of the document if this is a debug build.
    /// The name is unused for production builds and has no meaning on the equality of two group ids.
    pub fn group_id(&mut self, debug_name: &'static str) -> GroupId {
        let id = self.next_group_id;
        self.next_group_id += 1;
        let id = NonZeroU32::new(id).expect("ID overflowed");
        GroupId::new(id, debug_name)
    }
}

/// Language-specific state required while formatting.
pub trait FormatContext {
    type Options: FormatOptions;

    /// Return the formatting options.
    fn options(&self) -> &Self::Options;

    /// Return the source file.
    fn file(&self) -> &File;
}

#[derive(Debug, PartialEq, Clone)]
pub struct SimpleFormatContext {
    options: SimpleFormatOptions,
    file: File,
}

impl SimpleFormatContext {
    /// Create a new SimpleFormatContext with the given options and source.
    pub fn new(options: SimpleFormatOptions, source: File) -> Self {
        Self {
            options,
            file: source,
        }
    }

    /// Create an empty SimpleFormatContext.
    pub fn empty_destack() -> Self {
        Self {
            options: SimpleFormatOptions::default(),
            file: File::empty_text(FileType::Destack),
        }
    }
}

impl FormatContext for SimpleFormatContext {
    type Options = SimpleFormatOptions;

    fn options(&self) -> &Self::Options {
        &self.options
    }

    fn file(&self) -> &File {
        &self.file
    }
}
