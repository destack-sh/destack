use destack_core::{
    EntryRange, EntryStore, Optional, SectionEntry, SectionImage, SectionPacker, SectionSlice,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{FrameStateId, FunctionId, TypeId};

/// Native code map for entries, safepoints, roots, and deoptimization.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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

impl CodeMap {
    /// Pack one native code map.
    pub fn pack(
        sections: &mut SectionPacker,
        function: Vec<FunctionCode>,
        resume: Vec<ResumeCode>,
        safepoint: Vec<Option<SafepointBuilder>>,
    ) -> Self {
        let mut roots = EntryStore::<NativeRoot>::new();
        let safepoint = safepoint
            .into_iter()
            .map(|safepoint| safepoint.map(|safepoint| safepoint.build(&mut roots)))
            .map(Optional::from)
            .collect::<Vec<_>>();

        Self {
            function: sections.insert(function),
            resume: sections.insert(resume),
            safepoint: sections.insert(safepoint),
            root: sections.insert(roots.into_entries()),
        }
    }

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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Safepoint {
    /// The safepoint id passed through the native ABI.
    pub id: u32,
    /// The function containing this safepoint.
    pub function: FunctionId,
    /// The byte offset from the native image base.
    pub offset: u32,
    /// The VM frame state corresponding to this safepoint.
    pub frame_state: FrameStateId,
    /// Native roots live at this safepoint.
    pub roots: EntryRange<NativeRoot>,
    /// Materialization target when this safepoint can deoptimize.
    pub deopt: Optional<FrameStateId>,
}

/// Mutable native safepoint before section flattening.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SafepointBuilder {
    /// The safepoint id passed through the native ABI.
    pub id: u32,
    /// The function containing this safepoint.
    pub function: FunctionId,
    /// The byte offset from the native image base.
    pub offset: u32,
    /// The VM frame state corresponding to this safepoint.
    pub frame_state: FrameStateId,
    /// Native roots live at this safepoint.
    pub roots: Vec<NativeRoot>,
    /// Materialization target when this safepoint can deoptimize.
    pub deopt: Option<FrameStateId>,
}

impl SafepointBuilder {
    /// Create one native safepoint.
    pub fn new(
        id: u32,
        function: FunctionId,
        offset: u32,
        frame_state: FrameStateId,
        roots: Vec<NativeRoot>,
        deopt: Option<FrameStateId>,
    ) -> Self {
        Self {
            id,
            function,
            offset,
            frame_state,
            roots,
            deopt,
        }
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct NativeRoot {
    /// Signed byte offset from the native frame base.
    pub offset: i32,
    /// The root value type.
    pub ty: TypeId,
}

// SAFETY: native code map entries contain only fixed-width ids, offsets, and section ranges.
unsafe impl SectionEntry for FunctionCode {}
unsafe impl SectionEntry for ResumeCode {}
unsafe impl SectionEntry for CodeRange {}
unsafe impl SectionEntry for Safepoint {}
unsafe impl SectionEntry for NativeRoot {}

impl NativeRoot {
    /// Create one native root location.
    pub fn new(offset: i32, ty: TypeId) -> Self {
        Self { offset, ty }
    }
}
