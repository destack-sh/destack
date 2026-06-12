use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    CheckState, Condition, Constraint, ConstraintCause, FlowState, Origin, Relation, Task, Widening,
};
use crate::{CompilerError, CompilerResult};

/// State used only while walking one module.
pub(in crate::check) struct WalkState<'check, 'state> {
    /// The component check state being populated.
    pub(in crate::check) check: &'check mut CheckState<'state>,
    /// The visible DIR tree being walked.
    pub(in crate::check) tree: dir::View<'check>,
    /// The module being walked.
    pub(in crate::check) module: ModuleId,
    /// Flow state for the current module walk.
    flow: FlowState,
}

impl<'check, 'state> WalkState<'check, 'state> {
    /// Create walk state for one module.
    pub(in crate::check) fn new(
        module: ModuleId,
        tree: dir::View<'check>,
        check: &'check mut CheckState<'state>,
    ) -> Self {
        Self {
            check,
            tree,
            module,
            flow: FlowState::default(),
        }
    }

    /// Return flow state for the active module.
    pub(in crate::check) fn flow(&self) -> &FlowState {
        &self.flow
    }

    /// Return mutable flow state for the active module.
    pub(in crate::check) fn flow_mut(&mut self) -> &mut FlowState {
        &mut self.flow
    }

    /// Return one node's working type, opening a variable when missing.
    pub(in crate::check) fn node_type<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.node_type_any(id.into_global_any(self.module))
    }

    /// Return one node's working type by any id, opening a variable when missing.
    pub(in crate::check) fn node_type_any(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if let Some(ty) = self.check.inputs.node_type(node) {
            return Ok(ty);
        }

        // open a fresh variable carrying the node origin; node types
        // state what an expression is, only bindings widen
        let origin = Origin::Node(node);
        let variable = self
            .check
            .allocate_variable(self.module, origin, Widening::Preserve);
        let ty = self.check.push_variable_type(variable, node.local_id)?;
        self.check.inputs.set_node_type(node, ty)?;

        Ok(ty)
    }

    /// Open one fresh inference type at a source node.
    pub(in crate::check) fn open_type(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let origin = Origin::Node(source.into_global(self.module));
        let variable = self
            .check
            .allocate_variable(self.module, origin, Widening::Preserve);

        self.check.push_variable_type(variable, source)
    }

    /// Return the effective type for one value expression, preferring
    /// the active flow narrowing over the node's own type.
    pub(in crate::check) fn expression_type(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if let Some(narrowed) = self.flow_path_narrowing(id) {
            return Ok(narrowed);
        }

        self.node_type(id)
    }

    /// Declare one node's own type, equating against an existing type.
    pub(in crate::check) fn declare_node_type<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = id.into_global_any(self.module);
        let Some(target) = self.check.inputs.node_type(node) else {
            self.check.inputs.set_node_type(node, ty)?;

            return Ok(ty);
        };

        self.relate_type(Origin::Node(node), Relation::Equal, target, ty);

        Ok(target)
    }

    /// Expect one node's type to flow into a contextual type.
    pub(in crate::check) fn expect_assignable<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
        expected: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = id.into_global_any(self.module);
        let target = self.node_type_any(node)?;
        self.relate_type(Origin::Node(node), Relation::Assignable, target, expected);

        Ok(target)
    }

    /// Collect one relation constraint under the active static guard.
    pub(in crate::check) fn relate_type(
        &mut self,
        origin: Origin,
        relation: Relation,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) {
        self.push_relation(origin, ConstraintCause::General, relation, left, right);
    }

    /// Collect one relation constraint with its failure context.
    pub(in crate::check) fn push_relation(
        &mut self,
        origin: Origin,
        cause: ConstraintCause,
        relation: Relation,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) {
        let condition = self.flow.active_static_guard();
        self.check.push_constraint(Constraint {
            relation,
            left,
            right,
            origin,
            condition,
            cause,
        });
    }

    /// Queue one node selection under the active static guard.
    pub(in crate::check) fn queue_select(&mut self, node: dir::GlobalNodeIdAny) {
        // record the guard context the selection must run under
        if let Condition::When(predicates) = self.flow.active_static_guard() {
            self.check.inputs.set_node_condition(node, predicates);
        }

        self.check.queue_task(Task::Select(node));
    }

    /// Return one symbol's working type, opening a variable when missing.
    pub(in crate::check) fn symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if let Some(ty) = self.check.inputs.symbol_type(symbol) {
            return Ok(ty);
        }

        // external symbols chase aliases, then read committed types
        if !self.check.is_component_module(symbol.module_id) {
            let symbol = self.check.resolve_external_alias(symbol)?;
            self.check.import_external_module(symbol.module_id)?;
            let committed = self
                .check
                .external_modules
                .get(&symbol.module_id)
                .and_then(|external| external.types.get_symbol_type_id(symbol));
            let Some(ty) = committed else {
                return Err(CompilerError::Internal {
                    message: format!("external symbol {symbol:?} has no imported type"),
                });
            };
            self.check.inputs.set_symbol_type(symbol, ty)?;

            return Ok(ty);
        }
        if self
            .check
            .module(symbol.module_id)
            .is_import_alias(symbol.local_id)
        {
            return Err(CompilerError::Internal {
                message: format!("import alias {symbol:?} reached type creation"),
            });
        }

        // open source declaration types on first use
        let origin = Origin::Symbol(symbol);
        let source = self
            .check
            .module(symbol.module_id)
            .symbol_declaration_node(symbol.local_id)?;
        let variable = self
            .check
            .allocate_variable(symbol.module_id, origin, Widening::Preserve);
        let ty = self.check.push_variable_type(variable, source)?;
        self.check.inputs.set_symbol_type(symbol, ty)?;

        Ok(ty)
    }

    /// Return one binding's working type, opening a variable with the
    /// requested widening policy when missing.
    pub(in crate::check) fn binding_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        widening: Widening,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if let Some(ty) = self.check.inputs.symbol_type(symbol) {
            return Ok(ty);
        }

        let origin = Origin::Symbol(symbol);
        let source = self
            .check
            .module(symbol.module_id)
            .symbol_declaration_node(symbol.local_id)?;
        let variable = self
            .check
            .allocate_variable(symbol.module_id, origin, widening);
        let ty = self.check.push_variable_type(variable, source)?;
        self.check.inputs.set_symbol_type(symbol, ty)?;

        Ok(ty)
    }

    /// Declare one symbol's type, equating against an existing type.
    pub(in crate::check) fn declare_symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if let Some(existing) = self.check.inputs.symbol_type(symbol) {
            self.relate_type(Origin::Symbol(symbol), Relation::Equal, existing, ty);

            return Ok(existing);
        }

        self.check.inputs.set_symbol_type(symbol, ty)?;

        Ok(ty)
    }

    /// Declare one symbol's static value singleton type.
    pub(in crate::check) fn declare_symbol_value(
        &mut self,
        symbol: dir::GlobalSymbolId,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        self.check.inputs.set_symbol_value(symbol, value)
    }

    /// Push one working type at a source node.
    pub(in crate::check) fn push_type(
        &mut self,
        ty: dir::Type,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check.push_working_type(self.module, ty, source)
    }

    /// Return a reference type for one well-known library declaration.
    pub(in crate::check) fn language_type_reference(
        &mut self,
        source: dir::LocalNodeIdAny,
        item: dir::LanguageItem,
        arguments: Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let symbol = self.check.language_symbol(item);
        let reference = dir::Type::Reference(dir::GenericInstance { symbol, arguments });

        self.push_type(reference, source)
    }

    /// Return a value type with `undefined` included.
    pub(in crate::check) fn optional_value_type(
        &mut self,
        ty: dir::GlobalTypeId,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let undefined = self.push_type(dir::Type::Undefined, source)?;
        let union = dir::Type::Union(dir::UnionType {
            elements: vec![ty, undefined],
        });

        self.push_type(union, source)
    }

    /// Return the predicates of the active static guard.
    pub(in crate::check) fn guard_predicates(&self) -> SmallVec<[dir::GlobalTypeId; 2]> {
        match self.flow.active_static_guard() {
            Condition::Always => SmallVec::new(),
            Condition::When(predicates) => predicates,
        }
    }
}
