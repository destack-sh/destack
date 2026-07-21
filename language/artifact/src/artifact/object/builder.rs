use destack_bytecode as bytecode;
use destack_mir as mir;
use destack_program::{native, wasm};
use destack_source::ModuleId;

use super::{
    AllocationSite, CallSite, CounterSite, EdgeSite, Frame, FrameState, Function, Global,
    MemorySite, Object, SampleSite, SuspensionSite, Type,
};

/// One emitted object under construction.
#[derive(Debug)]
pub struct ObjectBuilder {
    /// The object being assembled.
    object: Object,
}

impl ObjectBuilder {
    /// Create one empty emitted object for a target layout.
    pub fn new(target: mir::TargetLayout) -> Self {
        Self {
            object: Object {
                dependencies: Vec::new(),
                target,
                types: Vec::new(),
                layouts: mir::LayoutTable::new(),
                drops: mir::DropTable::new(),
                dispatch: mir::DispatchTable::new(),
                functions: Vec::new(),
                globals: Vec::new(),
                frames: Vec::new(),
                frame_states: Vec::new(),
                allocations: Vec::new(),
                memory: Vec::new(),
                calls: Vec::new(),
                edges: Vec::new(),
                suspensions: Vec::new(),
                counters: Vec::new(),
                samples: Vec::new(),
                bytecode: None,
                native: None,
                wasm: None,
            },
        }
    }

    /// Set directly referenced modules.
    pub fn dependencies(mut self, dependencies: impl IntoIterator<Item = ModuleId>) -> Self {
        self.object.dependencies = dependencies.into_iter().collect();
        self.object.dependencies.sort_unstable();
        self.object.dependencies.dedup();

        self
    }

    /// Set object-local type declarations.
    pub fn types(mut self, types: impl IntoIterator<Item = Type>) -> Self {
        self.object.types = types.into_iter().collect();

        self
    }

    /// Set object-local physical layouts.
    pub fn layouts(mut self, layouts: mir::LayoutTable) -> Self {
        self.object.layouts = layouts;

        self
    }

    /// Set object-local drop declarations.
    pub fn drops(mut self, drops: mir::DropTable) -> Self {
        self.object.drops = drops;

        self
    }

    /// Set object-local dispatch declarations.
    pub fn dispatch(mut self, dispatch: mir::DispatchTable) -> Self {
        self.object.dispatch = dispatch;

        self
    }

    /// Set object-local function declarations.
    pub fn functions(mut self, functions: impl IntoIterator<Item = Function>) -> Self {
        self.object.functions = functions.into_iter().collect();

        self
    }

    /// Set object-local global declarations and definitions.
    pub fn globals(mut self, globals: impl IntoIterator<Item = Global>) -> Self {
        self.object.globals = globals.into_iter().collect();

        self
    }

    /// Set logical function frame shapes.
    pub fn frames(mut self, frames: impl IntoIterator<Item = Frame>) -> Self {
        self.object.frames = frames.into_iter().collect();

        self
    }

    /// Set live frame states at managed safepoints.
    pub fn frame_states(mut self, states: impl IntoIterator<Item = FrameState>) -> Self {
        self.object.frame_states = states.into_iter().collect();

        self
    }

    /// Set heap allocation sites.
    pub fn allocations(mut self, sites: impl IntoIterator<Item = AllocationSite>) -> Self {
        self.object.allocations = sites.into_iter().collect();

        self
    }

    /// Set addressable memory sites.
    pub fn memory(mut self, sites: impl IntoIterator<Item = MemorySite>) -> Self {
        self.object.memory = sites.into_iter().collect();

        self
    }

    /// Set function call sites.
    pub fn calls(mut self, sites: impl IntoIterator<Item = CallSite>) -> Self {
        self.object.calls = sites.into_iter().collect();

        self
    }

    /// Set control flow edge sites.
    pub fn edges(mut self, sites: impl IntoIterator<Item = EdgeSite>) -> Self {
        self.object.edges = sites.into_iter().collect();

        self
    }

    /// Set suspension sites.
    pub fn suspensions(mut self, sites: impl IntoIterator<Item = SuspensionSite>) -> Self {
        self.object.suspensions = sites.into_iter().collect();

        self
    }

    /// Set explicit profile counter sites.
    pub fn counters(mut self, sites: impl IntoIterator<Item = CounterSite>) -> Self {
        self.object.counters = sites.into_iter().collect();

        self
    }

    /// Set explicit profile sample sites.
    pub fn samples(mut self, sites: impl IntoIterator<Item = SampleSite>) -> Self {
        self.object.samples = sites.into_iter().collect();

        self
    }

    /// Set relocatable bytecode.
    pub fn bytecode(mut self, bytecode: bytecode::Object) -> Self {
        self.object.bytecode = Some(bytecode);

        self
    }

    /// Set relocatable native code.
    pub fn native(mut self, native: native::Object) -> Self {
        self.object.native = Some(native);

        self
    }

    /// Set relocatable WebAssembly.
    pub fn wasm(mut self, wasm: wasm::Object) -> Self {
        self.object.wasm = Some(wasm);

        self
    }

    /// Build the relocatable object.
    pub fn build(self) -> Object {
        self.object
    }
}
