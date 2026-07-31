use destack_core::{EntryRange, EntryStore, Optional, SectionEntry, StringId};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::abi;

/// Program identities assigned to one relocatable native module object.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Module {
    /// Native symbol imported by this module object.
    pub symbol: StringId,
    /// Program function identities keyed by native object-local function index.
    functions: EntryRange<abi::Function>,
    /// Optional Program type ids keyed by object-local type index.
    types: EntryRange<Optional<u32>>,
    /// Optional Program layout ids keyed by object-local type index.
    layouts: EntryRange<Optional<u32>>,
    /// Program global ids keyed by object-local global index.
    globals: EntryRange<u32>,
    /// Program dynamic-table ids keyed by object-local dispatch-table index.
    dynamics: EntryRange<u32>,
    /// First Program allocation-site id owned by this module.
    pub allocation: u32,
    /// First native frame-map id owned by this module.
    pub frame_map: u32,
}

impl Module {
    /// Return whether every identity range fits its shared column.
    pub(super) fn ranges_fit(
        self,
        functions: usize,
        types: usize,
        layouts: usize,
        globals: usize,
        dynamics: usize,
    ) -> bool {
        self.functions.fits(functions)
            && self.types.fits(types)
            && self.layouts.fits(layouts)
            && self.globals.fits(globals)
            && self.dynamics.fits(dynamics)
    }

    /// Return Program function identities in native object-local order.
    pub fn functions(self, functions: &[abi::Function]) -> &[abi::Function] {
        self.functions.slice(functions)
    }

    /// Return one linked function.
    pub fn function(self, functions: &[abi::Function], local: u32) -> Option<abi::Function> {
        self.functions.slice(functions).get(local as usize).copied()
    }

    /// Return optional Program type ids in object-local order.
    pub fn types(self, types: &[Optional<u32>]) -> &[Optional<u32>] {
        self.types.slice(types)
    }

    /// Return one Program type id.
    pub fn ty(self, types: &[Optional<u32>], local: u32) -> Option<u32> {
        self.types
            .slice(types)
            .get(local as usize)
            .and_then(|ty| ty.get())
    }

    /// Return optional Program layout ids in object-local type order.
    pub fn layouts(self, layouts: &[Optional<u32>]) -> &[Optional<u32>] {
        self.layouts.slice(layouts)
    }

    /// Return one Program layout id.
    pub fn layout(self, layouts: &[Optional<u32>], local: u32) -> Option<u32> {
        self.layouts
            .slice(layouts)
            .get(local as usize)
            .and_then(|layout| layout.get())
    }

    /// Return Program global ids in object-local order.
    pub fn globals(self, globals: &[u32]) -> &[u32] {
        self.globals.slice(globals)
    }

    /// Return one Program global id.
    pub fn global(self, globals: &[u32], local: u32) -> Option<u32> {
        self.globals.slice(globals).get(local as usize).copied()
    }

    /// Return Program dynamic-table ids in object-local order.
    pub fn dynamics(self, dynamics: &[u32]) -> &[u32] {
        self.dynamics.slice(dynamics)
    }

    /// Return one Program dynamic-table id.
    pub fn dynamic(self, dynamics: &[u32], local: u32) -> Option<u32> {
        self.dynamics.slice(dynamics).get(local as usize).copied()
    }
}

/// Program identities before Program section packing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleBuilder {
    /// Native symbol imported by this module object.
    symbol: StringId,
    /// Program function identities in native object-local order.
    functions: Vec<abi::Function>,
    /// Optional Program type ids in object-local order.
    types: Vec<Option<u32>>,
    /// Optional Program layout ids in object-local type order.
    layouts: Vec<Option<u32>>,
    /// Program global ids in object-local order.
    globals: Vec<u32>,
    /// Program dynamic-table ids in object-local order.
    dynamics: Vec<u32>,
    /// First Program allocation-site id.
    allocation: u32,
    /// First native frame-map id.
    frame_map: u32,
}

impl ModuleBuilder {
    /// Create one module identity builder.
    pub fn new(symbol: StringId, allocation: u32, frame_map: u32) -> Self {
        Self {
            symbol,
            functions: Vec::new(),
            types: Vec::new(),
            layouts: Vec::new(),
            globals: Vec::new(),
            dynamics: Vec::new(),
            allocation,
            frame_map,
        }
    }

    /// Set Program function identities in native object-local order.
    pub fn functions(mut self, functions: impl IntoIterator<Item = abi::Function>) -> Self {
        self.functions = functions.into_iter().collect();

        self
    }

    /// Set optional Program type ids in object-local order.
    pub fn types(mut self, types: impl IntoIterator<Item = Option<u32>>) -> Self {
        self.types = types.into_iter().collect();

        self
    }

    /// Set optional Program layout ids in object-local type order.
    pub fn layouts(mut self, layouts: impl IntoIterator<Item = Option<u32>>) -> Self {
        self.layouts = layouts.into_iter().collect();

        self
    }

    /// Set Program global ids in object-local order.
    pub fn globals(mut self, globals: impl IntoIterator<Item = u32>) -> Self {
        self.globals = globals.into_iter().collect();

        self
    }

    /// Set Program dynamic-table ids in object-local order.
    pub fn dynamics(mut self, dynamics: impl IntoIterator<Item = u32>) -> Self {
        self.dynamics = dynamics.into_iter().collect();

        self
    }

    /// Build one module into shared identity columns.
    pub(super) fn build(
        self,
        functions: &mut EntryStore<abi::Function>,
        types: &mut EntryStore<Optional<u32>>,
        layouts: &mut EntryStore<Optional<u32>>,
        globals: &mut EntryStore<u32>,
        dynamics: &mut EntryStore<u32>,
    ) -> Module {
        let functions = functions.append(self.functions);
        let types = types.append(self.types.into_iter().map(Optional::from));
        let layouts = layouts.append(self.layouts.into_iter().map(Optional::from));
        let globals = globals.append(self.globals);
        let dynamics = dynamics.append(self.dynamics);

        Module {
            symbol: self.symbol,
            functions,
            types,
            layouts,
            globals,
            dynamics,
            allocation: self.allocation,
            frame_map: self.frame_map,
        }
    }
}
