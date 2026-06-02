use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;

use crate::CompilerResult;
use crate::check::{
    Condition, GenericSlot, GenericSlotHeader, GenericSlotId, Origin, StaticOperand,
    StaticSolution, StaticTerm, TypeOperand, TypeSolution, TypeTerm, VariableId, VariableKind,
};

use super::CheckState;

impl CheckState<'_> {
    /// Load checked dependency modules visible to component modules.
    pub(in crate::check) fn load_module_dependencies(&mut self) -> CompilerResult<()> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();

        // load dependencies in stable component order
        for module in modules {
            let dependencies = self.collect_dependency_modules(module);
            for dependency in dependencies {
                self.load_dependency(dependency)?;
                self.module_mut(module).dependencies.insert(dependency);
            }
        }

        Ok(())
    }

    /// Return dependency modules that can be named from one component module.
    fn collect_dependency_modules(&self, module: ModuleId) -> IndexSet<ModuleId> {
        let mut dependencies = IndexSet::new();
        let imports = &self.module(module).resolved.imports;

        // include direct dependency modules
        for dependency in &imports.dependencies {
            if !self.modules.contains_key(dependency) {
                dependencies.insert(*dependency);
            }
        }

        // include resolved explicit import targets
        for (_, target) in imports.symbol_targets() {
            let dependency = target.module_id;
            if !self.modules.contains_key(&dependency) {
                dependencies.insert(dependency);
            }
        }

        // include resolved namespace import targets
        for target in imports.target_by_symbol.values() {
            let dir::ImportTarget::Namespace(dependency) = target else {
                continue;
            };
            if !self.modules.contains_key(dependency) {
                dependencies.insert(*dependency);
            }
        }

        // include resolved profile global targets
        for target in imports
            .global_symbol_by_key
            .values()
            .flat_map(|symbols| symbols.iter().copied())
        {
            let dependency = target.module_id;
            if !self.modules.contains_key(&dependency) {
                dependencies.insert(dependency);
            }
        }

        // include syntax-required language items
        for target in imports.language_symbols() {
            let dependency = target.module_id;
            if !self.modules.contains_key(&dependency) {
                dependencies.insert(dependency);
            }
        }

        dependencies
    }

    /// Import one dependency symbol value as a local check variable.
    pub(in crate::check) fn import_symbol_static_variable(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        if let Some(operand) = self.operands.symbol_statics.get(&symbol).copied() {
            return match operand {
                StaticOperand::Variable(variable) => variable,
                StaticOperand::Term(_) | StaticOperand::Static(_) => {
                    panic!("check symbol {symbol:?} has a static operand, not a solver slot")
                }
            };
        }

        assert!(
            self.module(module).dependencies.contains(&symbol.module_id),
            "dependency static symbol module must be visible"
        );

        let source = self
            .dependency(symbol.module_id)
            .statics
            .get_symbol_static_id(symbol)
            .unwrap_or_else(|| panic!("dependency static symbol {symbol:?} has no checked value"));

        match self.import_dependency_static_operand(source) {
            StaticOperand::Variable(variable) => {
                self.cache_symbol_static_operand(symbol, variable.into());

                variable
            }
            StaticOperand::Term(term) => {
                let origin = Origin::Symbol(symbol);
                let variable = self.allocate_variable(module, VariableKind::Static, origin);

                self.equate_static(origin, variable, term, Condition::Always);
                self.cache_symbol_static_operand(symbol, variable.into());

                variable
            }
            StaticOperand::Static(value) => {
                let origin = Origin::Symbol(symbol);
                let variable = self.allocate_variable(module, VariableKind::Static, origin);

                self.equate_static(origin, variable, value, Condition::Always);
                self.cache_symbol_static_operand(symbol, variable.into());

                variable
            }
        }
    }

    /// Import one dependency type id as a local check operand.
    pub(in crate::check) fn import_dependency_type_operand(
        &mut self,
        module: ModuleId,
        dependency: ModuleId,
        source: dir::GlobalTypeId,
    ) -> TypeOperand {
        assert_eq!(
            source.module_id, dependency,
            "dependency type id must belong to the imported module"
        );
        let parameter = match self.dependency(dependency).types.get_type(source.local_id) {
            dir::Type::Parameter(parameter) => Some(*parameter),
            _ => None,
        };
        if let Some(parameter) = parameter {
            return self.import_type_parameter_operand(module, source, parameter);
        }

        self.type_id_operand(source)
    }

    /// Return one checked type id as an operand in one module.
    pub(in crate::check) fn global_type_operand(
        &mut self,
        module: ModuleId,
        source: dir::GlobalTypeId,
    ) -> TypeOperand {
        let parameter = match self.global_type_value(source) {
            dir::Type::Parameter(parameter) => Some(*parameter),
            _ => None,
        };
        if let Some(parameter) = parameter {
            return TypeOperand::Variable(self.generic_parameter_variable(module, parameter));
        }
        if self.modules.contains_key(&source.module_id) {
            return self.type_id_operand(source);
        }

        self.import_dependency_type_operand(module, source.module_id, source)
    }

    /// Return the dependency type attached to one source node.
    pub(in crate::check) fn require_dependency_node_type(
        &mut self,
        module: ModuleId,
        dependency: ModuleId,
        source: dir::GlobalNodeIdAny,
    ) -> TypeOperand {
        assert_eq!(
            source.module_id, dependency,
            "dependency type node must belong to the imported module"
        );

        let source = self
            .dependency(dependency)
            .types
            .get_node_type_id(source)
            .unwrap_or_else(|| panic!("checked dependency node {source:?} has no type"));

        self.import_dependency_type_operand(module, dependency, source)
    }

    /// Import one dependency static id as a local check operand.
    fn import_dependency_static_operand(&mut self, source: dir::GlobalStaticId) -> StaticOperand {
        self.static_id_operand(source)
    }

    /// Cache one checked type parameter as a generic variable operand.
    fn import_type_parameter_operand(
        &mut self,
        module: ModuleId,
        target_id: dir::GlobalTypeId,
        parameter: dir::GenericParameterRef,
    ) -> TypeOperand {
        if let Some(operand) = self.type_operand(target_id) {
            return operand;
        }
        let variable = self.generic_parameter_variable(module, parameter);
        let operand = TypeOperand::Variable(variable);
        self.publish_type_operand(target_id, operand);

        operand
    }

    /// Return one generic parameter as a check variable in one module.
    pub(in crate::check) fn generic_parameter_variable(
        &mut self,
        module: ModuleId,
        parameter: dir::GenericParameterRef,
    ) -> VariableId {
        let slot_id = GenericSlotId::from(parameter);
        if self.modules.contains_key(&parameter.owner.module_id) {
            return self
                .generic_slot_variable(slot_id)
                .unwrap_or_else(|| panic!("generic parameter {parameter:?} has no check slot"));
        }
        if let Some(variable) = self.dependency_generic_slot_variable(module, slot_id) {
            return variable;
        }
        let dependency = parameter.owner.module_id;
        let slot = self
            .dependency(dependency)
            .generic_slot(parameter)
            .unwrap_or_else(|| panic!("dependency generic parameter {parameter:?} has no slot"));
        let generic = self.import_generic_slot(module, slot, dependency);
        let variable = self.allocate_dependency_generic_slot_variable(module, &generic);

        self.attach_dependency_generic_slot(variable, generic);
        self.insert_generic_parameter_solution(variable, parameter);

        variable
    }

    /// Insert the stable self-solution for one generic parameter variable.
    fn insert_generic_parameter_solution(
        &mut self,
        variable: VariableId,
        parameter: dir::GenericParameterRef,
    ) {
        match self.variable(variable).kind {
            VariableKind::Type => {
                let slot = GenericSlotId::from(parameter);
                let term = self.push_term(TypeTerm::Parameter(slot));

                self.insert_known_solution(variable, TypeSolution::Term(term).into());
            }
            VariableKind::Static => {
                let term = self.push_term(StaticTerm::Parameter(parameter.into()));

                self.insert_known_solution(variable, StaticSolution::Term(term).into());
            }
        }
    }

    /// Import one committed generic slot as check generic metadata.
    fn import_generic_slot(
        &mut self,
        module: ModuleId,
        slot: dir::GenericSlot,
        dependency: ModuleId,
    ) -> GenericSlot {
        let owner = self
            .dependency(dependency)
            .generics
            .get_template(slot.template())
            .owner;
        let generic_slot = GenericSlotHeader {
            owner,
            key: slot.key(),
            index: slot.index(),
            origin: slot.origin(),
        };

        match slot {
            dir::GenericSlot::Type {
                variance,
                constraint,
                default,
                ..
            } => GenericSlot::Type {
                slot: generic_slot,
                variance,
                constraint: constraint
                    .map(|id| self.import_dependency_type_operand(module, dependency, id)),
                default: default
                    .map(|id| self.import_dependency_type_operand(module, dependency, id)),
            },
            dir::GenericSlot::VariadicType {
                variance,
                constraint,
                default,
                ..
            } => GenericSlot::VariadicType {
                slot: generic_slot,
                variance,
                constraint: constraint
                    .map(|id| self.import_dependency_type_operand(module, dependency, id)),
                default: default
                    .map(|id| self.import_dependency_type_operand(module, dependency, id)),
            },
            dir::GenericSlot::Static {
                constraint,
                default,
                ..
            } => GenericSlot::Static {
                slot: generic_slot,
                constraint: constraint
                    .map(|id| self.import_dependency_type_operand(module, dependency, id)),
                default: default.map(|id| self.import_dependency_static_operand(id)),
            },
            dir::GenericSlot::VariadicStatic {
                constraint,
                default,
                ..
            } => GenericSlot::VariadicStatic {
                slot: generic_slot,
                constraint: constraint
                    .map(|id| self.import_dependency_type_operand(module, dependency, id)),
                default: default.map(|id| self.import_dependency_static_operand(id)),
            },
        }
    }

    /// Return a variable for one dependency generic parameter.
    fn allocate_dependency_generic_slot_variable(
        &mut self,
        module: ModuleId,
        generic: &GenericSlot,
    ) -> VariableId {
        let kind = if generic.is_static() {
            VariableKind::Static
        } else {
            VariableKind::Type
        };
        let source = Origin::Node(self.module(module).bound.module_node.into_global(module));

        self.allocate_variable(module, kind, source)
    }
}
