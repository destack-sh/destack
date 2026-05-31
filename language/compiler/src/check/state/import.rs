use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;

use crate::CompilerResult;
use crate::check::{
    Condition, GenericSlot, GenericSlotHeader, GenericSlotId, Origin, Solution, StaticTerm,
    TypeOperand, TypeTerm, VariableId, VariableKind,
};

use super::CheckState;

impl CheckState<'_> {
    /// Import checked dependency modules visible to component modules.
    pub(in crate::check) fn import_module_dependencies(&mut self) -> CompilerResult<()> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();

        // load dependency modules without broad symbol materialization
        for module in modules {
            let dependencies = self.dependency_modules(module);

            for dependency in dependencies {
                self.load_dependency(dependency)?;
                self.module_mut(module).dependencies.insert(dependency);
            }

            let namespace_dependencies = self.namespace_dependency_modules(module);
            for dependency in namespace_dependencies {
                self.import_dependency_module_symbols(module, dependency)?;
            }

            let imports = self
                .module(module)
                .resolved
                .imports
                .symbol_targets()
                .collect::<Vec<_>>();
            for (alias, target) in imports {
                if self.modules.contains_key(&target.module_id) {
                    continue;
                }

                self.import_dependency_symbol_alias(module, alias, target)?;
            }

            let symbols = self.dependency_symbols(module);
            for symbol in symbols {
                self.import_dependency_symbol(module, symbol)?;
            }
        }

        Ok(())
    }

    /// Return dependency modules that can be named from one component module.
    fn dependency_modules(&self, module: ModuleId) -> IndexSet<ModuleId> {
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

    /// Return namespace import modules that require broad member visibility.
    fn namespace_dependency_modules(&self, module: ModuleId) -> IndexSet<ModuleId> {
        let mut dependencies = IndexSet::new();
        let imports = &self.module(module).resolved.imports;

        // include namespace import target modules
        for target in imports.target_by_symbol.values() {
            let dir::ImportTarget::Namespace(dependency) = target else {
                continue;
            };
            if !self.modules.contains_key(dependency) {
                dependencies.insert(*dependency);
            }
        }

        dependencies
    }

    /// Return concrete dependency symbols selected by resolve.
    fn dependency_symbols(&self, module: ModuleId) -> IndexSet<dir::GlobalSymbolId> {
        let mut symbols = IndexSet::new();
        let imports = &self.module(module).resolved.imports;

        // include profile global symbols
        for target in imports
            .global_symbol_by_key
            .values()
            .flat_map(|symbols| symbols.iter().copied())
        {
            if !self.modules.contains_key(&target.module_id) {
                symbols.insert(target);
            }
        }

        // include syntax-required language item symbols
        for target in imports.language_symbols() {
            if !self.modules.contains_key(&target.module_id) {
                symbols.insert(target);
            }
        }

        symbols
    }

    /// Import one dependency symbol value as a local check variable.
    pub(in crate::check) fn import_symbol_static_variable(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        assert!(
            self.module(module).dependencies.contains(&symbol.module_id),
            "dependency static symbol module must be visible"
        );

        let source = self
            .dependency(symbol.module_id)
            .statics
            .get_symbol_static_id(symbol)
            .unwrap_or_else(|| panic!("dependency static symbol {symbol:?} has no checked value"));

        self.import_dependency_static_variable(symbol.module_id, source)
    }

    /// Import one dependency type id as a local check operand.
    pub(in crate::check) fn import_dependency_type_operand(
        &mut self,
        module: ModuleId,
        dependency: ModuleId,
        source: dir::LocalTypeId,
    ) -> TypeOperand {
        let source_id = source.into_global(dependency);
        let parameter = match self.dependency(dependency).types.get_type(source) {
            dir::Type::Parameter(parameter) => Some(*parameter),
            _ => None,
        };
        if let Some(parameter) = parameter {
            self.import_type_parameter_variable(module, source_id, parameter, dependency);
        }

        self.materialize_type_by_id(source_id).into()
    }

    /// Import the dependency type attached to one source node.
    pub(in crate::check) fn import_dependency_require_node_type(
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

    /// Import one dependency static id as a local check variable.
    fn import_dependency_static_variable(
        &mut self,
        dependency: ModuleId,
        source: dir::LocalStaticId,
    ) -> VariableId {
        let origin = Origin::Node(
            self.dependency(dependency)
                .module_node
                .into_global(dependency),
        );

        self.materialize_static_by_id(origin, source.into_global(dependency))
    }

    /// Import one checked dependency symbol through a local alias.
    fn import_dependency_symbol_alias(
        &mut self,
        module: ModuleId,
        alias: dir::GlobalSymbolId,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        assert!(
            self.module(module).dependencies.contains(&target.module_id),
            "dependency alias target module must be visible"
        );
        let mut imported = IndexSet::new();

        self.import_dependency_symbol_tree(module, target, &mut imported)?;
        self.set_import_alias_outputs(module, alias, target);

        Ok(())
    }

    /// Materialize checked values attached to one dependency symbol.
    fn materialize_dependency_symbol(&mut self, module: ModuleId, target: dir::GlobalSymbolId) {
        let ty = self
            .dependency(target.module_id)
            .types
            .get_symbol_type_id(target);
        if let Some(ty) = ty {
            self.import_dependency_type_operand(module, target.module_id, ty);
        }
        let value = self
            .dependency(target.module_id)
            .statics
            .get_symbol_static_id(target);
        if let Some(value) = value {
            self.import_dependency_static_variable(target.module_id, value);
        }
    }

    /// Set local import alias operands from a checked dependency symbol.
    fn set_import_alias_outputs(
        &mut self,
        module: ModuleId,
        alias: dir::GlobalSymbolId,
        target: dir::GlobalSymbolId,
    ) {
        assert_eq!(
            alias.module_id, module,
            "check import alias symbol must be local"
        );

        let ty = self
            .dependency(target.module_id)
            .types
            .get_symbol_type_id(target);
        if let Some(ty) = ty {
            self.set_import_alias_type(module, alias, ty, target.module_id);
        }
        let value = self
            .dependency(target.module_id)
            .statics
            .get_symbol_static_id(target);
        if let Some(value) = value {
            self.set_import_alias_static(module, alias, value, target.module_id);
        }
    }

    /// Set one local import alias type operand.
    fn set_import_alias_type(
        &mut self,
        module: ModuleId,
        alias: dir::GlobalSymbolId,
        source: dir::LocalTypeId,
        dependency: ModuleId,
    ) {
        let imported = self.import_dependency_type_operand(module, dependency, source);
        let term = imported.to_type_term(self);

        self.output_symbol_type(module, alias, term, Condition::Always);
    }

    /// Set one local import alias static operand.
    fn set_import_alias_static(
        &mut self,
        module: ModuleId,
        alias: dir::GlobalSymbolId,
        source: dir::LocalStaticId,
        dependency: ModuleId,
    ) {
        let imported = self.import_dependency_static_variable(dependency, source);
        let variable = self.output_symbol_static_variable(module, alias);
        let term = StaticTerm::Variable(imported);

        self.equate_static(variable, term, Condition::Always);
    }

    /// Import one checked type parameter as a generic variable.
    fn import_type_parameter_variable(
        &mut self,
        module: ModuleId,
        target_id: dir::GlobalTypeId,
        parameter: dir::GenericParameterRef,
        dependency: ModuleId,
    ) {
        if self.inference.contains_type_variable_id(target_id) {
            return;
        }
        let slot = self
            .dependency(dependency)
            .generic_slot(parameter)
            .unwrap_or_else(|| panic!("dependency generic parameter {parameter:?} has no slot"));
        let slot_id = GenericSlotId::from(parameter);
        if let Some(variable) = self.imported_generic_slot_variable(module, slot_id) {
            self.inference.insert_type_variable_id(target_id, variable);

            return;
        }

        let generic = self.import_generic_slot(module, slot, dependency);
        let variable = self.allocate_imported_generic_slot_variable(module, &generic);

        self.attach_imported_generic_slot(module, variable, generic);
        self.inference.insert_type_variable_id(target_id, variable);

        match self.variable(variable).kind {
            VariableKind::Type => {
                let term = self.push_term(TypeTerm::Parameter(slot_id));
                self.insert_known_solution(variable, Solution::Type(term.into()));
            }
            VariableKind::Static => {
                let term = self.push_term(StaticTerm::Parameter(parameter.into()));
                self.insert_known_solution(variable, Solution::Static(term.into()));
            }
        }
    }

    /// Return the imported generic variable for one source slot.
    fn imported_generic_slot_variable(
        &self,
        module: ModuleId,
        slot_id: GenericSlotId,
    ) -> Option<VariableId> {
        self.module(module)
            .imported_generic_by_slot
            .get(&slot_id)
            .copied()
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
                default: default.map(|id| {
                    self.import_dependency_static_variable(dependency, id)
                        .into()
                }),
            },
            dir::GenericSlot::VariadicStatic {
                constraint,
                default,
                ..
            } => GenericSlot::VariadicStatic {
                slot: generic_slot,
                constraint: constraint
                    .map(|id| self.import_dependency_type_operand(module, dependency, id)),
                default: default.map(|id| {
                    self.import_dependency_static_variable(dependency, id)
                        .into()
                }),
            },
        }
    }

    /// Return a variable for one imported generic parameter.
    fn allocate_imported_generic_slot_variable(
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

impl CheckState<'_> {
    /// Import one checked symbol from a dependency module into one component module.
    fn import_dependency_symbol(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        assert!(
            self.module(module).dependencies.contains(&symbol.module_id),
            "dependency symbol module must be visible"
        );
        let mut imported = IndexSet::new();

        self.import_dependency_symbol_tree(module, symbol, &mut imported)?;

        Ok(())
    }

    /// Import one checked symbol and its owned members.
    fn import_dependency_symbol_tree(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        imported: &mut IndexSet<dir::GlobalSymbolId>,
    ) -> CompilerResult<()> {
        if !imported.insert(symbol) {
            return Ok(());
        }

        self.materialize_dependency_symbol(module, symbol);

        let owned_symbols =
            Self::dependency_owned_symbols(&self.dependency(symbol.module_id).bindings, symbol);
        for owned_symbol in owned_symbols {
            self.import_dependency_symbol_tree(module, owned_symbol, imported)?;
        }

        Ok(())
    }

    /// Return symbols owned by one dependency symbol.
    fn dependency_owned_symbols(
        bindings: &dir::BindingTable<'_>,
        owner: dir::GlobalSymbolId,
    ) -> Vec<dir::GlobalSymbolId> {
        let Some(scope) = bindings.scope_for_owner(owner.local_id) else {
            return Vec::new();
        };
        let scope = bindings.get_scope(scope);
        let named = scope
            .named_symbols()
            .map(|(_, symbol)| symbol.into_global(owner.module_id));
        let anonymous = scope
            .anonymous_symbols()
            .map(|symbol| symbol.into_global(owner.module_id));

        named.chain(anonymous).collect()
    }

    /// Import all checked symbols from one namespace dependency module.
    fn import_dependency_module_symbols(
        &mut self,
        module: ModuleId,
        dependency_module: ModuleId,
    ) -> CompilerResult<()> {
        assert!(
            self.module(module)
                .dependencies
                .contains(&dependency_module),
            "namespace dependency module must be visible"
        );
        let symbols = self
            .dependency(dependency_module)
            .bindings
            .symbol_ids()
            .map(|symbol| symbol.into_global(dependency_module))
            .collect::<Vec<_>>();

        // import every namespace-visible checked symbol value
        for symbol in symbols {
            self.materialize_dependency_symbol(module, symbol);
        }

        Ok(())
    }
}
