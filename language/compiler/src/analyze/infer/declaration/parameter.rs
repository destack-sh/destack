use crate::analyze::common::AnalyzeDependencyStage;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Declaration, FunctionSignature, GlobalSymbolId, LocalNodeIdAny, LocalTypeId, Member, NodeTree,
    NodeType, Parameter, StaticArgument, StaticExpression, StaticParameter, StaticParameterKind,
    StaticProperty, SymbolTable, Timing, Type, TypeLiteral, TypeTable, VarianceModifier,
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
        let kind = self
            .with_module_tree_symbols_or_local_at_stage(
                module,
                profile,
                symbol.module_id,
                tree,
                symbols,
                AnalyzeDependencyStage::Declare,
                |_, tree, symbols| {
                    self.static_parameter_kind_for_symbol_in_module(symbol, tree, symbols)
                },
            )
            .unwrap_or(StaticParameterKind::Type);

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

    /// Resolve the static parameter variance inside a module tree.
    pub(crate) fn static_parameter_variance_for_symbol_in_module(
        &self,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<VarianceModifier> {
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let primary = symbol_entry.primary_declaration?;
        let Ok(parameter_id) = primary.local_id.try_into_typed::<Parameter>() else {
            return None;
        };
        let parameter = tree.get(parameter_id);
        parameter
            .modifiers()
            .and_then(|modifiers| modifiers.variance)
    }

    /// Resolve the static parameter variance for a symbol.
    pub(crate) fn static_parameter_variance_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<VarianceModifier> {
        // reuse cached variance when available
        if let Some(variance) = types.get_static_parameter_variance(symbol) {
            return variance;
        }

        // resolve from the owning module when needed
        let variance = self
            .with_module_tree_symbols_or_local_at_stage(
                module,
                profile,
                symbol.module_id,
                tree,
                symbols,
                AnalyzeDependencyStage::Declare,
                |_, tree, symbols| {
                    self.static_parameter_variance_for_symbol_in_module(symbol, tree, symbols)
                },
            )
            .ok()
            .flatten();

        // cache resolved variance
        types.set_static_parameter_variance(symbol, variance);
        variance
    }

    /// Resolve static parameter kind and variance with optional tree access.
    pub(crate) fn static_parameter_metadata_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        tree: Option<&NodeTree>,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> (Option<StaticParameterKind>, Option<VarianceModifier>) {
        // use local tree when available
        if let Some(tree) = tree {
            let kind = self
                .static_parameter_kind_for_symbol(module, profile, symbol, tree, symbols, types);
            let variance = self.static_parameter_variance_for_symbol(
                module, profile, symbol, tree, symbols, types,
            );

            return (Some(kind), variance);
        }

        // fall back to owner type metadata when tree is unavailable
        let (kind, variance) = self
            .with_module_types_or_local_at_stage(
                module,
                profile,
                symbol.module_id,
                types,
                AnalyzeDependencyStage::Declare,
                |_, owner_types| {
                    (
                        owner_types.get_static_parameter_kind(symbol),
                        owner_types.get_static_parameter_variance(symbol).flatten(),
                    )
                },
            )
            .unwrap_or((None, None));

        (kind, variance)
    }

    /// Index static parameter symbols and metadata declared in one module.
    pub(crate) fn index_static_parameter_metadata_for_module_declarations(
        &self,
        module: &Module,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) {
        for symbol_id in symbols.active_symbol_ids() {
            let symbol = symbol_id.into_global(module.id);
            let Some(parameters) =
                self.collect_static_parameter_symbols_in_module(module.id, symbol, tree, symbols)
            else {
                continue;
            };

            types.set_static_parameter_symbols(symbol, parameters.clone());
            for parameter_symbol in parameters {
                let kind = self.static_parameter_kind_for_symbol_in_module(
                    parameter_symbol,
                    tree,
                    symbols,
                );
                let variance = self.static_parameter_variance_for_symbol_in_module(
                    parameter_symbol,
                    tree,
                    symbols,
                );

                types.set_static_parameter_kind(parameter_symbol, kind);
                types.set_static_parameter_variance(parameter_symbol, variance);
            }
        }
    }

    /// Resolve the static parameter kind from a parameter node.
    fn static_parameter_kind_for_parameter(&self, parameter: &Parameter) -> StaticParameterKind {
        let timing = parameter.modifiers().and_then(|modifiers| modifiers.timing);
        if matches!(timing, Some(Timing::Comptime)) {
            StaticParameterKind::Value
        } else {
            StaticParameterKind::Type
        }
    }

    /// Build static parameter placeholders fo r a function signature.
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
        types: &TypeTable,
    ) -> Option<Vec<GlobalSymbolId>> {
        if let Some(cached) = self
            .with_module_types_or_local_at_stage(
                module,
                profile,
                symbol.module_id,
                types,
                AnalyzeDependencyStage::Declare,
                |_, owner_types| owner_types.get_static_parameter_symbols(symbol),
            )
            .ok()
            .flatten()
        {
            return Some(cached);
        }

        // follow imports until a declaration provides static parameters
        let mut current = symbol;
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(current) {
                break None;
            }

            if current.module_id == module.id {
                if let Some(cached) = types.get_static_parameter_symbols(current) {
                    break Some(cached);
                }

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

            let Some((parameters, next)) = self
                .with_module_tree_symbols_at_stage(
                    module,
                    profile,
                    current.module_id,
                    AnalyzeDependencyStage::Declare,
                    |owner_module, owner_tree, owner_symbols| {
                        let owner_types = owner_module.dir(profile).types.read();
                        if let Some(cached) = owner_types.get_static_parameter_symbols(current) {
                            return (Some(cached), None);
                        }

                        if let Some(parameters) = self.collect_static_parameter_symbols_in_module(
                            owner_module.id,
                            current,
                            owner_tree,
                            owner_symbols,
                        ) {
                            return (Some(parameters), None);
                        }

                        let symbol_entry = owner_symbols.get_symbol(current.local_id);
                        (
                            None,
                            symbol_entry.target_symbol.or(symbol_entry.canonical_symbol),
                        )
                    },
                )
                .ok()
            else {
                break None;
            };
            if let Some(parameters) = parameters {
                break Some(parameters);
            }

            let next = match next {
                Some(next) => next,
                None => break None,
            };
            current = next;
        }
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
        let parameters = match primary_declaration.local_id.ty {
            // declaration static parameters
            NodeType::Declaration => {
                let declaration_id = primary_declaration.local_id.into_typed::<Declaration>();
                let declaration = tree.get(declaration_id);
                match declaration {
                    Declaration::Type {
                        static_parameters, ..
                    } => static_parameters.as_ref(),
                    Declaration::Struct { generics, .. }
                    | Declaration::Class { generics, .. }
                    | Declaration::Enum { generics, .. }
                    | Declaration::Interface { generics, .. }
                    | Declaration::Extension { generics, .. }
                    | Declaration::Namespace { generics, .. } => {
                        generics.static_parameters.as_ref()
                    }
                    _ => None,
                }
            }
            // associated type static parameters
            NodeType::Member => {
                let member_id = primary_declaration.local_id.into_typed::<Member>();
                let member = tree.get(member_id);
                match member {
                    Member::Type {
                        static_parameters, ..
                    } => static_parameters.as_ref(),
                    Member::Method { signature, .. } => signature
                        .generics
                        .as_ref()
                        .and_then(|generics| generics.static_parameters.as_ref()),
                    _ => None,
                }
            }
            _ => return None,
        };

        let Some(parameters) = parameters else {
            return Some(Vec::new());
        };

        // map parameter nodes to global symbols
        let symbols = parameters
            .iter()
            .map(|parameter_id| {
                let symbol_id = tree.get(*parameter_id).symbol();
                let symbol_entry = symbols.get_symbol(symbol_id);
                GlobalSymbolId::new(module_id, symbol_id.with_type(symbol_entry.ty))
            })
            .collect();

        Some(symbols)
    }

    /// Resolve static parameter metadata for a symbol (in this or another module).
    pub(crate) fn resolve_static_parameter(
        &self,
        module: &Module,
        symbol_id: GlobalSymbolId,
        source_id: LocalNodeIdAny,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> StaticParameter {
        // prefer parameter metadata from the owning module
        let parameter = self
            .with_module_tree_symbols_or_local_at_stage(
                module,
                profile,
                symbol_id.module_id,
                tree,
                symbols,
                AnalyzeDependencyStage::Declare,
                |owner_module, owner_tree, owner_symbols| {
                    self.resolve_static_parameter_in_module(
                        owner_module,
                        profile,
                        symbol_id,
                        source_id,
                        owner_tree,
                        owner_symbols,
                        types,
                    )
                },
            )
            .ok()
            .flatten();

        // synthesize unknown metadata when parameter details are unavailable
        parameter.unwrap_or_else(|| {
            let kind = self
                .static_parameter_kind_for_symbol(module, profile, symbol_id, tree, symbols, types);
            self.synthesize_unknown_static_parameter(symbol_id, kind, source_id, types)
        })
    }

    /// Synthesize an unknown static parameter when metadata cannot be resolved.
    pub(crate) fn synthesize_unknown_static_parameter(
        &self,
        symbol: GlobalSymbolId,
        kind: StaticParameterKind,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> StaticParameter {
        let unknown_ty_id = self.synthesize_unknown_type_for_source(source_id, types);

        StaticParameter {
            symbol,
            name: None,
            declared_type_id: unknown_ty_id,
            default_expression: None,
            kind,
        }
    }

    /// Synthesize an unknown type id for one source node.
    fn synthesize_unknown_type_for_source(
        &self,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
            source_id,
        )
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
        let raw_evaluated = self.resolve_declared_type_expression_value(
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
        let resolved = self.resolve_declared_type_expression_value(
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
        // synthesize unknown when constraints are missing or unavailable

        // reuse cached constraints when available
        if let Some(cached) = types.get_static_parameter_constraint_type(symbol) {
            return Some(cached);
        }

        // avoid recursive constraint resolution
        if types.is_static_parameter_constraint_in_progress(symbol) {
            return Some(self.synthesize_unknown_type_for_source(source_id, types));
        }

        // mark constraint resolution as in progress
        types.mark_static_parameter_constraint_in_progress(symbol);

        // resolve local constraints only when the type table matches the module
        let resolved = if symbol.module_id == module.id && types.module_id == module.id {
            // read the local symbol entry
            let symbol_entry = symbols.get_symbol(symbol.local_id);
            if !symbol_entry.is_static_parameter() {
                self.error(AnalyzeError::InvalidStaticConstraint {
                    node: source_id.into_anchored(module.id, Some(profile)),
                });
                None
            } else if let Some(primary_declaration) = symbol_entry.primary_declaration {
                // read the declared constraint type or fall back to unknown
                let declared_type_id = types
                    .get_declared_type_id(primary_declaration)
                    .unwrap_or_else(|| self.synthesize_unknown_type_for_source(source_id, types));

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
                        Some(self.synthesize_unknown_type_for_source(source_id, types))
                    } else {
                        Some(declared_type_id)
                    }
                } else {
                    Some(declared_type_id)
                }
            } else {
                Some(self.synthesize_unknown_type_for_source(source_id, types))
            }
        } else {
            // resolve remote static parameter constraints by importing the declared type
            self.with_module_tree_symbols_at_stage(
                module,
                profile,
                symbol.module_id,
                AnalyzeDependencyStage::Declare,
                |owner_module, owner_tree, owner_symbols| {
                    // read the remote symbol
                    let owner_symbol = owner_symbols.get_symbol(symbol.local_id);
                    if !owner_symbol.is_static_parameter() {
                        self.error(AnalyzeError::InvalidStaticConstraint {
                            node: source_id.into_anchored(module.id, Some(profile)),
                        });
                        return None;
                    }

                    if let Some(primary_declaration) = owner_symbol.primary_declaration {
                        // read and import the declared constraint type
                        let mut owner_types = owner_module.dir(profile).types.write();
                        if let Some(remote_declared_type_id) =
                            owner_types.get_declared_type_id(primary_declaration)
                        {
                            let needs_evaluation = matches!(
                                owner_types.get_type(remote_declared_type_id),
                                Type::Unevaluated(_)
                            );
                            if needs_evaluation {
                                let _ = self.evaluate_static_parameter_constraint_type(
                                    owner_module,
                                    profile,
                                    remote_declared_type_id,
                                    owner_tree,
                                    owner_symbols,
                                    &mut owner_types,
                                );
                            }

                            let remote_declared_type =
                                owner_types.get_type(remote_declared_type_id);
                            if matches!(remote_declared_type, Type::Unevaluated(_)) {
                                Some(self.synthesize_unknown_type_for_source(source_id, types))
                            } else {
                                Some(self.import_type_from_remote_for_node(
                                    source_id,
                                    remote_declared_type,
                                    &owner_types,
                                    symbol,
                                    types,
                                ))
                            }
                        } else {
                            Some(self.synthesize_unknown_type_for_source(source_id, types))
                        }
                    } else {
                        Some(self.synthesize_unknown_type_for_source(source_id, types))
                    }
                },
            )
            .ok()
            .flatten()
            .or_else(|| Some(self.synthesize_unknown_type_for_source(source_id, types)))
        };

        // clear the in progress marker
        types.clear_static_parameter_constraint_in_progress(symbol);
        if let Some(resolved) = resolved {
            // cache the resolved constraint type id
            types.set_static_parameter_constraint_type(symbol, resolved);
        }

        resolved
    }

    /// Resolve static parameter metadata from a module.
    fn resolve_static_parameter_in_module(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol_id: GlobalSymbolId,
        source_id: LocalNodeIdAny,
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
                    let _ = self.resolve_declared_type(
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
                    source_id,
                    remote_declared_type,
                    &remote_types,
                    symbol_id,
                    types,
                )
            } else {
                self.synthesize_unknown_type_for_source(source_id, types)
            }
        };

        // resolve the static parameter kind from the parameter modifiers
        let kind = self.static_parameter_kind_for_parameter(parameter);

        // derive the parameter name for mapping
        let name = match parameter {
            Parameter::Named { name, .. } => Some(*name),
            Parameter::VariadicNamed { name, .. } => Some(*name),
            Parameter::Pattern { .. } | Parameter::VariadicPattern { .. } => None,
        };

        // resolve the default expression for the parameter
        let default_expression = match parameter {
            Parameter::Named { default, .. } => {
                default.map(|expression_id| expression_id.into_global(module.id))
            }
            Parameter::Pattern { default, .. } => {
                default.map(|expression_id| expression_id.into_global(module.id))
            }
            Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. } => None,
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
    pub(crate) fn collect_type_reference_symbols(
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
    pub(crate) fn collect_type_reference_symbols_in_static_argument(
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
    pub(crate) fn collect_type_reference_symbols_in_static_expression(
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
    pub(crate) fn collect_type_reference_symbols_in_static_property(
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
