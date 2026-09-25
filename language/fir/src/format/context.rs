use std::num::NonZeroU32;

use tspp_source::{File, FileType};

use crate::format::{
    Allocator, ConditionalGroup, FitsExpanded, FitsExpandedIndex, FitsExpandedState, FormatOptions,
    Group, GroupId, GroupIndex, GroupState, SimpleFormatOptions,
};

/// Shared state for one formatting pass.
pub struct FormatState<'a, Context> {
    /// The language-specific formatting context.
    context: Context,
    /// The arena that owns FIR storage.
    allocator: &'a Allocator,
    /// The next nonzero externally referenced group identifier.
    next_group_id: u32,
    /// The logical group rows written so far.
    groups: Vec<GroupState>,
    /// The fits-expanded rows written so far.
    fits_expanded: Vec<FitsExpandedState>,
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
            groups: Vec::new(),
            fits_expanded: Vec::new(),
        }
    }

    /// Return the formatter arena.
    pub fn allocator(&self) -> &'a Allocator {
        self.allocator
    }

    /// Complete this formatting pass.
    pub(crate) fn finish(self) -> (Context, Vec<GroupState>, Vec<FitsExpandedState>) {
        (self.context, self.groups, self.fits_expanded)
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
    pub fn group_id(&mut self) -> GroupId {
        // allocate the next dense nonzero identifier
        let id = self.next_group_id;
        self.next_group_id += 1;

        // safety: one document cannot hold enough group instructions to exhaust u32
        let id = unsafe { NonZeroU32::new_unchecked(id) };

        GroupId::new(id)
    }

    /// Append one regular group row.
    pub(crate) fn push_group(&mut self, group: Group) -> GroupIndex {
        let index = self.groups.len() as u32;
        self.groups.push(GroupState::regular(group));

        GroupIndex::new(index)
    }

    /// Append one conditional group row.
    pub(crate) fn push_conditional_group(&mut self, group: ConditionalGroup) -> GroupIndex {
        let index = self.groups.len() as u32;
        self.groups.push(GroupState::conditional(group));

        GroupIndex::new(index)
    }

    /// Append one fits-expanded row.
    pub(crate) fn push_fits_expanded(&mut self, fits: FitsExpanded) -> FitsExpandedIndex {
        let index = self.fits_expanded.len() as u32;
        self.fits_expanded.push(FitsExpandedState::new(fits));

        FitsExpandedIndex::new(index)
    }
}

/// Language-specific state required while formatting.
pub trait FormatContext {
    /// The language-specific formatting options.
    type Options: FormatOptions;

    /// Return the formatting options.
    fn options(&self) -> &Self::Options;

    /// Return the source file.
    fn file(&self) -> &File;
}

/// Minimal formatting context for FIR tests and standalone consumers.
#[derive(Debug, PartialEq, Clone)]
pub struct SimpleFormatContext {
    /// The formatting options.
    options: SimpleFormatOptions,
    /// The source file.
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
    pub fn empty_tspp() -> Self {
        Self {
            options: SimpleFormatOptions::default(),
            file: File::empty_text(FileType::Tspp),
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
