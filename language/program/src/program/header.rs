use destack_core::SectionDirectory;
use destack_heap::{HeapOptions, SharedHeapOptions};
use destack_mir::TargetLayout;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{GlobalTable, StaticImage, native, vm};

use super::{
    DispatchTable, FrameTable, FunctionTable, LayoutTable, ProgramInfo, StringTable, TraceTable,
    TypeTable,
};

/// Root descriptor for one program.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ProgramHeader {
    /// Program section directory.
    pub sections: SectionDirectory,
    /// Target ABI layout used by program layouts and pointer-sized integer types.
    pub target_layout: TargetLayout,

    /// Local heap geometry used by lowered allocation plans.
    pub local_heap: HeapOptions,
    /// Shared heap geometry used by lowered allocation plans.
    pub shared_heap: SharedHeapOptions,

    /// Program string table.
    pub strings: StringTable,
    /// Runtime type table.
    pub types: TypeTable,
    /// Runtime layouts keyed by layout id.
    pub layouts: LayoutTable,
    /// Runtime frame table.
    pub frames: FrameTable,
    /// Program function table.
    pub functions: FunctionTable,
    /// Runtime dispatch table.
    pub dispatch: DispatchTable,
    /// Canonical trace table used by heap tables.
    pub traces: TraceTable,
    /// Program globals keyed by dense global id.
    pub globals: GlobalTable,
    /// Source reflection table when this program ships it.
    pub info: Option<ProgramInfo>,

    /// Immutable constant storage owned by this program.
    pub constant_space: StaticImage,
    /// Initial shared static storage for each runtime.
    pub shared_static_space: StaticImage,
    /// Initial local static storage for each worker.
    pub local_static_space: StaticImage,

    /// VM code used for interpretation, deoptimization, and continuation resume.
    pub vm: vm::Code,
    /// Native code used as optional acceleration.
    pub native: Option<native::Code>,
}

impl ProgramHeader {
    /// Create a program header.
    pub fn new(
        sections: SectionDirectory,
        target_layout: TargetLayout,
        local_heap: HeapOptions,
        shared_heap: SharedHeapOptions,
        strings: StringTable,
        types: TypeTable,
        layouts: LayoutTable,
        frames: FrameTable,
        functions: FunctionTable,
        dispatch: DispatchTable,
        traces: TraceTable,
        globals: GlobalTable,
        info: Option<ProgramInfo>,
        constant_space: StaticImage,
        shared_static_space: StaticImage,
        local_static_space: StaticImage,
        vm: vm::Code,
        native: Option<native::Code>,
    ) -> Self {
        Self {
            sections,
            target_layout,
            local_heap,
            shared_heap,
            strings,
            types,
            layouts,
            frames,
            functions,
            dispatch,
            traces,
            globals,
            info,
            constant_space,
            shared_static_space,
            local_static_space,
            vm,
            native,
        }
    }
}
