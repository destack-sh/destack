use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;

use crate::check::{CheckState, StaticOperand, TypeOperand};
use crate::{CompilerError, CompilerResult};

/// Input identity to check operand index.
#[derive(Debug)]
pub(in crate::check) struct InputTable {
    /// Type operands keyed by source node.
    node_types: IndexMap<dir::GlobalNodeIdAny, TypeOperand>,
    /// Type operands keyed by source symbol.
    symbol_types: IndexMap<dir::GlobalSymbolId, TypeOperand>,

    /// Static operands keyed by source node.
    node_statics: IndexMap<dir::GlobalNodeIdAny, StaticOperand>,
    /// Static operands keyed by source symbol.
    symbol_statics: IndexMap<dir::GlobalSymbolId, StaticOperand>,

    /// Type operands cached by committed type id.
    types: IndexMap<dir::GlobalTypeId, TypeOperand>,
    /// Static operands cached by committed static id.
    statics: IndexMap<dir::GlobalStaticId, StaticOperand>,
}

impl InputTable {
    /// Create an empty input table.
    pub(in crate::check) fn new() -> Self {
        Self {
            node_types: IndexMap::new(),
            symbol_types: IndexMap::new(),
            node_statics: IndexMap::new(),
            symbol_statics: IndexMap::new(),
            types: IndexMap::new(),
            statics: IndexMap::new(),
        }
    }

    /// Iterate source node type operands declared in one module.
    pub(in crate::check) fn node_types_in(
        &self,
        module: ModuleId,
    ) -> impl Iterator<Item = (dir::GlobalNodeIdAny, TypeOperand)> + '_ {
        self.node_types.iter().filter_map(move |(node, operand)| {
            (node.module_id == module).then_some((*node, *operand))
        })
    }

    /// Iterate source symbol type operands declared in one module.
    pub(in crate::check) fn symbol_types_in(
        &self,
        module: ModuleId,
    ) -> impl Iterator<Item = (dir::GlobalSymbolId, TypeOperand)> + '_ {
        self.symbol_types
            .iter()
            .filter_map(move |(symbol, operand)| {
                (symbol.module_id == module).then_some((*symbol, *operand))
            })
    }

    /// Iterate source symbol static operands declared in one module.
    pub(in crate::check) fn symbol_statics_in(
        &self,
        module: ModuleId,
    ) -> impl Iterator<Item = (dir::GlobalSymbolId, StaticOperand)> + '_ {
        self.symbol_statics
            .iter()
            .filter_map(move |(symbol, operand)| {
                (symbol.module_id == module).then_some((*symbol, *operand))
            })
    }

    /// Return one source node type operand.
    pub(in crate::check) fn node_type(&self, node: dir::GlobalNodeIdAny) -> Option<TypeOperand> {
        self.node_types.get(&node).copied()
    }

    /// Return one source symbol type operand.
    pub(in crate::check) fn symbol_type(&self, symbol: dir::GlobalSymbolId) -> Option<TypeOperand> {
        self.symbol_types.get(&symbol).copied()
    }

    /// Return one source node static operand.
    pub(in crate::check) fn node_static(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> Option<StaticOperand> {
        self.node_statics.get(&node).copied()
    }

    /// Return one source symbol static operand.
    pub(in crate::check) fn symbol_static(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<StaticOperand> {
        self.symbol_statics.get(&symbol).copied()
    }

    /// Return one operand keyed by committed type id.
    pub(in crate::check) fn r#type(&self, id: dir::GlobalTypeId) -> Option<TypeOperand> {
        self.types.get(&id).copied()
    }

    /// Upsert one operand keyed by committed type id.
    pub(in crate::check) fn upsert_type(
        &mut self,
        id: dir::GlobalTypeId,
        operand: TypeOperand,
    ) -> CompilerResult<()> {
        if let Some(previous) = self.types.get(&id) {
            if *previous != operand {
                return Err(CompilerError::Internal {
                    message: format!("check type id {id:?} already has a different operand"),
                });
            }

            return Ok(());
        }

        self.types.insert(id, operand);

        Ok(())
    }

    /// Return one operand keyed by committed static id.
    pub(in crate::check) fn r#static(&self, id: dir::GlobalStaticId) -> Option<StaticOperand> {
        self.statics.get(&id).copied()
    }

    /// Upsert one operand keyed by committed static id.
    pub(in crate::check) fn upsert_static(
        &mut self,
        id: dir::GlobalStaticId,
        operand: StaticOperand,
    ) -> CompilerResult<()> {
        if let Some(previous) = self.statics.get(&id) {
            if *previous != operand {
                return Err(CompilerError::Internal {
                    message: format!("check static id {id:?} already has a different operand"),
                });
            }

            return Ok(());
        }

        self.statics.insert(id, operand);

        Ok(())
    }

    /// Insert one source node type operand.
    pub(in crate::check) fn insert_node_type(
        &mut self,
        node: dir::GlobalNodeIdAny,
        operand: TypeOperand,
    ) -> CompilerResult<TypeOperand> {
        if self.node_types.contains_key(&node) {
            return Err(CompilerError::Internal {
                message: format!("check node {node:?} already has a type operand"),
            });
        }

        self.node_types.insert(node, operand);

        Ok(operand)
    }

    /// Insert one source node static operand.
    pub(in crate::check) fn insert_node_static(
        &mut self,
        node: dir::GlobalNodeIdAny,
        operand: StaticOperand,
    ) -> CompilerResult<StaticOperand> {
        if self.node_statics.contains_key(&node) {
            return Err(CompilerError::Internal {
                message: format!("check node {node:?} already has a static operand"),
            });
        }

        self.node_statics.insert(node, operand);

        Ok(operand)
    }

    /// Insert one source symbol type operand.
    pub(in crate::check) fn insert_symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        operand: TypeOperand,
    ) -> CompilerResult<TypeOperand> {
        if self.symbol_types.contains_key(&symbol) {
            return Err(CompilerError::Internal {
                message: format!("check symbol {symbol:?} already has a type operand"),
            });
        }

        self.symbol_types.insert(symbol, operand);

        Ok(operand)
    }

    /// Insert one source symbol static operand.
    pub(in crate::check) fn insert_symbol_static(
        &mut self,
        symbol: dir::GlobalSymbolId,
        operand: StaticOperand,
    ) -> CompilerResult<StaticOperand> {
        if self.symbol_statics.contains_key(&symbol) {
            return Err(CompilerError::Internal {
                message: format!("check symbol {symbol:?} already has a static operand"),
            });
        }

        self.symbol_statics.insert(symbol, operand);

        Ok(operand)
    }
}

impl CheckState<'_> {
    /// Return one required node type operand.
    pub(in crate::check) fn node_type_operand(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<TypeOperand> {
        if let Some(operand) = self.inputs.node_type(node) {
            return Ok(operand);
        }

        // include the source location for missing walked node types
        let module = self.module(node.module_id);
        let span = module.view().get_span_by_id(node.local_id.id);

        Err(CompilerError::Internal {
            message: format!(
                "check node {node:?} in {:?} at {span:?} has no type operand",
                module.module.uri
            ),
        })
    }

    /// Return one required node static operand.
    pub(in crate::check) fn node_static_operand(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<StaticOperand> {
        if let Some(operand) = self.inputs.node_static(node) {
            return Ok(operand);
        }

        // include the source location for missing walked node statics
        let module = self.module(node.module_id);
        let span = module.view().get_span_by_id(node.local_id.id);

        Err(CompilerError::Internal {
            message: format!(
                "check node {node:?} in {:?} at {span:?} has no static operand",
                module.module.uri
            ),
        })
    }

    /// Return one required source symbol type operand.
    pub(in crate::check) fn symbol_type_operand(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<TypeOperand> {
        if let Some(operand) = self.inputs.symbol_type(symbol) {
            return Ok(operand);
        }

        let symbol = self.dump_in_module(module, &symbol);

        Err(CompilerError::Internal {
            message: format!("check symbol {symbol} has no type operand"),
        })
    }

    /// Return one required source symbol static operand.
    pub(in crate::check) fn symbol_static_operand(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<StaticOperand> {
        if let Some(operand) = self.inputs.symbol_static(symbol) {
            return Ok(operand);
        }

        let symbol = self.dump_in_module(module, &symbol);

        Err(CompilerError::Internal {
            message: format!("check symbol {symbol} has no static operand"),
        })
    }
}
