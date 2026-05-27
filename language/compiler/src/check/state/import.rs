use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::{IndexMap, IndexSet};

use crate::CompilerResult;
use crate::check::{
    ConstraintOrigin, GenericParameter, GenericSlot, GenericSlotId, Solution, StaticTerm, TypeTerm,
    VariableId, VariableKind,
};

use super::CheckState;

/// Imported checked ids for one module.
#[derive(Debug)]
pub(in crate::check) struct ImportTable {
    /// Dependency modules already imported into this module.
    pub(in crate::check) modules: IndexSet<ModuleId>,
    /// Imported symbol kinds keyed by source symbol.
    pub(in crate::check) symbol_kinds: IndexMap<dir::GlobalSymbolId, dir::SymbolKind>,
    /// Imported type ids keyed by source id.
    pub(in crate::check) types: IndexMap<dir::GlobalTypeId, dir::LocalTypeId>,
    /// Imported static ids keyed by source id.
    pub(in crate::check) statics: IndexMap<dir::GlobalStaticId, dir::LocalStaticId>,
}

/// State for copying one checked module into the active module.
struct CheckedModuleImport<'a> {
    /// Source checked module.
    source_module: ModuleId,
    /// Source checked type table.
    source_types: &'a dir::TypeTable<'a>,
    /// Source checked static table.
    source_statics: &'a dir::StaticTable<'a>,
    /// Source checked generic table.
    source_generics: &'a dir::GenericTable<'a>,
    /// Imported type ids keyed by source id.
    imported_types: IndexMap<dir::LocalTypeId, dir::LocalTypeId>,
    /// Imported static ids keyed by source id.
    imported_statics: IndexMap<dir::LocalStaticId, dir::LocalStaticId>,
}

impl ImportTable {
    /// Create empty import state.
    pub(in crate::check) fn new() -> Self {
        Self {
            modules: IndexSet::new(),
            symbol_kinds: IndexMap::new(),
            types: IndexMap::new(),
            statics: IndexMap::new(),
        }
    }
}

impl<'a> CheckedModuleImport<'a> {
    /// Create import state for one checked source module.
    fn new(
        source_module: ModuleId,
        source_types: &'a dir::TypeTable<'a>,
        source_statics: &'a dir::StaticTable<'a>,
        source_generics: &'a dir::GenericTable<'a>,
    ) -> Self {
        Self {
            source_module,
            source_types,
            source_statics,
            source_generics,
            imported_types: IndexMap::new(),
            imported_statics: IndexMap::new(),
        }
    }

    /// Return the committed slot for one source parameter type.
    fn source_generic_slot(&self, parameter: dir::ParameterType) -> Option<dir::GenericSlot> {
        self.source_generics
            .iter_slots()
            .map(|(_, slot)| slot)
            .find(|slot| {
                slot.owner() == parameter.owner
                    && slot.key() == parameter.key
                    && slot.index() == parameter.index
            })
            .cloned()
    }
}

impl CheckState<'_> {
    /// Import one checked dependency type into this module.
    pub(in crate::check) fn import_dependency_type(
        &mut self,
        module: ModuleId,
        source_module: ModuleId,
        source: dir::LocalTypeId,
        source_types: &dir::TypeTable<'_>,
        source_statics: &dir::StaticTable<'_>,
        source_generics: &dir::GenericTable<'_>,
    ) -> dir::LocalTypeId {
        let mut import =
            CheckedModuleImport::new(source_module, source_types, source_statics, source_generics);

        self.import_type_id(module, source, &mut import)
    }

    /// Import checked values attached to one dependency symbol.
    pub(in crate::check) fn import_symbol(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        target: dir::GlobalSymbolId,
        source_types: &dir::TypeTable<'_>,
        source_statics: &dir::StaticTable<'_>,
        source_generics: &dir::GenericTable<'_>,
    ) {
        let mut import = CheckedModuleImport::new(
            target.module_id,
            source_types,
            source_statics,
            source_generics,
        );

        if let Some(ty) = source_types.get_symbol_type_id(target) {
            self.import_symbol_type(module, symbol, ty, &mut import);
        }
        if let Some(value) = source_statics.get_symbol_static_id(target) {
            self.import_symbol_static(module, symbol, value, &mut import);
        }
    }

    /// Import one checked symbol type.
    fn import_symbol_type(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        source: dir::LocalTypeId,
        import: &mut CheckedModuleImport<'_>,
    ) {
        let ty = self.import_type_id(module, source, import);
        let variable = self.intern_symbol_type_variable(module, symbol);
        let term = TypeTerm::Variable(self.materialize_type_id(ty.into_global(module)));

        self.define_type(module, variable, term);
    }

    /// Import one checked symbol static value.
    fn import_symbol_static(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        source: dir::LocalStaticId,
        import: &mut CheckedModuleImport<'_>,
    ) {
        let value = self.import_static_id(module, source, import);
        let variable = self.intern_symbol_static_variable(module, symbol);
        let term = StaticTerm::Variable(
            self.materialize_static_id(ConstraintOrigin::Symbol(symbol), value.into_global(module)),
        );

        self.define_static(module, variable, term);
    }

    /// Import one checked type id into this module.
    fn import_type_id(
        &mut self,
        module: ModuleId,
        source: dir::LocalTypeId,
        import: &mut CheckedModuleImport<'_>,
    ) -> dir::LocalTypeId {
        let source_id = source.into_global(import.source_module);
        if let Some(target) = self.imports(module).types.get(&source_id).copied() {
            return target;
        }
        if let Some(target) = import.imported_types.get(&source).copied() {
            return target;
        }
        let source_ty = import.source_types.get_type(source).clone();
        let ty = self.import_type(module, source_ty.clone(), import);
        let source_node = self.input(module).bound.module_node;
        let target = self
            .output_mut(module)
            .types
            .insert_imported_type_from_any(ty, source_node);

        import.imported_types.insert(source, target);
        self.imports_mut(module).types.insert(source_id, target);
        if let dir::Type::Parameter(parameter) = source_ty {
            self.record_imported_type_parameter(module, target, parameter, import);
        }

        target
    }

    /// Record one imported type parameter as a real generic variable.
    fn record_imported_type_parameter(
        &mut self,
        module: ModuleId,
        target: dir::LocalTypeId,
        parameter: dir::ParameterType,
        import: &mut CheckedModuleImport<'_>,
    ) {
        let target_id = target.into_global(module);
        if self.variables.type_by_id.contains_key(&target_id) {
            return;
        }
        let Some(slot) = import.source_generic_slot(parameter) else {
            return;
        };
        let slot_id = GenericSlotId::from(parameter);
        if let Some(variable) = self.generic_slot_variable(slot_id) {
            self.variables.type_by_id.insert(target_id, variable);

            return;
        }

        let generic = self.import_generic_parameter(module, slot, import);
        let variable = self.imported_generic_parameter_variable(module, &generic);

        self.record_generic_parameter(variable, generic);
        self.variables.type_by_id.insert(target_id, variable);

        match self.variable(variable).kind {
            VariableKind::Type => {
                let term = self.terms.push(TypeTerm::Parameter(slot_id));
                self.solve_variable(variable, Solution::Type(term));
            }
            VariableKind::Static => {
                self.solve_imported_static_parameter(variable, parameter);
            }
        }
    }

    /// Import one committed generic slot as check generic metadata.
    fn import_generic_parameter(
        &mut self,
        module: ModuleId,
        slot: dir::GenericSlot,
        import: &mut CheckedModuleImport<'_>,
    ) -> GenericParameter {
        let owner = slot.owner();
        let generic_slot = GenericSlot {
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
            } => GenericParameter::Type {
                slot: generic_slot,
                variance,
                constraint: constraint.map(|id| self.import_type_variable(module, id, import)),
                default: default.map(|id| self.import_type_variable(module, id, import)),
            },
            dir::GenericSlot::VariadicType {
                variance,
                constraint,
                default,
                ..
            } => GenericParameter::VariadicType {
                slot: generic_slot,
                variance,
                constraint: constraint.map(|id| self.import_type_variable(module, id, import)),
                default: default.map(|id| self.import_type_variable(module, id, import)),
            },
            dir::GenericSlot::Static {
                constraint,
                default,
                ..
            } => GenericParameter::Static {
                slot: generic_slot,
                constraint: constraint.map(|id| self.import_type_variable(module, id, import)),
                default: default.map(|id| self.import_static_variable(module, owner, id, import)),
            },
            dir::GenericSlot::VariadicStatic {
                constraint,
                default,
                ..
            } => GenericParameter::VariadicStatic {
                slot: generic_slot,
                constraint: constraint.map(|id| self.import_type_variable(module, id, import)),
                default: default.map(|id| self.import_static_variable(module, owner, id, import)),
            },
        }
    }

    /// Return the imported type variable for one dependency type id.
    fn import_type_variable(
        &mut self,
        module: ModuleId,
        source: dir::LocalTypeId,
        import: &mut CheckedModuleImport<'_>,
    ) -> VariableId {
        let target = self.import_type_id(module, source, import);

        self.materialize_type_id(target.into_global(module))
    }

    /// Return the imported static variable for one dependency static id.
    fn import_static_variable(
        &mut self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        source: dir::LocalStaticId,
        import: &mut CheckedModuleImport<'_>,
    ) -> VariableId {
        let target = self.import_static_id(module, source, import);

        self.materialize_static_id(ConstraintOrigin::Symbol(owner), target.into_global(module))
    }

    /// Return a variable for one imported generic parameter.
    fn imported_generic_parameter_variable(
        &mut self,
        module: ModuleId,
        generic: &GenericParameter,
    ) -> VariableId {
        let kind = if generic.is_static() {
            VariableKind::Static
        } else {
            VariableKind::Type
        };

        match generic.slot().key {
            dir::GenericSlotKey::Symbol(symbol) if generic.is_static() => {
                self.intern_symbol_static_variable(module, symbol)
            }
            dir::GenericSlotKey::Symbol(symbol) => self.intern_symbol_type_variable(module, symbol),
            dir::GenericSlotKey::Generated(_) => {
                let source = ConstraintOrigin::Symbol(generic.slot().owner);

                self.allocate_intermediate_variable(module, kind, source)
            }
        }
    }

    /// Solve one imported static parameter when it has a source symbol.
    fn solve_imported_static_parameter(
        &mut self,
        variable: VariableId,
        parameter: dir::ParameterType,
    ) {
        let dir::GenericSlotKey::Symbol(symbol) = parameter.key else {
            return;
        };
        let term = StaticTerm::Literal(dir::StaticTerm::Symbol { symbol });
        let term = self.terms.push(term);

        self.solve_variable(variable, Solution::Static(term));
    }

    /// Import one checked type into this module.
    fn import_type(
        &mut self,
        module: ModuleId,
        ty: dir::Type,
        import: &mut CheckedModuleImport<'_>,
    ) -> dir::Type {
        match ty {
            dir::Type::Parameter(_)
            | dir::Type::Error
            | dir::Type::Never
            | dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Object
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::This
            | dir::Type::Range(_) => ty,
            dir::Type::Named(named) => dir::Type::Named(dir::NamedType {
                symbol: named.symbol,
                arguments: named
                    .arguments
                    .into_iter()
                    .map(|argument| self.import_static_argument(module, argument, import))
                    .collect(),
            }),
            dir::Type::Form(form) => dir::Type::Form(dir::FormType {
                form: self.import_form(module, form.form, import),
                value: self.import_type_id(module, form.value, import),
            }),
            dir::Type::Dynamic(erased) => dir::Type::Dynamic(dir::DynamicType {
                constraint: self.import_type_id(module, erased.constraint, import),
            }),
            dir::Type::Predicate(predicate) => dir::Type::Predicate(dir::PredicateType {
                asserts: predicate.asserts,
                subject: predicate.subject,
                target: predicate
                    .target
                    .map(|target| self.import_type_id(module, target, import)),
            }),
            dir::Type::Operation(operation) => {
                dir::Type::Operation(self.import_type_operation(module, operation, import))
            }
            dir::Type::FixedArray(array) => dir::Type::FixedArray(dir::FixedArrayType {
                element: self.import_type_id(module, array.element, import),
                count: self.import_static_id(module, array.count, import),
                is_readonly: array.is_readonly,
            }),
            dir::Type::Slice(slice) => dir::Type::Slice(dir::SliceType {
                element: self.import_type_id(module, slice.element, import),
                is_readonly: slice.is_readonly,
            }),
            dir::Type::Tuple(tuple) => dir::Type::Tuple(dir::TupleType {
                form: tuple.form,
                elements: tuple
                    .elements
                    .into_iter()
                    .map(|element| dir::TypeElement {
                        label: element.label,
                        ty: self.import_type_id(module, element.ty, import),
                        is_optional: element.is_optional,
                        is_readonly: element.is_readonly,
                        is_rest: element.is_rest,
                    })
                    .collect(),
                is_readonly: tuple.is_readonly,
            }),
            dir::Type::Shape(shape) => dir::Type::Shape(dir::ShapeType {
                fields: shape
                    .fields
                    .into_iter()
                    .map(|field| dir::TypeField {
                        key: field.key,
                        ty: self.import_type_id(module, field.ty, import),
                        is_optional: field.is_optional,
                        is_readonly: field.is_readonly,
                    })
                    .collect(),
                call_signatures: shape
                    .call_signatures
                    .into_iter()
                    .map(|ty| self.import_type_id(module, ty, import))
                    .collect(),
                construct_signatures: shape
                    .construct_signatures
                    .into_iter()
                    .map(|ty| self.import_type_id(module, ty, import))
                    .collect(),
                index_signatures: shape
                    .index_signatures
                    .into_iter()
                    .map(|signature| dir::TypeIndexSignature {
                        name: signature.name,
                        key_type: self.import_type_id(module, signature.key_type, import),
                        value_type: self.import_type_id(module, signature.value_type, import),
                        is_optional: signature.is_optional,
                        is_readonly: signature.is_readonly,
                    })
                    .collect(),
            }),
            dir::Type::Function(function) => dir::Type::Function(dir::FunctionType {
                asynchrony: function.asynchrony,
                generic_parameters: function
                    .generic_parameters
                    .into_iter()
                    .map(|ty| self.import_type_id(module, ty, import))
                    .collect(),
                this_parameter: function
                    .this_parameter
                    .map(|ty| self.import_type_id(module, ty, import)),
                parameters: function
                    .parameters
                    .into_iter()
                    .map(|parameter| dir::FunctionParameterType {
                        ty: self.import_type_id(module, parameter.ty, import),
                        is_optional: parameter.is_optional,
                        is_rest: parameter.is_rest,
                    })
                    .collect(),
                return_type: function
                    .return_type
                    .map(|ty| self.import_type_id(module, ty, import)),
                is_generator: function.is_generator,
            }),
            dir::Type::Closure(closure) => dir::Type::Closure(dir::ClosureType {
                function: self.import_type_id(module, closure.function, import),
                environment: self.import_type_id(module, closure.environment, import),
            }),
            dir::Type::Union(union) => dir::Type::Union(dir::UnionType {
                elements: union
                    .elements
                    .into_iter()
                    .map(|ty| self.import_type_id(module, ty, import))
                    .collect(),
            }),
            dir::Type::Intersection(intersection) => {
                dir::Type::Intersection(dir::IntersectionType {
                    elements: intersection
                        .elements
                        .into_iter()
                        .map(|ty| self.import_type_id(module, ty, import))
                        .collect(),
                })
            }
        }
    }

    /// Import one checked memory form constructor into this module.
    fn import_form(
        &mut self,
        module: ModuleId,
        form: dir::Form,
        import: &mut CheckedModuleImport<'_>,
    ) -> dir::Form {
        match form {
            dir::Form::Borrowed { lifetime, access } => dir::Form::Borrowed {
                lifetime: self.import_static_id(module, lifetime, import),
                access: self.import_static_id(module, access, import),
            },
            dir::Form::Placed { place } => dir::Form::Placed {
                place: self.import_static_id(module, place, import),
            },
            dir::Form::Managed | dir::Form::Owned | dir::Form::Raw | dir::Form::Readonly => form,
        }
    }

    /// Import one checked type operation into this module.
    fn import_type_operation(
        &mut self,
        module: ModuleId,
        operation: dir::TypeOperation,
        import: &mut CheckedModuleImport<'_>,
    ) -> dir::TypeOperation {
        match operation {
            dir::TypeOperation::BuiltinTypeFunction(_) => operation,
            dir::TypeOperation::Conditional(conditional) => {
                dir::TypeOperation::Conditional(dir::ConditionalType {
                    distributive_symbol: conditional.distributive_symbol,
                    left: self.import_type_id(module, conditional.left, import),
                    right: self.import_type_id(module, conditional.right, import),
                    then_type: self.import_type_id(module, conditional.then_type, import),
                    else_type: self.import_type_id(module, conditional.else_type, import),
                })
            }
            dir::TypeOperation::Mapped(mapped) => dir::TypeOperation::Mapped(dir::MappedType {
                parameter: dir::MappedTypeParameter {
                    name: mapped.parameter.name,
                    symbol: mapped.parameter.symbol,
                    constraint: self.import_type_id(module, mapped.parameter.constraint, import),
                    key_remap: mapped
                        .parameter
                        .key_remap
                        .map(|ty| self.import_type_id(module, ty, import)),
                },
                modifiers: mapped.modifiers,
                value: self.import_type_id(module, mapped.value, import),
            }),
            dir::TypeOperation::Index(index) => dir::TypeOperation::Index(dir::IndexType {
                left: self.import_type_id(module, index.left, import),
                index: self.import_type_id(module, index.index, import),
            }),
            dir::TypeOperation::TemplateLiteral(template) => {
                dir::TypeOperation::TemplateLiteral(dir::TemplateLiteralType {
                    strings: template.strings,
                    spans: template
                        .spans
                        .into_iter()
                        .map(|ty| self.import_type_id(module, ty, import))
                        .collect(),
                })
            }
            dir::TypeOperation::Infer(infer) => dir::TypeOperation::Infer(dir::InferType {
                name: infer.name,
                constraint: infer
                    .constraint
                    .map(|ty| self.import_type_id(module, ty, import)),
            }),
            dir::TypeOperation::KeyOf(key) => dir::TypeOperation::KeyOf(dir::UnaryType {
                target: self.import_type_id(module, key.target, import),
            }),
        }
    }

    /// Import one checked static argument into this module.
    fn import_static_argument(
        &mut self,
        module: ModuleId,
        argument: dir::StaticArgument,
        import: &mut CheckedModuleImport<'_>,
    ) -> dir::StaticArgument {
        dir::StaticArgument {
            name: argument.name,
            value: self.import_static_id(module, argument.value, import),
        }
    }

    /// Import one checked static id into this module.
    fn import_static_id(
        &mut self,
        module: ModuleId,
        source: dir::LocalStaticId,
        import: &mut CheckedModuleImport<'_>,
    ) -> dir::LocalStaticId {
        let source_id = source.into_global(import.source_module);
        if let Some(target) = self.imports(module).statics.get(&source_id).copied() {
            return target;
        }
        if let Some(target) = import.imported_statics.get(&source).copied() {
            return target;
        }
        let term = import.source_statics.get_static(source).clone();
        let term = self.import_static(module, term, import);
        let target = self.intern_static(module, term);

        import.imported_statics.insert(source, target);
        self.imports_mut(module).statics.insert(source_id, target);

        target
    }

    /// Import one checked static value into this module.
    fn import_static(
        &mut self,
        module: ModuleId,
        term: dir::StaticTerm,
        import: &mut CheckedModuleImport<'_>,
    ) -> dir::StaticTerm {
        match term {
            dir::StaticTerm::Type { ty } => dir::StaticTerm::Type {
                ty: self.import_type_id(module, ty, import),
            },
            dir::StaticTerm::Array { elements } => dir::StaticTerm::Array {
                elements: elements
                    .into_iter()
                    .map(|term| self.import_static(module, term, import))
                    .collect(),
            },
            dir::StaticTerm::FixedArray { value, length } => dir::StaticTerm::FixedArray {
                value: Box::new(self.import_static(module, *value, import)),
                length: Box::new(self.import_static(module, *length, import)),
            },
            dir::StaticTerm::Tuple { elements } => dir::StaticTerm::Tuple {
                elements: elements
                    .into_iter()
                    .map(|term| self.import_static(module, term, import))
                    .collect(),
            },
            dir::StaticTerm::Object { properties } => dir::StaticTerm::Object {
                properties: properties
                    .into_iter()
                    .map(|property| self.import_static_property(module, property, import))
                    .collect(),
            },
            dir::StaticTerm::Struct { ty, properties } => dir::StaticTerm::Struct {
                ty: self.import_type_id(module, ty, import),
                properties: properties
                    .into_iter()
                    .map(|property| self.import_static_property(module, property, import))
                    .collect(),
            },
            term => term,
        }
    }

    /// Import one checked static property into this module.
    fn import_static_property(
        &mut self,
        module: ModuleId,
        property: dir::StaticProperty,
        import: &mut CheckedModuleImport<'_>,
    ) -> dir::StaticProperty {
        match property {
            dir::StaticProperty::Field { key, value } => dir::StaticProperty::Field {
                key,
                value: self.import_static(module, value, import),
            },
            dir::StaticProperty::Spread { value } => dir::StaticProperty::Spread {
                value: self.import_static(module, value, import),
            },
            dir::StaticProperty::Method {
                key,
                signature,
                body,
            } => dir::StaticProperty::Method {
                key,
                signature,
                body: self.import_static(module, body, import),
            },
        }
    }
}

impl CheckState<'_> {
    /// Mark one dependency module as visible to one component module.
    pub(in crate::check) fn mark_dependency_module(
        &mut self,
        module: ModuleId,
        dependency_module: ModuleId,
    ) -> CompilerResult<()> {
        if self.inputs.contains_key(&dependency_module) {
            return Ok(());
        }
        if self.imports(module).modules.contains(&dependency_module) {
            return Ok(());
        }

        self.load_dependency_input(dependency_module)?;
        self.imports_mut(module).modules.insert(dependency_module);

        Ok(())
    }

    /// Import one checked symbol from a dependency module into one component module.
    pub(in crate::check) fn import_dependency_symbol(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        if self.inputs.contains_key(&symbol.module_id) {
            return Ok(());
        }

        self.mark_dependency_module(module, symbol.module_id)?;
        let dependency = self.load_dependency_input(symbol.module_id)?;
        let bindings = dependency.bindings.clone();
        let types = dependency.types.clone();
        let statics = dependency.statics.clone();
        let generics = dependency.generics.clone();
        let mut imported = IndexSet::new();

        self.import_dependency_symbol_tree(
            module,
            symbol,
            &bindings,
            &types,
            &statics,
            &generics,
            &mut imported,
        )?;

        Ok(())
    }

    /// Import one checked symbol and its owned members.
    fn import_dependency_symbol_tree(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        bindings: &dir::BindingTable<'_>,
        types: &dir::TypeTable<'_>,
        statics: &dir::StaticTable<'_>,
        generics: &dir::GenericTable<'_>,
        imported: &mut IndexSet<dir::GlobalSymbolId>,
    ) -> CompilerResult<()> {
        if !imported.insert(symbol) {
            return Ok(());
        }

        let kind = bindings.get_symbol(symbol.local_id).kind;
        self.imports_mut(module).symbol_kinds.insert(symbol, kind);
        self.import_symbol(module, symbol, symbol, types, statics, generics);

        let owned_symbols = Self::dependency_owned_symbols(bindings, symbol);
        for owned_symbol in owned_symbols {
            self.import_dependency_symbol_tree(
                module,
                owned_symbol,
                bindings,
                types,
                statics,
                generics,
                imported,
            )?;
        }

        Ok(())
    }

    /// Return symbols owned by one dependency symbol.
    fn dependency_owned_symbols(
        bindings: &dir::BindingTable<'_>,
        owner: dir::GlobalSymbolId,
    ) -> Vec<dir::GlobalSymbolId> {
        for scope_id in bindings.scope_ids() {
            let scope = bindings.get_scope_by_id(scope_id);
            if scope.owner != Some(owner.local_id) {
                continue;
            }

            let named = scope
                .named_symbols()
                .map(|(_, symbol)| symbol.into_global(owner.module_id));
            let anonymous = scope
                .anonymous_symbols()
                .map(|symbol| symbol.into_global(owner.module_id));

            return named.chain(anonymous).collect();
        }

        Vec::new()
    }

    /// Import all checked symbols from one namespace dependency module.
    pub(in crate::check) fn import_dependency_module_symbols(
        &mut self,
        module: ModuleId,
        dependency_module: ModuleId,
    ) -> CompilerResult<()> {
        if self.inputs.contains_key(&dependency_module) {
            return Ok(());
        }

        self.mark_dependency_module(module, dependency_module)?;
        let dependency = self.load_dependency_input(dependency_module)?;
        let types = dependency.types.clone();
        let statics = dependency.statics.clone();
        let generics = dependency.generics.clone();
        let symbols = dependency
            .bindings
            .symbol_ids()
            .map(|symbol| {
                let global = symbol.into_global(dependency_module);
                let kind = dependency.bindings.get_symbol(symbol).kind;

                (global, kind)
            })
            .collect::<Vec<_>>();

        // import every namespace-visible checked symbol value
        for (symbol, kind) in symbols {
            self.imports_mut(module).symbol_kinds.insert(symbol, kind);
            self.import_symbol(module, symbol, symbol, &types, &statics, &generics);
        }

        Ok(())
    }
}
