use tspp_bytecode as bytecode;
use tspp_mir as mir;
use tspp_native as native;
use tspp_source::ModuleId;
use tspp_webassembly as wasm;

use super::{
    AllocationSite, CallSite, CounterSite, EdgeSite, FrameState, Function, Global, MemorySite,
    Object, SampleSite, Type,
};

/// One emitted object under construction.
#[derive(Debug)]
pub struct ObjectBuilder {
    /// Modules referenced directly by this object.
    dependencies: Vec<ModuleId>,
    /// Target layout shared by every emitted code form.
    target: mir::TargetLayout,

    /// Object-local type declarations in ascending MIR id order.
    types: Vec<Type>,
    /// Object-local physical layouts.
    layouts: mir::LayoutTable,
    /// Object-local drop declarations.
    drops: mir::DropTable,
    /// Object-local dispatch declarations.
    dispatch: mir::DispatchTable,
    /// Object-local function declarations in ascending MIR id order.
    functions: Vec<Function>,
    /// The object-local module initializer when one exists.
    initializer: Option<mir::FunctionId>,
    /// Object-local global declarations and definitions in ascending MIR id order.
    globals: Vec<Global>,

    /// Logical frame states in object-local identity order.
    frames: Vec<FrameState>,

    /// Heap allocation sites.
    allocations: Vec<AllocationSite>,
    /// Addressable memory sites.
    memory: Vec<MemorySite>,
    /// Function call sites.
    calls: Vec<CallSite>,
    /// Control flow edges.
    edges: Vec<EdgeSite>,
    /// Explicit profile counter sites.
    counters: Vec<CounterSite>,
    /// Explicit profile sample sites.
    samples: Vec<SampleSite>,

    /// Relocatable bytecode when emitted for this module.
    bytecode: Option<bytecode::Object>,
    /// Relocatable native code when emitted for this module.
    native: Option<native::Object>,
    /// Relocatable WebAssembly when emitted for this module.
    wasm: Option<wasm::Object>,
}

impl ObjectBuilder {
    /// Create one empty emitted object for a target layout.
    pub fn new(target: mir::TargetLayout) -> Self {
        Self {
            dependencies: Vec::new(),
            target,
            types: Vec::new(),
            layouts: mir::LayoutTable::new(),
            drops: mir::DropTable::new(),
            dispatch: mir::DispatchTable::new(),
            functions: Vec::new(),
            initializer: None,
            globals: Vec::new(),
            frames: Vec::new(),
            allocations: Vec::new(),
            memory: Vec::new(),
            calls: Vec::new(),
            edges: Vec::new(),
            counters: Vec::new(),
            samples: Vec::new(),
            bytecode: None,
            native: None,
            wasm: None,
        }
    }

    /// Set directly referenced modules.
    pub fn dependencies(mut self, dependencies: impl IntoIterator<Item = ModuleId>) -> Self {
        self.dependencies = dependencies.into_iter().collect();
        self.dependencies.sort_unstable();
        self.dependencies.dedup();

        self
    }

    /// Set object-local type declarations in ascending MIR id order.
    pub fn types(mut self, types: impl IntoIterator<Item = Type>) -> Self {
        self.types = types.into_iter().collect();

        self
    }

    /// Set object-local physical layouts.
    pub fn layouts(mut self, layouts: mir::LayoutTable) -> Self {
        self.layouts = layouts;

        self
    }

    /// Set object-local drop declarations.
    pub fn drops(mut self, drops: mir::DropTable) -> Self {
        self.drops = drops;

        self
    }

    /// Set object-local dispatch declarations.
    pub fn dispatch(mut self, dispatch: mir::DispatchTable) -> Self {
        self.dispatch = dispatch;

        self
    }

    /// Set object-local function declarations in ascending MIR id order.
    pub fn functions(mut self, functions: impl IntoIterator<Item = Function>) -> Self {
        self.functions = functions.into_iter().collect();

        self
    }

    /// Set the object-local module initializer.
    pub fn initializer(mut self, initializer: Option<mir::FunctionId>) -> Self {
        self.initializer = initializer;

        self
    }

    /// Set object-local global declarations and definitions in ascending MIR id order.
    pub fn globals(mut self, globals: impl IntoIterator<Item = Global>) -> Self {
        self.globals = globals.into_iter().collect();

        self
    }

    /// Set logical frame states in object-local identity order.
    pub fn frames(mut self, frames: impl IntoIterator<Item = FrameState>) -> Self {
        self.frames = frames.into_iter().collect();

        self
    }

    /// Set heap allocation sites.
    pub fn allocations(mut self, sites: impl IntoIterator<Item = AllocationSite>) -> Self {
        self.allocations = sites.into_iter().collect();

        self
    }

    /// Set addressable memory sites.
    pub fn memory(mut self, sites: impl IntoIterator<Item = MemorySite>) -> Self {
        self.memory = sites.into_iter().collect();

        self
    }

    /// Set function call sites.
    pub fn calls(mut self, sites: impl IntoIterator<Item = CallSite>) -> Self {
        self.calls = sites.into_iter().collect();

        self
    }

    /// Set control flow edge sites.
    pub fn edges(mut self, sites: impl IntoIterator<Item = EdgeSite>) -> Self {
        self.edges = sites.into_iter().collect();

        self
    }

    /// Set explicit profile counter sites.
    pub fn counters(mut self, sites: impl IntoIterator<Item = CounterSite>) -> Self {
        self.counters = sites.into_iter().collect();

        self
    }

    /// Set explicit profile sample sites.
    pub fn samples(mut self, sites: impl IntoIterator<Item = SampleSite>) -> Self {
        self.samples = sites.into_iter().collect();

        self
    }

    /// Set relocatable bytecode.
    pub fn bytecode(mut self, bytecode: bytecode::Object) -> Self {
        self.bytecode = Some(bytecode);

        self
    }

    /// Set relocatable native code.
    pub fn native(mut self, native: native::Object) -> Self {
        self.native = Some(native);

        self
    }

    /// Set relocatable WebAssembly.
    pub fn wasm(mut self, wasm: wasm::Object) -> Self {
        self.wasm = Some(wasm);

        self
    }

    /// Build the relocatable object.
    pub fn build(self) -> Object {
        Object {
            dependencies: self.dependencies,
            target: self.target,
            types: self.types,
            layouts: self.layouts,
            drops: self.drops,
            dispatch: self.dispatch,
            functions: self.functions,
            initializer: self.initializer,
            globals: self.globals,
            frames: self.frames,
            allocations: self.allocations,
            memory: self.memory,
            calls: self.calls,
            edges: self.edges,
            counters: self.counters,
            samples: self.samples,
            bytecode: self.bytecode,
            native: self.native,
            wasm: self.wasm,
        }
    }
}
