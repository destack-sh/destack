use crate::{AnalyzeResult, Compiler};
use destack_dir::{
    Declaration, FunctionSignature, GlobalSymbolId, LocalNodeIdAny, LocalTypeId, NodeTree,
    Parameter, StaticArgument, StaticExpression, StaticParameter, StaticParameterKind,
    StaticProperty, SymbolTable, Type, TypeLiteral, TypeTable,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};
use std::collections::HashSet;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve the static parameter kind for a symbol.
    pub(crate) fn static_parameter_kind_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> StaticParameterKind {
        // reuse cached kinds when available
        if let Some(kind) = types.get_static_parameter_kind(symbol) {
            return kind;
        }

        // resolve from the owning module when needed
        let kind = if symbol.module_id == module.id {
            self.static_parameter_kind_for_symbol_in_module(symbol, tree, symbols)
        } else {
            let remote_module = self.program.modules.get(symbol.module_id);
            let remote_module = remote_module.read();
            let remote_tree = remote_module.dir(profile).tree.read();
            let remote_symbols = remote_module.dir(profile).symbols.read();
            self.static_parameter_kind_for_symbol_in_module(symbol, &remote_tree, &remote_symbols)
        };

        // cache resolved kinds
        types.set_static_parameter_kind(symbol, kind);
        kind
    }

    /// Resolve the static parameter kind inside a module tree.
    pub(crate) fn static_parameter_kind_for_symbol_in_module(
        &self,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> StaticParameterKind {
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let Some(primary) = symbol_entry.primary_declaration else {
            return StaticParameterKind::Type;
        };
        let Ok(parameter_id) = primary.local_id.try_into_typed::<Parameter>() else {
            return StaticParameterKind::Type;
        };
        let parameter = tree.get(parameter_id);
        self.static_parameter_kind_for_parameter(parameter)
    }

    /// Resolve the static parameter kind from a parameter node.
    fn static_parameter_kind_for_parameter(&self, parameter: &Parameter) -> StaticParameterKind {
        let timing = parameter.modifiers().and_then(|modifiers| modifiers.timing);
        if matches!(timing, Some(destack_dir::Timing::Comptime)) {
            StaticParameterKind::Value
        } else {
            StaticParameterKind::Type
        }
    }

    /// Build static parameter placeholders for a function signature.
    pub(crate) fn static_parameter_placeholders_for_signature(
        &self,
        module: &Module,
        signature: &FunctionSignature,
        tree: &NodeTree,
        types: &mut TypeTable,
    ) -> Vec<LocalTypeId> {
        // stop when the signature has no generics
        let Some(generics) = signature.generics.as_ref() else {
            return Vec::new();
        };

        // stop when the generics have no static parameters
        let Some(parameters) = generics.static_parameters.as_ref() else {
            return Vec::new();
        };

        // map static parameters to reference placeholders
        let mut placeholders = Vec::with_capacity(parameters.len());
        for parameter_id in parameters {
            let symbol = tree.get(*parameter_id).symbol().into_global(module.id);
            let ty = Type::Reference {
                symbol,
                static_arguments: None,
            };
            let type_id = types.insert_type_from(ty, *parameter_id);
            placeholders.push(type_id);
        }

        placeholders
    }

    /// Collect static parameter symbols for a declaration symbol.
    pub(crate) fn collect_static_parameter_symbols(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<Vec<GlobalSymbolId>> {
        let cache_key = (profile, symbol);
        if let Some(entry) = self.cached_static_parameter_symbols(cache_key.0, cache_key.1) {
            return entry;
        }

        // follow imports until a declaration provides static parameters
        let mut current = symbol;
        let mut visited = HashSet::new();
        let result = loop {
            if !visited.insert(current) {
                break None;
            }

            if current.module_id == module.id {
                if let Some(parameters) = self
                    .collect_static_parameter_symbols_in_module(module.id, current, tree, symbols)
                {
                    break Some(parameters);
                }

                let symbol_entry = symbols.get_symbol(current.local_id);
                let next = symbol_entry.target_symbol.or(symbol_entry.canonical_symbol);
                let next = match next {
                    Some(next) => next,
                    None => break None,
                };
                current = next;
                continue;
            }

            let remote_module = self.program.modules.get(current.module_id);
            let remote_module = remote_module.read();
            let remote_tree = remote_module.dir(profile).tree.read();
            let remote_symbols = remote_module.dir(profile).symbols.read();
            if let Some(parameters) = self.collect_static_parameter_symbols_in_module(
                remote_module.id,
                current,
                &remote_tree,
                &remote_symbols,
            ) {
                break Some(parameters);
            }

            let symbol_entry = remote_symbols.get_symbol(current.local_id);
            let next = symbol_entry.target_symbol.or(symbol_entry.canonical_symbol);
            let next = match next {
                Some(next) => next,
                None => break None,
            };
            current = next;
        };

        self.set_cached_static_parameter_symbols(cache_key.0, cache_key.1, result.clone());
        result
    }

    /// Collect static parameter symbols for a declaration in a module.
    fn collect_static_parameter_symbols_in_module(
        &self,
        module_id: ModuleId,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<Vec<GlobalSymbolId>> {
        // extract the declaration for the symbol
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let primary_declaration = symbol_entry.primary_declaration?;
        let declaration_id = match primary_declaration.try_into_local_typed::<Declaration>() {
            Ok(declaration_id) => declaration_id,
            Err(_) => return None,
        };
        let declaration = tree.get(declaration_id);

        // locate static parameters on the declaration shape
        let parameters = match declaration {
            Declaration::Type {
                static_parameters, ..
            } => static_parameters.as_ref(),
            Declaration::Struct { generics, .. }
            | Declaration::Class { generics, .. }
            | Declaration::Enum { generics, .. }
            | Declaration::Interface { generics, .. }
            | Declaration::Extension { generics, .. }
            | Declaration::Namespace { generics, .. } => generics.static_parameters.as_ref(),
            _ => None,
        };

        let Some(parameters) = parameters else {
            return Some(Vec::new());
        };

        // map parameter nodes to global symbols
        let symbols = parameters
            .iter()
            .map(|parameter_id| {
                let symbol = tree.get(*parameter_id).symbol();
                symbol.into_global(module_id)
            })
            .collect();

        Some(symbols)
    }

    /// Collect static parameter metadata for a symbol (in this or another module).
    pub(super) fn collect_static_parameter(
        &self,
        module: &Module,
        symbol_id: GlobalSymbolId,
        fallback_source_id: LocalNodeIdAny,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> StaticParameter {
        // prefer parameter metadata from the owning module
        let parameter = if symbol_id.module_id == module.id {
            self.collect_static_parameter_in_module(
                module,
                profile,
                symbol_id,
                fallback_source_id,
                tree,
                symbols,
                types,
            )
        } else {
            let remote_module = self.program.modules.get(symbol_id.module_id);
            let remote_module = remote_module.read();
            let remote_tree = remote_module.dir(profile).tree.read();
            let remote_symbols = remote_module.dir(profile).symbols.read();

            self.collect_static_parameter_in_module(
                &remote_module,
                profile,
                symbol_id,
                fallback_source_id,
                &remote_tree,
                &remote_symbols,
                types,
            )
        };

        // fall back when parameter metadata is unavailable
        parameter.unwrap_or_else(|| {
            let kind = self
                .static_parameter_kind_for_symbol(module, profile, symbol_id, tree, symbols, types);
            self.fallback_static_parameter(symbol_id, kind, fallback_source_id, types)
        })
    }

    /// Build a fallback static parameter when metadata cannot be resolved.
    pub(super) fn fallback_static_parameter(
        &self,
        symbol: GlobalSymbolId,
        kind: StaticParameterKind,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> StaticParameter {
        let unknown_ty_id = types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
            source_id,
        );

        StaticParameter {
            symbol,
            name: None,
            declared_type_id: unknown_ty_id,
            default_expression: None,
            kind,
        }
    }

    /// Evaluate a static parameter constraint while preserving symbolic bounds.
    fn evaluate_static_parameter_constraint_type(
        &self,
        module: &Module,
        profile: ProfileId,
        declared_type_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // skip when the declared type is already evaluated
        let expression_id = match types.get_type(declared_type_id) {
            Type::Unevaluated(expression_id) => *expression_id,
            _ => return Ok(()),
        };

        // evaluate once without resolving static arguments to keep symbolic structure
        let raw_evaluated = self.try_evaluate_expression_to_type_value(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
            false,
            true,
            false,
            false,
        )?;

        // stage the raw evaluation result in the declared type slot
        types.update_type(declared_type_id, raw_evaluated);

        // stop here when the bound still depends on static parameters
        let mut visited = HashSet::new();
        if self.type_contains_static_parameters(
            module,
            profile,
            declared_type_id,
            symbols,
            types,
            &mut visited,
        ) {
            return Ok(());
        }

        // otherwise resolve static arguments now that the constraint is independent
        let resolved = self.try_evaluate_expression_to_type_value(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
            false,
            true,
            true,
            false,
        )?;
        types.update_type(declared_type_id, resolved);

        Ok(())
    }

    /// Resolve a static parameter constraint into the local type table.
    pub(crate) fn static_parameter_constraint_type(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        // fallback to unknown when constraints are missing or unavailable
        let unknown_type = Type::TypeLiteral {
            value: TypeLiteral::Unknown,
        };

        // reuse cached constraints when available
        if let Some(cached) = types.get_static_parameter_constraint_type(symbol) {
            return Some(cached);
        }

        // avoid recursive constraint resolution
        if types.is_static_parameter_constraint_in_progress(symbol) {
            return Some(types.insert_type_from_any(unknown_type.clone(), source_id));
        }

        // mark constraint resolution as in progress
        types.mark_static_parameter_constraint_in_progress(symbol);

        // resolve local constraints only when the type table matches the module
        let resolved = if symbol.module_id == module.id && types.module_id == module.id {
            // read the local symbol entry
            let symbol_entry = symbols.get_symbol(symbol.local_id);
            if !symbol_entry.is_static_parameter() {
                None
            } else if let Some(primary_declaration) = symbol_entry.primary_declaration {
                // read the declared constraint type or fall back to unknown
                let declared_type_id = types
                    .get_declared_type_id(primary_declaration)
                    .unwrap_or_else(|| types.insert_type_from_any(unknown_type.clone(), source_id));

                // evaluate unevaluated constraint types on demand
                let needs_evaluation =
                    matches!(types.get_type(declared_type_id), Type::Unevaluated(_));
                if needs_evaluation {
                    let tree = module.dir(profile).tree.read();
                    if self
                        .evaluate_static_parameter_constraint_type(
                            module,
                            profile,
                            declared_type_id,
                            &tree,
                            symbols,
                            types,
                        )
                        .is_err()
                    {
                        Some(types.insert_type_from_any(unknown_type.clone(), source_id))
                    } else {
                        Some(declared_type_id)
                    }
                } else {
                    Some(declared_type_id)
                }
            } else {
                Some(types.insert_type_from_any(unknown_type.clone(), source_id))
            }
        } else {
            // resolve remote static parameter constraints by importing the declared type
            let remote_module = self.program.modules.get(symbol.module_id);
            let remote_module = remote_module.read();
            let remote_dir = remote_module.dir(profile);
            let remote_tree = remote_dir.tree.read();
            let remote_symbols = remote_dir.symbols.read();

            // read the remote symbol
            let remote_symbol = remote_symbols.get_symbol(symbol.local_id);
            if !remote_symbol.is_static_parameter() {
                None
            } else if let Some(primary_declaration) = remote_symbol.primary_declaration {
                // read and import the declared constraint type
                let mut remote_types = remote_dir.types.write();
                if let Some(remote_declared_type_id) =
                    remote_types.get_declared_type_id(primary_declaration)
                {
                    // evaluate unevaluated remote constraints without bound validation
                    let needs_evaluation = matches!(
                        remote_types.get_type(remote_declared_type_id),
                        Type::Unevaluated(_)
                    );
                    if needs_evaluation {
                        let _ = self.evaluate_static_parameter_constraint_type(
                            &remote_module,
                            profile,
                            remote_declared_type_id,
                            &remote_tree,
                            &remote_symbols,
                            &mut remote_types,
                        );
                    }

                    let remote_declared_type = remote_types.get_type(remote_declared_type_id);
                    if matches!(remote_declared_type, Type::Unevaluated(_)) {
                        Some(types.insert_type_from_any(unknown_type.clone(), source_id))
                    } else {
                        Some(self.import_type_from_remote_for_node(
                            source_id,
                            remote_declared_type,
                            &remote_types,
                            symbol,
                            types,
                        ))
                    }
                } else {
                    Some(types.insert_type_from_any(unknown_type.clone(), source_id))
                }
            } else {
                Some(types.insert_type_from_any(unknown_type.clone(), source_id))
            }
        };

        // clear the in progress marker
        types.clear_static_parameter_constraint_in_progress(symbol);
        if let Some(resolved) = resolved {
            // cache the resolved constraint type id
            types.set_static_parameter_constraint_type(symbol, resolved);
        }

        resolved
    }

    /// Collect static parameter metadata from a module.
    fn collect_static_parameter_in_module(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol_id: GlobalSymbolId,
        fallback_source_id: LocalNodeIdAny,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<StaticParameter> {
        // read the parameter declaration
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let primary_declaration = symbol.primary_declaration?;
        let parameter_id = primary_declaration
            .try_into_local_typed::<Parameter>()
            .ok()?;
        let parameter = tree.get(parameter_id);

        // resolve the declared type for the parameter
        let declared_type_id = if module.id == types.module_id {
            types
                .get_declared_type_id(primary_declaration)
                .unwrap_or_else(|| {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type_from_any(ty, parameter_id.into_any())
                })
        } else {
            // import the declared type for remote parameters when possible
            let remote_dir = module.dir(profile);
            let mut remote_types = remote_dir.types.write();
            let remote_declared_type_id = remote_types.get_declared_type_id(primary_declaration);

            if let Some(remote_declared_type_id) = remote_declared_type_id {
                if matches!(
                    remote_types.get_type(remote_declared_type_id),
                    Type::Unevaluated(_)
                ) {
                    let _ = self.evaluate_type(
                        module,
                        profile,
                        remote_declared_type_id,
                        tree,
                        symbols,
                        &mut remote_types,
                    );
                }

                let remote_declared_type = remote_types.get_type(remote_declared_type_id);
                self.import_type_from_remote_for_node(
                    fallback_source_id,
                    remote_declared_type,
                    &remote_types,
                    symbol_id,
                    types,
                )
            } else {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from_any(ty, fallback_source_id)
            }
        };

        // resolve the static parameter kind from the parameter modifiers
        let kind = self.static_parameter_kind_for_parameter(parameter);

        // derive the parameter name for mapping
        let name = match parameter {
            Parameter::Named { name, .. } => Some(*name),
            Parameter::Variadic { name, .. } => Some(*name),
            Parameter::Pattern { .. } => None,
        };

        // resolve the default expression for the parameter
        let default_expression = match parameter {
            Parameter::Named { default, .. } => {
                default.map(|expression_id| expression_id.into_global(module.id))
            }
            Parameter::Pattern { default, .. } => {
                default.map(|expression_id| expression_id.into_global(module.id))
            }
            Parameter::Variadic { .. } => None,
        };

        Some(StaticParameter {
            symbol: symbol_id,
            name,
            declared_type_id,
            default_expression,
            kind,
        })
    }

    /// Collect static parameter symbols referenced in a type.
    pub(super) fn collect_type_reference_symbols(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
        symbols: &mut HashSet<GlobalSymbolId>,
        visited: &mut HashSet<LocalTypeId>,
    ) {
        if !visited.insert(ty_id) {
            return;
        }
        match types.get_type(ty_id) {
            Type::TypeLiteral { .. } | Type::InferVar { .. } | Type::Error | Type::This => {}
            Type::Value { value } => {
                self.collect_type_reference_symbols(*value, types, symbols, visited);
            }
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                symbols.insert(*symbol);
                if let Some(static_arguments) = static_arguments {
                    for argument in static_arguments {
                        self.collect_type_reference_symbols_in_static_argument(
                            argument, types, symbols, visited,
                        );
                    }
                }
            }
            Type::Unevaluated(_) => {}
            Type::Unary { right, .. } => {
                self.collect_type_reference_symbols(*right, types, symbols, visited);
            }
            Type::Binary { left, right, .. } => {
                self.collect_type_reference_symbols(*left, types, symbols, visited);
                self.collect_type_reference_symbols(*right, types, symbols, visited);
            }
            Type::Conditional {
                distributive_symbol: _,
                left,
                right,
                then_type,
                else_type,
            } => {
                self.collect_type_reference_symbols(*left, types, symbols, visited);
                self.collect_type_reference_symbols(*right, types, symbols, visited);
                self.collect_type_reference_symbols(*then_type, types, symbols, visited);
                self.collect_type_reference_symbols(*else_type, types, symbols, visited);
            }
            Type::Mapped {
                parameter, value, ..
            } => {
                self.collect_type_reference_symbols(parameter.constraint, types, symbols, visited);
                if let Some(key_remap) = parameter.key_remap {
                    self.collect_type_reference_symbols(key_remap, types, symbols, visited);
                }
                self.collect_type_reference_symbols(*value, types, symbols, visited);
            }
            Type::Index { left, index } => {
                self.collect_type_reference_symbols(*left, types, symbols, visited);
                self.collect_type_reference_symbols(*index, types, symbols, visited);
            }
            Type::TemplateLiteral { spans, .. } => {
                for span in spans {
                    self.collect_type_reference_symbols(*span, types, symbols, visited);
                }
            }
            Type::Import { .. } => {}
            Type::Infer { constraint, .. } => {
                if let Some(constraint) = constraint {
                    self.collect_type_reference_symbols(*constraint, types, symbols, visited);
                }
            }
            Type::Predicate { target, .. } => {
                if let Some(target) = target {
                    self.collect_type_reference_symbols(*target, types, symbols, visited);
                }
            }
            Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. }
            | Type::PointerOf { right, .. } => {
                self.collect_type_reference_symbols(*right, types, symbols, visited);
            }
            Type::ArraySized { element, .. } => {
                self.collect_type_reference_symbols(*element, types, symbols, visited);
            }
            Type::Array { element, .. } => {
                if let Some(element) = element {
                    self.collect_type_reference_symbols(*element, types, symbols, visited);
                }
            }
            Type::Tuple { elements, .. } => {
                for element in elements {
                    self.collect_type_reference_symbols(element.ty, types, symbols, visited);
                }
            }
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                for field in fields {
                    self.collect_type_reference_symbols(field.ty, types, symbols, visited);
                }
                for signature in call_signatures {
                    self.collect_type_reference_symbols(*signature, types, symbols, visited);
                }
                for signature in construct_signatures {
                    self.collect_type_reference_symbols(*signature, types, symbols, visited);
                }
                for signature in index_signatures {
                    self.collect_type_reference_symbols(
                        signature.key_type,
                        types,
                        symbols,
                        visited,
                    );
                    self.collect_type_reference_symbols(
                        signature.value_type,
                        types,
                        symbols,
                        visited,
                    );
                }
            }
            Type::Function {
                this_parameter,
                dynamic_parameters,
                return_type,
                ..
            } => {
                if let Some(this_parameter) = this_parameter {
                    self.collect_type_reference_symbols(*this_parameter, types, symbols, visited);
                }
                for parameter in dynamic_parameters {
                    self.collect_type_reference_symbols(*parameter, types, symbols, visited);
                }
                if let Some(return_type) = return_type {
                    self.collect_type_reference_symbols(*return_type, types, symbols, visited);
                }
            }
            Type::Union { elements } | Type::Intersection { elements } => {
                for element in elements {
                    self.collect_type_reference_symbols(*element, types, symbols, visited);
                }
            }
        }
    }

    /// Collect static parameter symbols referenced in a static argument.
    pub(super) fn collect_type_reference_symbols_in_static_argument(
        &self,
        argument: &StaticArgument,
        types: &TypeTable,
        symbols: &mut HashSet<GlobalSymbolId>,
        visited: &mut HashSet<LocalTypeId>,
    ) {
        match argument {
            StaticArgument::Unevaluated { .. } => {}
            StaticArgument::Evaluated { value, .. } => {
                self.collect_type_reference_symbols_in_static_expression(
                    value, types, symbols, visited,
                );
            }
        }
    }

    /// Collect static parameter symbols referenced in a static expression.
    pub(super) fn collect_type_reference_symbols_in_static_expression(
        &self,
        expression: &StaticExpression,
        types: &TypeTable,
        symbols: &mut HashSet<GlobalSymbolId>,
        visited: &mut HashSet<LocalTypeId>,
    ) {
        match expression {
            StaticExpression::Unevaluated { .. }
            | StaticExpression::ScalarLiteral { .. }
            | StaticExpression::TypeLiteral { .. } => {}
            StaticExpression::Type { ty } => {
                self.collect_type_reference_symbols(*ty, types, symbols, visited);
            }
            StaticExpression::Declaration {
                static_arguments, ..
            } => {
                if let Some(static_arguments) = static_arguments {
                    for argument in static_arguments {
                        self.collect_type_reference_symbols_in_static_argument(
                            argument, types, symbols, visited,
                        );
                    }
                }
            }
            StaticExpression::RangeExpression { start, end, .. } => {
                self.collect_type_reference_symbols_in_static_expression(
                    start, types, symbols, visited,
                );
                self.collect_type_reference_symbols_in_static_expression(
                    end, types, symbols, visited,
                );
            }
            StaticExpression::ArrayExpression { elements }
            | StaticExpression::TupleExpression { elements } => {
                for element in elements {
                    self.collect_type_reference_symbols_in_static_expression(
                        element, types, symbols, visited,
                    );
                }
            }
            StaticExpression::ObjectExpression { properties } => {
                for property in properties {
                    self.collect_type_reference_symbols_in_static_property(
                        property, types, symbols, visited,
                    );
                }
            }
        }
    }

    /// Collect static parameter symbols referenced in a static property.
    pub(super) fn collect_type_reference_symbols_in_static_property(
        &self,
        property: &StaticProperty,
        types: &TypeTable,
        symbols: &mut HashSet<GlobalSymbolId>,
        visited: &mut HashSet<LocalTypeId>,
    ) {
        match property {
            StaticProperty::Unevaluated { .. } => {}
            StaticProperty::Field { value, default, .. } => {
                self.collect_type_reference_symbols_in_static_expression(
                    value, types, symbols, visited,
                );
                if let Some(default) = default {
                    self.collect_type_reference_symbols_in_static_expression(
                        default, types, symbols, visited,
                    );
                }
            }
            StaticProperty::Method { body, .. } => {
                self.collect_type_reference_symbols_in_static_expression(
                    body, types, symbols, visited,
                );
            }
        }
    }
}
