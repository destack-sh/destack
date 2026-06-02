use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;

use crate::check::{
    CheckState, Condition, Origin, StaticOperand, StaticTerm, TypeOperand, TypeRelation, TypeTerm,
    VariableId, VariableKind,
};

/// Check operands keyed by source or committed DIR identity.
#[derive(Debug)]
pub(in crate::check) struct OperandTable {
    /// Type operands keyed by checked source node.
    pub(in crate::check) node_types: IndexMap<dir::GlobalNodeIdAny, TypeOperand>,
    /// Type operands keyed by checked source symbol.
    pub(in crate::check) symbol_types: IndexMap<dir::GlobalSymbolId, TypeOperand>,

    /// Static operands keyed by checked source node.
    pub(in crate::check) node_statics: IndexMap<dir::GlobalNodeIdAny, StaticOperand>,
    /// Static operands keyed by checked source symbol.
    pub(in crate::check) symbol_statics: IndexMap<dir::GlobalSymbolId, StaticOperand>,

    /// Type operands keyed by checked DIR type id.
    pub(in crate::check) types: IndexMap<dir::GlobalTypeId, TypeOperand>,
    /// Static operands keyed by checked DIR static id.
    pub(in crate::check) statics: IndexMap<dir::GlobalStaticId, StaticOperand>,
}

impl OperandTable {
    /// Create an empty operand table.
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
}

impl CheckState<'_> {
    /// Reserve one checked type variable for a node.
    pub(in crate::check) fn reserve_node_type(
        &mut self,
        module: ModuleId,
        node: dir::GlobalNodeIdAny,
    ) -> VariableId {
        let variable = self.allocate_variable(module, VariableKind::Type, Origin::Node(node));
        let operand = TypeOperand::Variable(variable);
        self.publish_node_type_entry(node, operand);
        variable
    }

    /// Reserve one checked static variable for a node.
    pub(in crate::check) fn reserve_node_static(
        &mut self,
        module: ModuleId,
        node: dir::GlobalNodeIdAny,
    ) -> VariableId {
        let variable = self.allocate_variable(module, VariableKind::Static, Origin::Node(node));
        let operand = StaticOperand::Variable(variable);
        self.publish_node_static_entry(node, operand);
        variable
    }

    /// Reserve one checked type variable for a symbol.
    pub(in crate::check) fn reserve_symbol_type(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        let variable = self.allocate_variable(module, VariableKind::Type, Origin::Symbol(symbol));
        let operand = TypeOperand::Variable(variable);
        self.publish_symbol_type_entry(symbol, operand);
        variable
    }

    /// Return one checked symbol type variable, reserving it when missing.
    pub(in crate::check) fn reserve_symbol_type_if_missing(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        match self.operands.symbol_types.get(&symbol).copied() {
            Some(TypeOperand::Variable(variable)) => variable,
            Some(TypeOperand::Term(_) | TypeOperand::Type(_)) => {
                panic!("check symbol {symbol:?} already has checked type operand")
            }
            None => self.reserve_symbol_type(module, symbol),
        }
    }

    /// Reserve one checked static variable for a symbol.
    pub(in crate::check) fn reserve_symbol_static(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        let variable = self.allocate_variable(module, VariableKind::Static, Origin::Symbol(symbol));

        let operand = StaticOperand::Variable(variable);

        self.publish_symbol_static_entry(symbol, operand);

        variable
    }

    /// Cache one checked symbol type operand without committing it.
    pub(in crate::check) fn cache_symbol_type_operand(
        &mut self,
        symbol: dir::GlobalSymbolId,
        operand: TypeOperand,
    ) -> TypeOperand {
        if let Some(existing) = self.operands.symbol_types.get(&symbol).copied() {
            return existing;
        }

        self.operands.symbol_types.insert(symbol, operand);

        operand
    }

    /// Cache one checked symbol static operand without committing it.
    pub(in crate::check) fn cache_symbol_static_operand(
        &mut self,
        symbol: dir::GlobalSymbolId,
        operand: StaticOperand,
    ) -> StaticOperand {
        if let Some(existing) = self.operands.symbol_statics.get(&symbol).copied() {
            return existing;
        }

        self.operands.symbol_statics.insert(symbol, operand);
        operand
    }

    /// Publish one checked static operand for a symbol.
    pub(in crate::check) fn publish_symbol_static_operand(
        &mut self,
        symbol: dir::GlobalSymbolId,
        operand: StaticOperand,
    ) -> StaticOperand {
        self.publish_symbol_static_entry(symbol, operand);
        operand
    }

    /// Publish one checked type term for a node.
    pub(in crate::check) fn publish_node_type<T: dir::Node + Clone>(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<T>,
        term: TypeTerm,
    ) -> TypeOperand {
        let node = id.into_global_any(module);
        let operand = self.create_type_term_operand(module, Origin::Node(node), term);
        self.publish_node_type_entry(node, operand);
        operand
    }

    /// Publish one checked type operand for a node.
    pub(in crate::check) fn publish_node_type_operand<T: dir::Node + Clone>(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<T>,
        operand: TypeOperand,
    ) -> TypeOperand {
        let node = id.into_global_any(module);
        self.publish_node_type_entry(node, operand);
        operand
    }

    /// Publish one checked type term for a symbol.
    pub(in crate::check) fn publish_symbol_type(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        term: TypeTerm,
        condition: Condition,
    ) -> TypeOperand {
        if let Some(operand) = self.operands.symbol_types.get(&symbol).copied() {
            return match operand {
                TypeOperand::Variable(variable) => {
                    self.equate_type(variable, term, condition);

                    operand
                }
                TypeOperand::Term(_) | TypeOperand::Type(_) => {
                    panic!("check symbol {symbol:?} already has checked type operand")
                }
            };
        }

        let operand = self.create_type_term_operand(module, Origin::Symbol(symbol), term);
        self.publish_symbol_type_entry(symbol, operand);
        operand
    }

    /// Publish one checked type operand for a symbol.
    pub(in crate::check) fn publish_symbol_type_operand(
        &mut self,
        symbol: dir::GlobalSymbolId,
        operand: TypeOperand,
        condition: Condition,
    ) -> TypeOperand {
        if let Some(existing) = self.operands.symbol_types.get(&symbol).copied() {
            return match existing {
                TypeOperand::Variable(variable) => {
                    let origin = Origin::Symbol(symbol);

                    self.relate_type(origin, TypeRelation::Equal, variable, operand, condition);

                    existing
                }
                TypeOperand::Term(_) | TypeOperand::Type(_) => {
                    panic!("check symbol {symbol:?} already has checked type operand")
                }
            };
        }

        self.publish_symbol_type_entry(symbol, operand);

        operand
    }

    /// Reserve one checked static expression variable for a node.
    pub(in crate::check) fn reserve_static_expression(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::Expression>,
        condition: Condition,
    ) -> VariableId {
        let node = id.into_global_any(module);
        let origin = Origin::Node(node);
        let variable = self.reserve_node_static(module, node);
        let term = StaticTerm::Expression(id.into_global(module));
        let term = self.push_term(term);

        self.equate_static(origin, variable, term, condition);

        variable
    }

    /// Return one operand for a checked DIR type id.
    pub(in crate::check) fn type_operand(&self, id: dir::GlobalTypeId) -> Option<TypeOperand> {
        self.operands.types.get(&id).copied()
    }

    /// Publish one operand for a checked DIR type id.
    pub(in crate::check) fn publish_type_operand(
        &mut self,
        id: dir::GlobalTypeId,
        operand: TypeOperand,
    ) {
        if let Some(previous) = self.operands.types.get(&id) {
            assert_eq!(
                *previous, operand,
                "check type id {id:?} already has a different operand"
            );

            return;
        }

        self.operands.types.insert(id, operand);
    }

    /// Return one operand for a checked DIR static id.
    pub(in crate::check) fn static_operand(
        &self,
        id: dir::GlobalStaticId,
    ) -> Option<StaticOperand> {
        self.operands.statics.get(&id).copied()
    }

    /// Return one checked DIR type id as an operand.
    pub(in crate::check) fn type_id_operand(&self, id: dir::GlobalTypeId) -> TypeOperand {
        self.type_operand(id).unwrap_or(TypeOperand::Type(id))
    }

    /// Return one checked DIR static id as an operand.
    pub(in crate::check) fn static_id_operand(&self, id: dir::GlobalStaticId) -> StaticOperand {
        self.static_operand(id).unwrap_or(StaticOperand::Static(id))
    }

    /// Return one checked local node type operand.
    pub(in crate::check) fn local_node_type<T: dir::Node + Clone>(
        &self,
        module: ModuleId,
        id: dir::LocalNodeId<T>,
    ) -> Option<TypeOperand> {
        let node = id.into_global_any(module);

        self.node_type(node)
    }

    /// Return one checked global node type operand.
    pub(in crate::check) fn node_type(&self, node: dir::GlobalNodeIdAny) -> Option<TypeOperand> {
        self.operands.node_types.get(&node).copied()
    }

    /// Return one checked local node type operand.
    pub(in crate::check) fn require_local_node_type<T: dir::Node + Clone>(
        &self,
        module: ModuleId,
        id: dir::LocalNodeId<T>,
    ) -> TypeOperand {
        let node = id.into_global_any(module);
        self.require_node_type(node)
    }

    /// Return one checked global node type operand.
    pub(in crate::check) fn require_node_type(&self, node: dir::GlobalNodeIdAny) -> TypeOperand {
        self.node_type(node).unwrap_or_else(|| {
            let module = self.module(node.module_id);
            let span = module.parsed.tree.get_span_by_id(node.local_id.id);

            panic!(
                "check node {node:?} in {:?} at {span:?} has no checked type",
                module.module.uri
            )
        })
    }

    /// Return one checked symbol type operand, caching it when missing.
    pub(in crate::check) fn require_symbol_type(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> TypeOperand {
        if let Some(operand) = self.operands.symbol_types.get(&symbol).copied() {
            return operand;
        }

        if let Some(target) = self.import_alias_target(symbol) {
            let operand = self.require_symbol_type(module, target);

            self.publish_symbol_type_operand(symbol, operand, Condition::Always);

            return operand;
        }

        if self.modules.contains_key(&symbol.module_id) {
            panic!("check symbol {symbol:?} has no checked type");
        }

        self.require_dependency_symbol_type(module, symbol)
    }

    /// Return one dependency symbol type operand, caching it when missing.
    fn require_dependency_symbol_type(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> TypeOperand {
        assert!(
            self.module(module).dependencies.contains(&symbol.module_id),
            "dependency symbol module must be visible"
        );
        let source = self
            .dependency(symbol.module_id)
            .types
            .get_symbol_type_id(symbol)
            .unwrap_or_else(|| panic!("dependency symbol {symbol:?} has no checked type"));
        let operand = self.import_dependency_type_operand(module, symbol.module_id, source);

        self.cache_symbol_type_operand(symbol, operand)
    }

    /// Return one type operand as a type variable.
    pub(in crate::check) fn ensure_type_operand_variable(
        &mut self,
        module: ModuleId,
        origin: Origin,
        operand: TypeOperand,
        condition: Condition,
    ) -> VariableId {
        match operand {
            TypeOperand::Variable(variable) => variable,
            TypeOperand::Term(term) => {
                let variable = self.allocate_variable(module, VariableKind::Type, origin);
                let term = self.term(term).clone();

                self.equate_type(variable, term, condition);

                variable
            }
            TypeOperand::Type(ty) => {
                let variable = self.allocate_variable(module, VariableKind::Type, origin);
                let term = TypeTerm::Type(ty);

                self.equate_type(variable, term, condition);

                variable
            }
        }
    }

    /// Return one checked symbol static variable.
    pub(in crate::check) fn require_local_symbol_static_variable(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        match self.operands.symbol_statics.get(&symbol).copied() {
            Some(StaticOperand::Variable(variable)) => variable,
            Some(StaticOperand::Term(_) | StaticOperand::Static(_)) => {
                panic!("check symbol {symbol:?} has a static operand, not a solver slot")
            }
            None => panic!("check symbol {symbol:?} has no checked static"),
        }
    }

    /// Publish one checked type table entry for a node.
    fn publish_node_type_entry(&mut self, node: dir::GlobalNodeIdAny, operand: TypeOperand) {
        if self.operands.node_types.contains_key(&node) {
            panic!("check node {node:?} already has checked type");
        }

        self.operands.node_types.insert(node, operand);
    }

    /// Publish one checked static table entry for a node.
    fn publish_node_static_entry(&mut self, node: dir::GlobalNodeIdAny, operand: StaticOperand) {
        if self.operands.node_statics.contains_key(&node) {
            panic!("check node {node:?} already has checked static");
        }

        self.operands.node_statics.insert(node, operand);
    }

    /// Publish one checked type table entry for a symbol.
    fn publish_symbol_type_entry(&mut self, symbol: dir::GlobalSymbolId, operand: TypeOperand) {
        if self.operands.symbol_types.contains_key(&symbol) {
            panic!("check symbol {symbol:?} already has checked type");
        }

        self.operands.symbol_types.insert(symbol, operand);
    }

    /// Publish one checked static table entry for a symbol.
    fn publish_symbol_static_entry(&mut self, symbol: dir::GlobalSymbolId, operand: StaticOperand) {
        if self.operands.symbol_statics.contains_key(&symbol) {
            panic!("check symbol {symbol:?} already has checked static");
        }

        self.operands.symbol_statics.insert(symbol, operand);
    }

    /// Create one checked operand for a type term.
    fn create_type_term_operand(
        &mut self,
        module: ModuleId,
        origin: Origin,
        term: TypeTerm,
    ) -> TypeOperand {
        if term.is_stable(self) {
            let term = self.push_term(term);

            return TypeOperand::Term(term);
        }

        let variable = self.allocate_variable(module, VariableKind::Type, origin);

        self.equate_type(variable, term, Condition::Always);

        TypeOperand::Variable(variable)
    }
}
