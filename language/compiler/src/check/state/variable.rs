use destack_source::ModuleId;

use destack_dir as dir;

use super::{CheckModuleState, GenericVariable, StaticTerm, TypeTerm};

/// Component-valid id for one check variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in crate::check) struct VariableId {
    /// The module that owns the variable.
    pub(in crate::check) module: ModuleId,
    /// The variable index inside the owning module.
    pub(in crate::check) index: u32,
}

impl VariableId {
    /// Create one variable id.
    pub(in crate::check) fn new(module: ModuleId, index: u32) -> Self {
        Self { module, index }
    }
}

/// One check variable.
///
/// ```ts
/// let value = input.name;
/// ```
///
/// The checker creates variables for the binding symbol `value`, the expression node `input.name`,
/// and any static or type operations needed to solve the expression.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct Variable {
    /// The variable id.
    pub(in crate::check) id: VariableId,
    /// The variable kind.
    pub(in crate::check) kind: VariableKind,
    /// The source that produced the variable.
    pub(in crate::check) origin: VariableOrigin,
    /// The solved value, when known.
    pub(in crate::check) value: Option<VariableValue>,
}

impl Variable {
    /// Create one unsolved variable.
    pub(in crate::check) fn new(
        id: VariableId,
        kind: VariableKind,
        origin: VariableOrigin,
    ) -> Self {
        Self {
            id,
            kind,
            origin,
            value: None,
        }
    }
}

/// The value space of one check variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum VariableKind {
    /// Type variable.
    Type,
    /// Static value variable.
    Static,
}

/// Solved value for one check variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum VariableValue {
    /// Solved type term.
    Type(TypeTerm),
    /// Solved static term.
    Static(StaticTerm),
}

/// Source that produced one variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum VariableOrigin {
    /// Variable recorded as one generic slot.
    ///
    /// ```ts
    /// function first<T>(value: &T): &T {
    ///     return value;
    /// }
    /// ```
    ///
    /// Explicit `T` and induced lifetime slots are represented as generic variables.
    Generic(GenericVariable),

    /// Variable attached to a source node.
    ///
    /// ```ts
    /// value.name
    /// ```
    ///
    /// The member expression node has its own type variable.
    Node(dir::GlobalNodeIdAny),

    /// Variable attached to a source symbol.
    ///
    /// ```ts
    /// let value = 1;
    /// ```
    ///
    /// The binding symbol `value` has its own type variable.
    Symbol(dir::GlobalSymbolId),

    /// Variable introduced by solver rules.
    ///
    /// ```ts
    /// WithLifetime<T, L>
    /// ```
    ///
    /// The normalized memory form can create intermediate solver variables.
    Synthetic,
}

impl CheckModuleState {
    /// Push one check variable.
    pub(in crate::check) fn push_variable(
        &mut self,
        kind: VariableKind,
        origin: VariableOrigin,
    ) -> VariableId {
        let id = VariableId::new(self.module, self.variables.len() as u32);
        let variable = Variable::new(id, kind, origin);
        self.variables.push(variable);

        id
    }

    /// Return one variable.
    pub(in crate::check) fn variable(&self, id: VariableId) -> &Variable {
        &self.variables[id.index as usize]
    }

    /// Return one variable mutably.
    pub(in crate::check) fn variable_mut(&mut self, id: VariableId) -> &mut Variable {
        &mut self.variables[id.index as usize]
    }

    /// Return the solved value for one variable.
    pub(in crate::check) fn variable_value(&self, id: VariableId) -> Option<&VariableValue> {
        self.variable(id).value.as_ref()
    }

    /// Bind the solved value for one variable and return whether it changed.
    pub(in crate::check) fn bind_variable(&mut self, id: VariableId, value: VariableValue) -> bool {
        let variable = self.variable_mut(id);
        let Some(existing) = &variable.value else {
            variable.value = Some(value);

            return true;
        };

        // keep the first solution, relation constraints report conflicts
        if existing != &value {
            return false;
        }

        false
    }

    /// Return one variable's source node for diagnostics.
    pub(in crate::check) fn variable_source_node(&self, id: VariableId) -> dir::LocalNodeIdAny {
        let module_node = self.bound.module_node;

        match &self.variable(id).origin {
            VariableOrigin::Node(node) if node.module_id == self.module => node.local_id,
            VariableOrigin::Symbol(symbol) if symbol.module_id == self.module => {
                match self.symbol_source_node(*symbol) {
                    Some(node) => node,
                    None => module_node,
                }
            }
            VariableOrigin::Generic(generic) if generic.slot().owner.module_id == self.module => {
                match self.symbol_source_node(generic.slot().owner) {
                    Some(node) => node,
                    None => module_node,
                }
            }
            VariableOrigin::Synthetic
            | VariableOrigin::Symbol(_)
            | VariableOrigin::Node(_)
            | VariableOrigin::Generic(_) => module_node,
        }
    }

    /// Return the solved type for one variable.
    pub(in crate::check) fn variable_type_value(&self, id: VariableId) -> Option<&TypeTerm> {
        match self.variable_value(id) {
            Some(VariableValue::Type(term)) => Some(term),
            _ => None,
        }
    }

    /// Return or create a type variable for one node.
    pub(in crate::check) fn node_type_variable(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> VariableId {
        if let Some(variable) = self.node_type_variables.get(&node).copied() {
            return variable;
        }

        let variable = self.push_variable(VariableKind::Type, VariableOrigin::Node(node));
        self.node_type_variables.insert(node, variable);

        variable
    }

    /// Return or create a type variable for one symbol.
    pub(in crate::check) fn symbol_type_variable(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        if let Some(variable) = self.symbol_type_variables.get(&symbol).copied() {
            return variable;
        }

        let variable = self.push_variable(VariableKind::Type, VariableOrigin::Symbol(symbol));

        // seed declarations that already have checked input types
        if let Some(type_id) = self.visible_symbol_type_id(symbol) {
            let ty = self.get_type(type_id);
            let _ = self.bind_variable(variable, VariableValue::Type(TypeTerm::Type(ty)));
        }

        self.symbol_type_variables.insert(symbol, variable);

        variable
    }

    /// Return or create a static variable for one node.
    pub(in crate::check) fn node_static_variable(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> VariableId {
        if let Some(variable) = self.node_static_variables.get(&node).copied() {
            return variable;
        }

        let variable = self.push_variable(VariableKind::Static, VariableOrigin::Node(node));
        self.node_static_variables.insert(node, variable);

        variable
    }

    /// Return or create a static variable for one symbol.
    pub(in crate::check) fn symbol_static_variable(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        if let Some(variable) = self.symbol_static_variables.get(&symbol).copied() {
            return variable;
        }

        let variable = self.push_variable(VariableKind::Static, VariableOrigin::Symbol(symbol));

        // seed declarations that already have checked input statics
        if let Some(static_id) = self.visible_symbol_static_id(symbol) {
            let term = self.get_static(static_id);
            let _ = self.bind_variable(variable, VariableValue::Static(StaticTerm::Value(term)));
        }

        self.symbol_static_variables.insert(symbol, variable);

        variable
    }

    /// Return one synthetic static value variable.
    pub(in crate::check) fn static_value_variable(&mut self, value: dir::StaticTerm) -> VariableId {
        let variable = self.push_variable(VariableKind::Static, VariableOrigin::Synthetic);

        self.define_static_term(variable, StaticTerm::Value(value));

        variable
    }

    /// Return the solved static value for one variable.
    pub(in crate::check) fn variable_static_value(&self, id: VariableId) -> Option<&StaticTerm> {
        match self.variable_value(id) {
            Some(VariableValue::Static(term)) => Some(term),
            _ => None,
        }
    }
}
