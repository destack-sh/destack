use destack_core::{
    EntryRange, EntryStore, Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{FrameStateId, FunctionId, TypeId};

use super::{FrameLocation, FrameMap, FrameMapBuilder, FrameValue};

/// Native code map for entries, safepoints, roots, and deoptimization.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct CodeMap {
    /// Native function code ranges.
    function: SectionSlice<FunctionCode>,
    /// Native safepoints keyed by safepoint id.
    safepoint: SectionSlice<Optional<Safepoint>>,
    /// Native roots referenced by safepoints.
    root: SectionSlice<NativeRoot>,
    /// Canonical native frame values.
    value: SectionSlice<FrameValue>,
    /// Physical locations containing canonical frame values.
    location: SectionSlice<FrameLocation>,
    /// Immutable bytes referenced by constant frame locations.
    constant: SectionSlice<u8>,
}

/// Build-time native code map.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodeMapBuilder {
    /// Native function code ranges.
    functions: Vec<FunctionCode>,
    /// Native safepoints keyed by safepoint id.
    safepoints: Vec<Option<SafepointBuilder>>,
    /// Immutable bytes referenced by constant frame locations.
    constants: Vec<u8>,
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

    /// Set native safepoints in dense id order.
    pub fn safepoints(
        mut self,
        safepoints: impl IntoIterator<Item = Option<SafepointBuilder>>,
    ) -> Self {
        self.safepoints = safepoints.into_iter().collect();

        self
    }

    /// Set immutable bytes referenced by constant frame locations.
    pub fn constants(mut self, constants: impl IntoIterator<Item = u8>) -> Self {
        self.constants = constants.into_iter().collect();

        self
    }

    /// Build this code map into program sections.
    pub(super) fn build(self, sections: &mut SectionBuilder) -> CodeMap {
        let mut roots = EntryStore::<NativeRoot>::new();
        let mut values = EntryStore::<FrameValue>::new();
        let mut locations = EntryStore::<FrameLocation>::new();
        let safepoints = self
            .safepoints
            .into_iter()
            .map(|safepoint| {
                safepoint.map(|safepoint| safepoint.build(&mut roots, &mut values, &mut locations))
            })
            .map(Optional::from)
            .collect::<Vec<_>>();

        CodeMap {
            function: sections.insert(self.functions),
            safepoint: sections.insert(safepoints),
            root: sections.insert(roots.into_entries()),
            value: sections.insert(values.into_entries()),
            location: sections.insert(locations.into_entries()),
            constant: sections.insert(self.constants),
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

    /// Return canonical values in one native frame map.
    pub fn values<'a>(&self, sections: SectionImage<'a>, frame: FrameMap) -> &'a [FrameValue] {
        frame.values(sections.entries(self.value))
    }

    /// Return physical locations containing one canonical frame value.
    pub fn locations<'a>(
        &self,
        sections: SectionImage<'a>,
        value: FrameValue,
    ) -> &'a [FrameLocation] {
        value.locations(sections.entries(self.location))
    }

    /// Return immutable bytes referenced by constant frame locations.
    pub fn constants<'a>(&self, sections: SectionImage<'a>) -> &'a [u8] {
        sections.entries(self.constant)
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
    /// Physical projection of the canonical frame state.
    pub frame: FrameMap,
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
    /// Physical projection of the canonical frame state.
    frame: FrameMapBuilder,
    /// Materialization target when this safepoint can deoptimize.
    deopt: Option<FrameStateId>,
}

impl SafepointBuilder {
    /// Create one native safepoint.
    pub fn new(
        id: u32,
        function: FunctionId,
        offset: u32,
        frame_state: FrameStateId,
        frame: FrameMapBuilder,
    ) -> Self {
        Self {
            id,
            function,
            offset,
            frame_state,
            roots: Vec::new(),
            frame,
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
    fn build(
        self,
        roots: &mut EntryStore<NativeRoot>,
        values: &mut EntryStore<FrameValue>,
        locations: &mut EntryStore<FrameLocation>,
    ) -> Safepoint {
        let roots = roots.append(self.roots);
        let frame = self.frame.build(values, locations);

        Safepoint {
            id: self.id,
            function: self.function,
            offset: self.offset,
            frame_state: self.frame_state,
            roots,
            frame,
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

#[cfg(test)]
mod tests {
    use destack_core::{SectionBuilder, SectionImage};

    use super::*;
    use crate::native::{FrameSource, FrameValueBuilder};

    /// Preserve complete canonical frame projections in native code maps.
    #[test]
    fn test_build_native_frame_map() {
        let value = FrameValueBuilder::new().locations([
            FrameLocation::new(FrameSource::Register, 3, 0, 8),
            FrameLocation::new(FrameSource::Stack, -16, 8, 8),
        ]);
        let frame = FrameMapBuilder::new().values([value]);
        let safepoint = SafepointBuilder::new(7, FunctionId(2), 24, FrameStateId(4), frame)
            .roots([NativeRoot::new(-16, TypeId(3))]);
        let mut sections = SectionBuilder::new();
        let map = CodeMapBuilder::new()
            .safepoints([Some(safepoint)])
            .constants([1, 2, 3, 4])
            .build(&mut sections);
        let storage = sections.build();
        let sections = SectionImage::new(&storage);

        // recover each physical piece in canonical value order
        let safepoint = map
            .safepoint(sections, 0)
            .expect("native safepoint should exist");
        let [value] = map.values(sections, safepoint.frame) else {
            panic!("native frame should contain one canonical value");
        };
        assert_eq!(
            map.locations(sections, *value),
            &[
                FrameLocation::new(FrameSource::Register, 3, 0, 8),
                FrameLocation::new(FrameSource::Stack, -16, 8, 8),
            ]
        );
        assert_eq!(
            map.roots(sections, safepoint),
            &[NativeRoot::new(-16, TypeId(3))]
        );
        assert_eq!(map.constants(sections), &[1, 2, 3, 4]);
    }
}
