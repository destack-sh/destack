use destack_core::{Optional, StringId};
use destack_native as native;
use destack_native::abi;
use destack_program::Program;

use super::Error;

/// Process-local mappings from native object-local indices to Program identities.
#[derive(Debug)]
pub struct ModuleTable {
    /// Descriptor symbols in archive object order.
    symbols: Box<[StringId]>,
    /// Program function identities retained by native modules.
    functions: Box<[abi::Function]>,
    /// Optional Program type identities retained by modules.
    types: Box<[Optional<u32>]>,
    /// Optional Program layout identities retained by modules.
    layouts: Box<[Optional<u32>]>,
    /// Program global identities retained by modules.
    globals: Box<[u32]>,
    /// Program dynamic-table identities retained by modules.
    dynamics: Box<[u32]>,
    /// Process-local Program identity mappings referenced by generated code.
    modules: Box<[abi::Module]>,
}

// SAFETY: every raw module pointer targets an immutable boxed column owned by this table.
unsafe impl Send for ModuleTable {}

// SAFETY: the owned columns and their raw module projections remain immutable after construction.
unsafe impl Sync for ModuleTable {}

impl ModuleTable {
    /// Build process-local Program identity mappings from durable native code.
    pub fn new(program: &Program, code: &native::Code) -> Result<Self, Error> {
        let sections = program.sections();
        let functions = code.functions(sections).to_vec().into_boxed_slice();
        let types = code.types(sections).to_vec().into_boxed_slice();
        let layouts = code.layouts(sections).to_vec().into_boxed_slice();
        let globals = code.globals(sections).to_vec().into_boxed_slice();
        let dynamics = code.dynamics(sections).to_vec().into_boxed_slice();
        let modules = code.modules(sections);
        let symbols = modules
            .iter()
            .map(|module| module.symbol)
            .collect::<Vec<_>>()
            .into_boxed_slice();

        // point every module into the stable boxed identity columns
        let loaded = modules
            .iter()
            .map(|module| abi::Module {
                functions: module.functions(&functions).as_ptr(),
                types: module.types(&types).as_ptr(),
                layouts: module.layouts(&layouts).as_ptr(),
                globals: module.globals(&globals).as_ptr(),
                dynamics: module.dynamics(&dynamics).as_ptr(),
                allocation: module.allocation,
                frame_map: module.frame_map,
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        // require every module symbol to remain available in Program strings
        for symbol in &symbols {
            if program.string(*symbol).is_none() {
                return Err(Error::ProgramStringMissing { string: *symbol });
            }
        }

        Ok(Self {
            symbols,
            functions,
            types,
            layouts,
            globals,
            dynamics,
            modules: loaded,
        })
    }

    /// Create one empty module table.
    pub fn empty() -> Self {
        Self {
            symbols: Box::new([]),
            functions: Box::new([]),
            types: Box::new([]),
            layouts: Box::new([]),
            globals: Box::new([]),
            dynamics: Box::new([]),
            modules: Box::new([]),
        }
    }

    /// Return one module by archive object index.
    pub fn get(&self, index: u32) -> Option<&abi::Module> {
        self.modules.get(index as usize)
    }

    /// Return retained Program function identities.
    pub fn functions(&self) -> &[abi::Function] {
        &self.functions
    }

    /// Return retained optional Program type identities.
    pub fn types(&self) -> &[Optional<u32>] {
        &self.types
    }

    /// Return retained optional Program layout identities.
    pub fn layouts(&self) -> &[Optional<u32>] {
        &self.layouts
    }

    /// Return retained Program global identities.
    pub fn globals(&self) -> &[u32] {
        &self.globals
    }

    /// Return retained Program dynamic-table identities.
    pub fn dynamics(&self) -> &[u32] {
        &self.dynamics
    }

    /// Return the module address for one imported symbol.
    pub fn address(&self, program: &Program, symbol: &str) -> Option<usize> {
        self.symbols
            .iter()
            .position(|candidate| program.string(*candidate) == Some(symbol))
            .and_then(|index| self.modules.get(index))
            .map(|module| std::ptr::from_ref(module).addr())
    }
}
