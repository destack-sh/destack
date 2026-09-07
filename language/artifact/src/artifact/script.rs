use destack_core::StringId;
use destack_dir::GlobalSymbolId;
use destack_js::{LocalNodeId, Module, Node, ScopeId, SymbolId, SymbolNamespace};
use destack_serde::Reflect;
use destack_source::{ModuleId, ProvenanceId, ProvenanceJournal};
use serde::{Deserialize, Serialize};

/// The compiler identity represented by one JavaScript symbol.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Reflect)]
enum SymbolOrigin {
    /// A checked source declaration.
    Source(GlobalSymbolId),
    /// A generated resource module default.
    ModuleDefault(ModuleId),
    /// An unresolved runtime global.
    External,
}

/// One structured JavaScript linker input.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct Script {
    /// The emitted JavaScript module.
    pub module: Module,
    /// Compiler identities keyed by JavaScript symbol id.
    symbol_origins: Vec<Option<SymbolOrigin>>,
    /// Resolved dependency modules keyed by JavaScript node id.
    dependency_modules: Vec<Option<ModuleId>>,
}

impl Script {
    /// Create one JavaScript linker input.
    pub fn new(
        module: Module,
        source_symbols: Vec<Option<GlobalSymbolId>>,
        dependency_modules: Vec<Option<ModuleId>>,
    ) -> Self {
        let symbol_origins = source_symbols
            .into_iter()
            .map(|source| source.map(SymbolOrigin::Source))
            .collect();

        Self {
            module,
            symbol_origins,
            dependency_modules,
        }
    }

    /// Return the emitted JavaScript module.
    pub const fn module(&self) -> &Module {
        &self.module
    }

    /// Rewrite this script under one recorded transform.
    pub fn rewrite<T>(
        &mut self,
        transform: &str,
        rewrite: impl FnOnce(&mut Self, &mut ProvenanceJournal<'_>) -> T,
    ) -> T {
        let mut provenance = self.module.provenance.extend();
        let mut journal = provenance.record(transform);
        let result = rewrite(self, &mut journal);
        self.module.provenance = provenance.finish();

        result
    }

    /// Return the source symbol represented by one JavaScript symbol.
    pub fn source_symbol(&self, symbol: SymbolId) -> Option<GlobalSymbolId> {
        match self.symbol_origins.get(symbol.0 as usize)? {
            Some(SymbolOrigin::Source(source)) => Some(*source),
            Some(SymbolOrigin::ModuleDefault(_) | SymbolOrigin::External) | None => None,
        }
    }

    /// Return the resolved module referenced by one dependency node.
    pub fn dependency_module<T: Node>(&self, node: LocalNodeId<T>) -> Option<ModuleId> {
        self.dependency_modules
            .get(node.id as usize)
            .copied()
            .flatten()
    }

    /// Record the source symbol represented by one JavaScript symbol.
    pub fn set_source_symbol(&mut self, symbol: SymbolId, source: GlobalSymbolId) {
        self.set_symbol_origin(symbol, SymbolOrigin::Source(source));
    }

    /// Return the generated module default represented by one JavaScript symbol.
    pub fn default_module(&self, symbol: SymbolId) -> Option<ModuleId> {
        match self.symbol_origins.get(symbol.0 as usize)? {
            Some(SymbolOrigin::ModuleDefault(module)) => Some(*module),
            Some(SymbolOrigin::Source(_) | SymbolOrigin::External) | None => None,
        }
    }

    /// Record the generated module default represented by one JavaScript symbol.
    pub fn set_default_module(&mut self, symbol: SymbolId, module: ModuleId) {
        self.set_symbol_origin(symbol, SymbolOrigin::ModuleDefault(module));
    }

    /// Return whether one JavaScript symbol names an unresolved runtime global.
    pub fn is_external_symbol(&self, symbol: SymbolId) -> bool {
        matches!(
            self.symbol_origins.get(symbol.0 as usize),
            Some(Some(SymbolOrigin::External))
        )
    }

    /// Record one unresolved runtime global.
    pub fn set_external_symbol(&mut self, symbol: SymbolId) {
        self.set_symbol_origin(symbol, SymbolOrigin::External);
    }

    /// Record one JavaScript symbol origin.
    fn set_symbol_origin(&mut self, symbol: SymbolId, origin: SymbolOrigin) {
        let required_len = symbol.0 as usize + 1;
        if self.symbol_origins.len() < required_len {
            self.symbol_origins.resize(required_len, None);
        }

        self.symbol_origins[symbol.0 as usize] = Some(origin);
    }

    /// Allocate one source-backed JavaScript symbol.
    pub fn insert_source_symbol(
        &mut self,
        name: StringId,
        namespace: SymbolNamespace,
        scope: ScopeId,
        provenance: ProvenanceId,
        source: GlobalSymbolId,
    ) -> SymbolId {
        let symbol = self
            .module
            .insert_symbol(name, namespace, scope, provenance);
        self.set_source_symbol(symbol, source);

        symbol
    }
}
