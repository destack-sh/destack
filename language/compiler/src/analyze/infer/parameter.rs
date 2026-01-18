use std::collections::HashSet;

use super::super::common::StaticParameterReferences;
use crate::{AnalyzeResult, Compiler};
use destack_dir::{
    Declaration, FunctionSignature, GlobalSymbolId, LocalNodeIdAny, LocalTypeId, NodeTree,
    Parameter, StaticArgument, StaticExpression, StaticParameter, StaticParameterKind,
    StaticProperty, SymbolTable, SymbolType, Type, TypeLiteral, TypeTable,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Determine whether static parameters for a symbol should be treated as type parameters.
    pub(super) fn static_parameters_are_type_only(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
    ) -> bool {
        // treat local symbols based on the current module language type
        if symbol.module_id == module.id {
            return module.language_type.is_declaration();
        }

        // fall back to the symbol module's language type
        let symbol_module = self.program.modules.get(symbol.module_id);
        let symbol_module = symbol_module.read();
        symbol_module.language_type.is_declaration()
    }

    /// Ensure static parameter kinds are inferred for a declaration symbol.
    pub(super) fn ensure_static_parameter_kinds_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // collect parameter symbols for the declaration
        let parameter_symbols =
            self.collect_static_parameter_symbols(module, symbol, profile, tree, symbols);
        let Some(parameter_symbols) = parameter_symbols else {
            return Ok(());
        };
        if parameter_symbols.is_empty() {
            return Ok(());
        }

        // skip when all kinds are already cached
        let mut needs_inference = false;
        for symbol_id in parameter_symbols.iter().copied() {
            if types.get_static_parameter_kind(symbol_id).is_none() {
                needs_inference = true;
                break;
            }
        }
        if !needs_inference {
            return Ok(());
        }

        // handle extension parameter kinds through target mappings
        if symbol.ty() == SymbolType::Extension
            && self.ensure_extension_static_parameter_kinds(
                module,
                profile,
                node_id,
                symbol,
                &parameter_symbols,
                tree,
                symbols,
                types,
            )?
        {
            return Ok(());
        }

        // prefer type-only parameters in declaration modules
        let force_type_parameters = self.static_parameters_are_type_only(module, symbol);

        // collect reference usage from the declaration
        let references = if force_type_parameters {
            StaticParameterReferences::default()
        } else {
            self.collect_static_parameter_reference_symbols(
                module,
                profile,
                node_id,
                symbol,
                &parameter_symbols,
                tree,
                symbols,
                types,
            )?
        };

        // infer and cache kinds for each parameter symbol
        for symbol_id in parameter_symbols.iter().copied() {
            // skip parameters with cached kinds
            if types.get_static_parameter_kind(symbol_id).is_some() {
                continue;
            }

            // collect static parameter metadata
            let static_parameter = self.collect_static_parameter(
                module,
                symbol_id,
                StaticParameterKind::Type,
                node_id,
                profile,
                tree,
                symbols,
                types,
            );

            // derive default kind hints from parameter defaults
            let default_kind_hint =
                if let Some(default_expression) = static_parameter.default_expression.as_ref() {
                    if self.static_default_expression_prefers_type(
                        module,
                        profile,
                        default_expression,
                        tree,
                        symbols,
                        types,
                    )? {
                        Some(StaticParameterKind::Type)
                    } else {
                        Some(StaticParameterKind::Value)
                    }
                } else {
                    None
                };

            // derive constraint kind hints from parameter bounds
            let constraint_kind_hint = if force_type_parameters {
                None
            } else {
                self.static_parameter_constraint_kind_hint(
                    module, profile, symbol_id, node_id, symbols, types,
                )
            };

            self.resolve_static_parameter_kind(
                types,
                symbol_id,
                force_type_parameters,
                &references,
                default_kind_hint,
                constraint_kind_hint,
            );
        }

        Ok(())
    }

    /// Ensure static parameter kinds are inferred for a function signature.
    pub(super) fn ensure_static_parameter_kinds_for_signature(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        static_parameter_symbols: &[GlobalSymbolId],
        referenced_symbols: &HashSet<GlobalSymbolId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // skip when there are no parameters
        if static_parameter_symbols.is_empty() {
            return Ok(());
        }

        // skip when all kinds are already cached
        let mut needs_inference = false;
        for symbol_id in static_parameter_symbols.iter().copied() {
            if types.get_static_parameter_kind(symbol_id).is_none() {
                needs_inference = true;
                break;
            }
        }
        if !needs_inference {
            return Ok(());
        }

        // prefer type-only parameters in declaration modules
        let force_type_parameters = static_parameter_symbols
            .iter()
            .any(|symbol_id| self.static_parameters_are_type_only(module, *symbol_id));

        // map referenced symbols into static parameter references
        let mut references = StaticParameterReferences::default();
        references
            .type_symbols
            .extend(referenced_symbols.iter().copied());

        // infer and cache kinds for each parameter symbol
        for symbol_id in static_parameter_symbols.iter().copied() {
            // skip parameters with cached kinds
            if types.get_static_parameter_kind(symbol_id).is_some() {
                continue;
            }

            // collect static parameter metadata
            let static_parameter = self.collect_static_parameter(
                module,
                symbol_id,
                StaticParameterKind::Type,
                node_id,
                profile,
                tree,
                symbols,
                types,
            );

            // derive default kind hints from parameter defaults
            let default_kind_hint =
                if let Some(default_expression) = static_parameter.default_expression.as_ref() {
                    if self.static_default_expression_prefers_type(
                        module,
                        profile,
                        default_expression,
                        tree,
                        symbols,
                        types,
                    )? {
                        Some(StaticParameterKind::Type)
                    } else {
                        Some(StaticParameterKind::Value)
                    }
                } else {
                    None
                };

            // derive constraint kind hints from parameter bounds
            let constraint_kind_hint = if force_type_parameters {
                None
            } else {
                self.static_parameter_constraint_kind_hint(
                    module, profile, symbol_id, node_id, symbols, types,
                )
            };

            self.resolve_static_parameter_kind(
                types,
                symbol_id,
                force_type_parameters,
                &references,
                default_kind_hint,
                constraint_kind_hint,
            );
        }

        Ok(())
    }

    /// Resolve and cache the kind of a static parameter symbol.
    pub(super) fn resolve_static_parameter_kind(
        &self,
        types: &mut TypeTable,
        symbol_id: GlobalSymbolId,
        force_type_parameters: bool,
        usage: &StaticParameterReferences,
        default_kind_hint: Option<StaticParameterKind>,
        constraint_kind_hint: Option<StaticParameterKind>,
    ) -> StaticParameterKind {
        // reuse cached kinds when available
        if let Some(kind) = types.get_static_parameter_kind(symbol_id) {
            return kind;
        }

        // avoid recursive kind inference
        if types.is_static_parameter_kind_in_progress(symbol_id) {
            return StaticParameterKind::Type;
        }

        // mark inference as in progress
        types.mark_static_parameter_kind_in_progress(symbol_id);

        // infer and cache the kind
        let kind = self.infer_static_parameter_kind(
            force_type_parameters,
            usage,
            symbol_id,
            default_kind_hint,
            constraint_kind_hint,
        );
        types.clear_static_parameter_kind_in_progress(symbol_id);
        types.set_static_parameter_kind(symbol_id, kind);

        kind
    }

    /// Infer the kind for a static parameter symbol.
    fn infer_static_parameter_kind(
        &self,
        force_type_parameters: bool,
        usage: &StaticParameterReferences,
        symbol_id: GlobalSymbolId,
        default_kind_hint: Option<StaticParameterKind>,
        constraint_kind_hint: Option<StaticParameterKind>,
    ) -> StaticParameterKind {
        // respect declaration forcing
        if force_type_parameters {
            return StaticParameterKind::Type;
        }

        // treat value-position references as value parameters
        if usage.value_symbols.contains(&symbol_id) {
            return StaticParameterKind::Value;
        }

        // treat defaults as their hinted kind
        if let Some(default_kind) = default_kind_hint {
            return default_kind;
        }

        // treat referenced parameters as type parameters
        if usage.type_symbols.contains(&symbol_id) {
            return StaticParameterKind::Type;
        }

        // treat constraints as a value hint when provided
        if let Some(constraint_kind) = constraint_kind_hint {
            return constraint_kind;
        }

        StaticParameterKind::Type
    }

    /// Infer a static parameter kind from its declared constraint.
    pub(super) fn static_parameter_constraint_kind_hint(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<StaticParameterKind> {
        // resolve the declared constraint type
        let constraint_id = self
            .static_parameter_constraint_type(module, profile, symbol, source_id, symbols, types)?;

        // evaluate unevaluated constraints when possible
        if matches!(types.get_type(constraint_id), Type::Unevaluated(_)) {
            let tree = module.dir(profile).tree.read();
            if self
                .evaluate_type(module, profile, constraint_id, &tree, symbols, types)
                .is_err()
            {
                return None;
            }
        }

        // treat scalar constraints as value hints
        match types.get_type(constraint_id) {
            Type::TypeLiteral { value } => match value {
                TypeLiteral::Unknown | TypeLiteral::Any | TypeLiteral::Infer => None,
                _ => Some(StaticParameterKind::Value),
            },
            _ => None,
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
    pub(super) fn collect_static_parameter_symbols(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<Vec<GlobalSymbolId>> {
        // select the module that owns the symbol
        if symbol.module_id == module.id {
            self.collect_static_parameter_symbols_in_module(module.id, symbol, tree, symbols)
        } else {
            let remote_module = self.program.modules.get(symbol.module_id);
            let remote_module = remote_module.read();
            let remote_tree = remote_module.dir(profile).tree.read();
            let remote_symbols = remote_module.dir(profile).symbols.read();
            self.collect_static_parameter_symbols_in_module(
                remote_module.id,
                symbol,
                &remote_tree,
                &remote_symbols,
            )
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
        kind: StaticParameterKind,
        fallback_source_id: LocalNodeIdAny,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> StaticParameter {
        let parameter = if symbol_id.module_id == module.id {
            self.collect_static_parameter_in_module(
                module,
                profile,
                symbol_id,
                kind,
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
                kind,
                fallback_source_id,
                &remote_tree,
                &remote_symbols,
                types,
            )
        };

        parameter.unwrap_or_else(|| {
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

        // resolve local static parameter constraints from declared types
        let resolved = if symbol.module_id == module.id {
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
                if matches!(types.get_type(declared_type_id), Type::Unevaluated(_)) {
                    let tree = module.dir(profile).tree.read();
                    if self
                        .evaluate_type(module, profile, declared_type_id, &tree, symbols, types)
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
            let remote_symbols = remote_dir.symbols.read();

            // read the remote symbol
            let remote_symbol = remote_symbols.get_symbol(symbol.local_id);
            if !remote_symbol.is_static_parameter() {
                None
            } else if let Some(primary_declaration) = remote_symbol.primary_declaration {
                // read and import the declared constraint type
                let remote_types = remote_dir.types.read();
                if let Some(remote_declared_type_id) =
                    remote_types.get_declared_type_id(primary_declaration)
                {
                    let remote_declared_type = remote_types.get_type(remote_declared_type_id);
                    // fall back when the declared type is unevaluated
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
        _profile: ProfileId,
        symbol_id: GlobalSymbolId,
        kind: StaticParameterKind,
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
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            types.insert_type_from_any(ty, fallback_source_id)
        };

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
            Type::Mutable { right, .. }
            | Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. }
            | Type::PointerOf { right, .. } => {
                self.collect_type_reference_symbols(*right, types, symbols, visited);
            }
            Type::ArraySized { element, .. } => {
                self.collect_type_reference_symbols(*element, types, symbols, visited);
            }
            Type::Array { element } => {
                if let Some(element) = element {
                    self.collect_type_reference_symbols(*element, types, symbols, visited);
                }
            }
            Type::Tuple { elements } => {
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
