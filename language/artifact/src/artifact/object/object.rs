use destack_bytecode as bytecode;
use destack_mir as mir;
use destack_program::{native, wasm};
use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use super::{
    AllocationSite, CallSite, CounterSite, EdgeSite, Frame, FrameState, Function, Global,
    MemorySite, SampleSite, SuspensionSite, Type,
};

/// One relocatable module linked into a Program.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct Object {
    /// Modules referenced directly by this object.
    pub(super) dependencies: Vec<ModuleId>,
    /// Target layout shared by every emitted code form.
    pub(super) target: mir::TargetLayout,

    /// Object-local type declarations.
    pub(super) types: Vec<Type>,
    /// Object-local physical layouts.
    pub(super) layouts: mir::LayoutTable,
    /// Object-local drop declarations.
    pub(super) drops: mir::DropTable,
    /// Object-local dispatch declarations.
    pub(super) dispatch: mir::DispatchTable,
    /// Object-local function declarations.
    pub(super) functions: Vec<Function>,
    /// Object-local global declarations and definitions.
    pub(super) globals: Vec<Global>,

    /// Logical function frame shapes.
    pub(super) frames: Vec<Frame>,
    /// Live frame states at managed safepoints.
    pub(super) frame_states: Vec<FrameState>,

    /// Heap allocation sites.
    pub(super) allocations: Vec<AllocationSite>,
    /// Addressable memory sites.
    pub(super) memory: Vec<MemorySite>,
    /// Function call sites.
    pub(super) calls: Vec<CallSite>,
    /// Control flow edges.
    pub(super) edges: Vec<EdgeSite>,
    /// Suspension sites.
    pub(super) suspensions: Vec<SuspensionSite>,
    /// Explicit profile counter sites.
    pub(super) counters: Vec<CounterSite>,
    /// Explicit profile sample sites.
    pub(super) samples: Vec<SampleSite>,

    /// Relocatable bytecode for this module.
    pub(super) bytecode: bytecode::Object,
    /// Relocatable native code when emitted for this module.
    pub(super) native: Option<native::Object>,
    /// Relocatable WebAssembly when emitted for this module.
    pub(super) wasm: Option<wasm::Object>,
}

impl Object {
    /// Return directly referenced modules.
    pub fn dependencies(&self) -> &[ModuleId] {
        &self.dependencies
    }

    /// Return the target layout shared by every code form.
    pub const fn target(&self) -> mir::TargetLayout {
        self.target
    }

    /// Return object-local type declarations.
    pub fn types(&self) -> &[Type] {
        &self.types
    }

    /// Return one object-local type declaration.
    pub fn ty(&self, id: mir::TypeId) -> Option<&Type> {
        self.types.iter().find(|ty| ty.id == id)
    }

    /// Return the transparent storage type for one object-local type.
    pub fn storage_type(&self, mut id: mir::TypeId) -> Option<mir::TypeId> {
        loop {
            let ty = &self.ty(id)?.definition;
            match ty {
                mir::Type::WithLifetimes { base, .. }
                | mir::Type::Uninit { value: base }
                | mir::Type::Atomic { value: base }
                | mir::Type::ManuallyDrop { value: base }
                | mir::Type::Newtype { inner: base, .. } => id = *base,
                _ => return Some(id),
            }
        }
    }

    /// Return object-local physical layouts.
    pub const fn layouts(&self) -> &mir::LayoutTable {
        &self.layouts
    }

    /// Return object-local drop declarations.
    pub const fn drops(&self) -> &mir::DropTable {
        &self.drops
    }

    /// Return object-local dispatch declarations.
    pub const fn dispatch(&self) -> &mir::DispatchTable {
        &self.dispatch
    }

    /// Return object-local function declarations.
    pub fn functions(&self) -> &[Function] {
        &self.functions
    }

    /// Return one object-local function declaration.
    pub fn function(&self, id: mir::FunctionId) -> Option<&Function> {
        self.functions.iter().find(|function| function.id == id)
    }

    /// Return object-local global declarations and definitions.
    pub fn globals(&self) -> &[Global] {
        &self.globals
    }

    /// Return one object-local global declaration.
    pub fn global(&self, id: mir::GlobalId) -> Option<&Global> {
        self.globals.iter().find(|global| global.id == id)
    }

    /// Return logical function frame shapes.
    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    /// Return live frame states at managed safepoints.
    pub fn frame_states(&self) -> &[FrameState] {
        &self.frame_states
    }

    /// Return heap allocation sites.
    pub fn allocations(&self) -> &[AllocationSite] {
        &self.allocations
    }

    /// Return addressable memory sites.
    pub fn memory(&self) -> &[MemorySite] {
        &self.memory
    }

    /// Return function call sites.
    pub fn calls(&self) -> &[CallSite] {
        &self.calls
    }

    /// Return control flow edges.
    pub fn edges(&self) -> &[EdgeSite] {
        &self.edges
    }

    /// Return suspension sites.
    pub fn suspensions(&self) -> &[SuspensionSite] {
        &self.suspensions
    }

    /// Return explicit profile counter sites.
    pub fn counters(&self) -> &[CounterSite] {
        &self.counters
    }

    /// Return explicit profile sample sites.
    pub fn samples(&self) -> &[SampleSite] {
        &self.samples
    }

    /// Return relocatable bytecode.
    pub const fn bytecode(&self) -> &bytecode::Object {
        &self.bytecode
    }

    /// Return relocatable native code when present.
    pub const fn native(&self) -> Option<&native::Object> {
        self.native.as_ref()
    }

    /// Return relocatable WebAssembly when present.
    pub const fn wasm(&self) -> Option<&wasm::Object> {
        self.wasm.as_ref()
    }
}
