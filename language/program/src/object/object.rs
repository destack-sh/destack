use destack_bytecode as bytecode;
use destack_mir as mir;
use destack_native as native;
use destack_serde::Reflect;
use destack_source::ModuleId;
use destack_webassembly as wasm;
use serde::{Deserialize, Serialize};

use super::{
    AllocationSite, CallSite, CounterSite, EdgeSite, FrameState, Function, Global, MemorySite,
    SampleSite, Type,
};

/// One relocatable module linked into a Program.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct Object {
    /// Modules referenced directly by this object.
    pub(super) dependencies: Vec<ModuleId>,
    /// Target layout shared by every emitted code form.
    pub(super) target: mir::TargetLayout,

    /// Object-local type declarations in ascending MIR id order.
    pub(super) types: Vec<Type>,
    /// Object-local physical layouts.
    pub(super) layouts: mir::LayoutTable,
    /// Object-local drop declarations.
    pub(super) drops: mir::DropTable,
    /// Object-local dispatch declarations.
    pub(super) dispatch: mir::DispatchTable,
    /// Object-local function declarations in ascending MIR id order.
    pub(super) functions: Vec<Function>,
    /// The object-local module initializer when one exists.
    pub(super) initializer: Option<mir::FunctionId>,
    /// Object-local global declarations and definitions in ascending MIR id order.
    pub(super) globals: Vec<Global>,

    /// Logical frame states in object-local identity order.
    pub(super) frames: Vec<FrameState>,

    /// Heap allocation sites.
    pub(super) allocations: Vec<AllocationSite>,
    /// Addressable memory sites.
    pub(super) memory: Vec<MemorySite>,
    /// Function call sites.
    pub(super) calls: Vec<CallSite>,
    /// Control flow edges.
    pub(super) edges: Vec<EdgeSite>,
    /// Explicit profile counter sites.
    pub(super) counters: Vec<CounterSite>,
    /// Explicit profile sample sites.
    pub(super) samples: Vec<SampleSite>,

    /// Relocatable bytecode when emitted for this module.
    pub(super) bytecode: Option<bytecode::Object>,
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
        let index = self.types.binary_search_by_key(&id, |ty| ty.id).ok()?;

        self.types.get(index)
    }

    /// Return the transparent storage type for one object-local type.
    pub fn storage_type(&self, mut id: mir::TypeId) -> Option<mir::TypeId> {
        loop {
            let ty = &self.ty(id)?.definition;
            match ty {
                mir::Type::Application { base, .. }
                | mir::Type::Uninit { value: base }
                | mir::Type::ManuallyDrop { value: base }
                | mir::Type::Newtype { value: base, .. } => id = *base,
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

    /// Return the object-local module initializer when one exists.
    pub const fn initializer(&self) -> Option<mir::FunctionId> {
        self.initializer
    }

    /// Return one object-local function declaration.
    pub fn function(&self, id: mir::FunctionId) -> Option<&Function> {
        let index = self
            .functions
            .binary_search_by_key(&id, |function| function.id)
            .ok()?;

        self.functions.get(index)
    }

    /// Return object-local global declarations and definitions.
    pub fn globals(&self) -> &[Global] {
        &self.globals
    }

    /// Return logical frame states in object-local identity order.
    pub fn frames(&self) -> &[FrameState] {
        &self.frames
    }

    /// Return one object-local global declaration.
    pub fn global(&self, id: mir::GlobalId) -> Option<&Global> {
        let index = self
            .globals
            .binary_search_by_key(&id, |global| global.id)
            .ok()?;

        self.globals.get(index)
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

    /// Return explicit profile counter sites.
    pub fn counters(&self) -> &[CounterSite] {
        &self.counters
    }

    /// Return explicit profile sample sites.
    pub fn samples(&self) -> &[SampleSite] {
        &self.samples
    }

    /// Return relocatable bytecode when present.
    pub const fn bytecode(&self) -> Option<&bytecode::Object> {
        self.bytecode.as_ref()
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
