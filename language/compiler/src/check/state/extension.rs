use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;

use crate::check::TypeOperand;

/// Checked extension declarations for one component.
#[derive(Debug, Default)]
pub(in crate::check) struct ExtensionTable {
    /// Extension definitions keyed by declaring symbol.
    definitions: IndexMap<dir::GlobalSymbolId, ExtensionDefinition>,
}

impl ExtensionTable {
    /// Create an empty extension table.
    pub(in crate::check) fn new() -> Self {
        Self {
            definitions: IndexMap::new(),
        }
    }

    /// Insert one extension definition.
    pub(in crate::check) fn insert_definition(
        &mut self,
        symbol: dir::GlobalSymbolId,
        definition: ExtensionDefinition,
    ) {
        if self.definitions.contains_key(&symbol) {
            panic!("check extension symbol {symbol:?} already has a definition");
        }

        self.definitions.insert(symbol, definition);
    }

    /// Return one extension definition.
    pub(in crate::check) fn definition(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<&ExtensionDefinition> {
        self.definitions.get(&symbol)
    }

    /// Iterate definitions declared by one module.
    pub(in crate::check) fn definitions_in(
        &self,
        module: ModuleId,
    ) -> impl Iterator<Item = (dir::GlobalSymbolId, &ExtensionDefinition)> + '_ {
        self.definitions
            .iter()
            .filter_map(move |(symbol, definition)| {
                (symbol.module_id == module).then_some((*symbol, definition))
            })
    }

    /// Return extension symbols declared in one module for one target symbol.
    pub(in crate::check) fn symbols_for_target(
        &self,
        module: ModuleId,
        target: dir::GlobalSymbolId,
    ) -> Vec<dir::GlobalSymbolId> {
        self.definitions
            .iter()
            .filter_map(|(symbol, definition)| {
                (symbol.module_id == module && definition.target_symbol == target)
                    .then_some(*symbol)
            })
            .collect()
    }
}

/// Checked declaration data for one extension symbol.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ExtensionDefinition {
    /// The source declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The extension visibility form.
    pub(in crate::check) form: dir::ExtensionForm,
    /// The nominal type symbol used to index extension lookup.
    pub(in crate::check) target_symbol: dir::GlobalSymbolId,
    /// The checked target type being extended.
    pub(in crate::check) target_type: TypeOperand,
    /// The checked where clauses that gate this extension.
    pub(in crate::check) where_clauses: Vec<ExtensionWhereClause>,
}

/// A checked where clause attached to one extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct ExtensionWhereClause {
    /// The source where clause node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The constrained type.
    pub(in crate::check) left: TypeOperand,
    /// The required constraint type.
    pub(in crate::check) right: TypeOperand,
}
