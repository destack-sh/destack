use destack_core::{
    EntryRange, EntryStore, Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{FrameStateId, FunctionId, TypeId};

/// Native code map for entries, safepoints, roots, and deoptimization.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct CodeMap {
    /// Native function code ranges.
    function: SectionSlice<FunctionCode>,
    /// Native continuation resume code ranges.
    resume: SectionSlice<ResumeCode>,
    /// Native safepoints keyed by safepoint id.
    safepoint: SectionSlice<Optional<Safepoint>>,
    /// Native roots referenced by safepoints.
    root: SectionSlice<NativeRoot>,
}

/// Build-time native code map.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodeMapBuilder {
    /// Native function code ranges.
    functions: Vec<FunctionCode>,
    /// Native continuation resume code ranges.
    resumes: Vec<ResumeCode>,
    /// Native safepoints keyed by safepoint id.
    safepoints: Vec<Option<SafepointBuilder>>,
}

impl CodeMapBuilder {
    /// Create an empty native code map builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set native function code ranges.
    pub fn functions(mut self, functions: impl IntoIterator<Item = FunctionCode>) -> Self {
        self.functions = functions.into_iter().collect();

        self
    }

    /// Set native continuation resume code ranges.
    pub fn resumes(mut self, resumes: impl IntoIterator<Item = ResumeCode>) -> Self {
        self.resumes = resumes.into_iter().collect();

        self
    }

    /// Set native safepoints in dense id order.
    pub fn safepoints(
        mut self,
        safepoints: impl IntoIterator<Item = Option<SafepointBuilder>>,
    ) -> Self {
        self.safepoints = safepoints.into_iter().collect();

        self
    }

    /// Build this code map into program sections.
    pub(super) fn build(self, sections: &mut SectionBuilder) -> CodeMap {
        let mut roots = EntryStore::<NativeRoot>::new();
        let safepoints = self
            .safepoints
            .into_iter()
            .map(|safepoint| safepoint.map(|safepoint| safepoint.build(&mut roots)))
            .map(Optional::from)
            .collect::<Vec<_>>();

        CodeMap {
            function: sections.insert(self.functions),
            resume: sections.insert(self.resumes),
            safepoint: sections.insert(safepoints),
            root: sections.insert(roots.into_entries()),
        }
    }
}

impl CodeMap {
    /// Create one empty native code map.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Return native function code ranges.
    pub fn functions<'a>(&self, sections: SectionImage<'a>) -> &'a [FunctionCode] {
        sections.entries(self.function)
    }

    /// Return native resume code ranges.
    pub fn resumes<'a>(&self, sections: SectionImage<'a>) -> &'a [ResumeCode] {
        sections.entries(self.resume)
    }

    /// Return native safepoints.
    pub fn safepoints<'a>(&self, sections: SectionImage<'a>) -> &'a [Optional<Safepoint>] {
        sections.entries(self.safepoint)
    }

    /// Return one safepoint by id.
    pub fn safepoint(&self, sections: SectionImage<'_>, id: u32) -> Option<Safepoint> {
        sections
            .entries(self.safepoint)
            .get(id as usize)
            .and_then(|safepoint| safepoint.get())
    }

    /// Return native roots for one safepoint.
    pub fn roots<'a>(&self, sections: SectionImage<'a>, safepoint: Safepoint) -> &'a [NativeRoot] {
        safepoint.roots.slice(sections.entries(self.root))
    }
}

/// One native function code range.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct FunctionCode {
    /// The function covered by this range.
    pub function: FunctionId,
    /// The native code byte range.
    pub range: CodeRange,
}

impl FunctionCode {
    /// Create one native function code range.
    pub fn new(function: FunctionId, range: CodeRange) -> Self {
        Self { function, range }
    }
}

/// One native resume entry code range.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct ResumeCode {
    /// The frame state resumed by this range.
    pub frame_state: FrameStateId,
    /// The native code byte range.
    pub range: CodeRange,
}

impl ResumeCode {
    /// Create one native resume code range.
    pub fn new(frame_state: FrameStateId, range: CodeRange) -> Self {
        Self { frame_state, range }
    }
}

/// One native image byte range.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct CodeRange {
    /// The byte offset from the native image base.
    pub offset: u32,
    /// The byte length of this range.
    pub byte_len: u32,
}

impl CodeRange {
    /// Create one native image byte range.
    pub const fn new(offset: u32, byte_len: u32) -> Self {
        Self { offset, byte_len }
    }
}

/// One native safepoint.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Safepoint {
    /// The safepoint id passed through the native ABI.
    pub id: u32,
    /// The function containing this safepoint.
    pub function: FunctionId,
    /// The byte offset from the native image base.
    pub offset: u32,
    /// The runtime frame state corresponding to this safepoint.
    pub frame_state: FrameStateId,
    /// Native roots live at this safepoint.
    pub roots: EntryRange<NativeRoot>,
    /// Materialization target when this safepoint can deoptimize.
    pub deopt: Optional<FrameStateId>,
}

/// Mutable native safepoint before section flattening.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafepointBuilder {
    /// The safepoint id passed through the native ABI.
    id: u32,
    /// The function containing this safepoint.
    function: FunctionId,
    /// The byte offset from the native image base.
    offset: u32,
    /// The runtime frame state corresponding to this safepoint.
    frame_state: FrameStateId,
    /// Native roots live at this safepoint.
    roots: Vec<NativeRoot>,
    /// Materialization target when this safepoint can deoptimize.
    deopt: Option<FrameStateId>,
}

impl SafepointBuilder {
    /// Create one native safepoint.
    pub fn new(id: u32, function: FunctionId, offset: u32, frame_state: FrameStateId) -> Self {
        Self {
            id,
            function,
            offset,
            frame_state,
            roots: Vec::new(),
            deopt: None,
        }
    }

    /// Set native roots live at this safepoint.
    pub fn roots(mut self, roots: impl IntoIterator<Item = NativeRoot>) -> Self {
        self.roots = roots.into_iter().collect();

        self
    }

    /// Set the materialization target.
    pub fn deopt(mut self, deopt: FrameStateId) -> Self {
        self.deopt = Some(deopt);

        self
    }

    /// Build this safepoint into one section entry.
    fn build(self, roots: &mut EntryStore<NativeRoot>) -> Safepoint {
        let roots = roots.append(self.roots);

        Safepoint {
            id: self.id,
            function: self.function,
            offset: self.offset,
            frame_state: self.frame_state,
            roots,
            deopt: self.deopt.into(),
        }
    }
}

/// One native root location at one safepoint.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct NativeRoot {
    /// Signed byte offset from the native frame base.
    pub offset: i32,
    /// The root value type.
    pub ty: TypeId,
}

impl NativeRoot {
    /// Create one native root location.
    pub fn new(offset: i32, ty: TypeId) -> Self {
        Self { offset, ty }
    }
}
