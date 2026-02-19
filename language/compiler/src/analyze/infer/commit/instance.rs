use crate::analyze::common::StaticSubstitutionEnvironment;
use crate::{AnalyzeResult, Compiler};
use destack_dir::{
    Expression, GlobalNodeIdAny, GlobalSymbolId, Instance, LocalInstanceId, LocalNodeId,
    LocalTypeId, NodeTree, NodeType, StaticArgument, StaticExpression, SymbolTable, SymbolType,
    Type, TypeTable,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};
use std::collections::{HashMap, HashSet};

/// The lookup result for one `(symbol, static_arguments)` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstanceMatch {
    /// An exact environment match exists.
    Exact(LocalInstanceId),
    /// At least one conflicting environment exists for the same key.
    Conflict,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Return true when one expression node is a reference-instantiation source.
    fn expression_is_reference_instance_source(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        matches!(
            tree.get(expression_id),
            Expression::LocalReference { .. }
                | Expression::ModuleReference { .. }
                | Expression::GlobalReference { .. }
                | Expression::TypeImport { .. }
                | Expression::Instantiation { .. }
        )
    }

    /// Collect candidate node and type pairs for reference-instance registration.
    fn collect_reference_instance_candidates(
        &self,
        module_id: ModuleId,
        tree: &NodeTree,
        types: &TypeTable,
    ) -> Vec<(GlobalNodeIdAny, LocalTypeId)> {
        let mut candidates = Vec::new();
        let mut seen_nodes = HashSet::new();

        for (node_id, ty_id) in types
            .iter_declared_type_ids()
            .chain(types.iter_inferred_type_ids())
        {
            // skip non local node facts
            if node_id.module_id != module_id {
                continue;
            }

            // skip non annotation and non reference-source expression nodes
            if node_id.local_id.ty == NodeType::Expression {
                let expression_id = node_id.local_id.into_typed::<Expression>();
                if !self.expression_is_reference_instance_source(tree, expression_id) {
                    continue;
                }
            } else if node_id.local_id.ty != NodeType::Annotation {
                continue;
            }

            // keep one candidate type per node
            if !seen_nodes.insert(node_id) {
                continue;
            }

            candidates.push((node_id, ty_id));
        }

        candidates
    }

    /// Whether a symbol is instantiable (i.e. can have an instance type).
    pub(crate) fn symbol_is_instantiable(&self, symbol: GlobalSymbolId) -> bool {
        matches!(
            symbol.ty(),
            SymbolType::Class
                | SymbolType::Struct
                | SymbolType::Interface
                | SymbolType::Enum
                | SymbolType::Extension
                | SymbolType::TypeAlias
                | SymbolType::Newtype
        )
    }

    /// Commit instance facts for reference types that carry static arguments.
    pub(crate) fn commit_reference_instances(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        let options = self.analyze_context_options_for_module(module.id);
        let candidates = self.collect_reference_instance_candidates(module.id, tree, types);
        for (node_id, ty_id) in candidates {
            let source_id = node_id.local_id;

            // skip non reference types
            let Some((symbol, static_arguments, _)) = self.unwrap_type_symbol(types, ty_id) else {
                continue;
            };

            // skip symbols that cannot be instantiated
            if !self.symbol_is_instantiable(symbol) {
                continue;
            }

            // resolve static arguments and commit one type-instantiation event
            if let Some(resolved) = self.resolve_type_reference_static_arguments(
                module,
                profile,
                source_id,
                symbol,
                static_arguments.as_deref(),
                false,
                &options,
                tree,
                symbols,
                types,
            )? && !resolved.is_empty()
            {
                let Some(environment) = self.instance_environment_for_symbol_arguments(
                    module, profile, symbol, resolved, 0, tree, symbols, types,
                ) else {
                    continue;
                };
                let _ = self.commit_instance_for_node_maybe(node_id, symbol, environment, types);
            }
        }

        Ok(())
    }

    /// Look up an existing instance id for one full canonical environment.
    fn query_instance_for_symbol_environment(
        &self,
        symbol_id: GlobalSymbolId,
        static_arguments: &[StaticArgument],
        parameter_symbols: &[GlobalSymbolId],
        inherited_arity: usize,
        types: &TypeTable,
    ) -> Option<InstanceMatch> {
        let mut saw_conflict = false;

        for (instance_id, instance) in types.iter_instances() {
            if instance.symbol_id != symbol_id {
                continue;
            }
            if instance.static_arguments != static_arguments {
                continue;
            }

            if instance.static_parameter_symbols == parameter_symbols
                && instance.inherited_static_argument_count == inherited_arity
            {
                return Some(InstanceMatch::Exact(instance_id));
            }

            saw_conflict = true;
        }

        if saw_conflict {
            Some(InstanceMatch::Conflict)
        } else {
            None
        }
    }

    /// Look up an existing instance id attached to a node.
    pub(crate) fn query_instance_for_node(
        &self,
        node_id: GlobalNodeIdAny,
        types: &TypeTable,
    ) -> Option<LocalInstanceId> {
        types.get_instance_for_node(node_id)
    }

    /// Look up non-empty instance arguments attached to a node for an optional symbol.
    pub(crate) fn query_instance_arguments_for_node(
        &self,
        node_id: GlobalNodeIdAny,
        symbol_id: Option<GlobalSymbolId>,
        types: &TypeTable,
    ) -> Option<Vec<StaticArgument>> {
        let instance_id = self.query_instance_for_node(node_id, types)?;
        let instance = types.get_instance(instance_id);

        if let Some(symbol_id) = symbol_id
            && instance.symbol_id != symbol_id
        {
            return None;
        }
        if instance.static_arguments.is_empty() {
            return None;
        }

        Some(instance.static_arguments.clone())
    }

    /// Look up non-empty instance symbol and arguments attached to a node.
    pub(crate) fn query_instance_symbol_arguments_for_node(
        &self,
        node_id: GlobalNodeIdAny,
        types: &TypeTable,
    ) -> Option<(GlobalSymbolId, Vec<StaticArgument>)> {
        let instance_id = self.query_instance_for_node(node_id, types)?;
        let instance = types.get_instance(instance_id);
        if instance.static_arguments.is_empty() {
            return None;
        }

        Some((instance.symbol_id, instance.static_arguments.clone()))
    }

    /// Infer base instance arguments for member resolution from inherited or extension context.
    pub(crate) fn infer_member_instance_base_arguments(
        &self,
        inherited_arguments: &[StaticArgument],
        extension_arguments: Option<&[StaticArgument]>,
    ) -> Vec<StaticArgument> {
        match extension_arguments {
            Some(arguments) => arguments.to_vec(),
            None => inherited_arguments.to_vec(),
        }
    }

    /// Normalize one static argument for canonical instance-key usage.
    fn canonicalize_instance_argument_for_key(&self, argument: &StaticArgument) -> StaticArgument {
        match argument {
            StaticArgument::Unevaluated { node } => StaticArgument::Unevaluated { node: *node },
            StaticArgument::Evaluated { value, .. } => StaticArgument::Evaluated {
                name: None,
                value: value.clone(),
            },
        }
    }

    /// Normalize static arguments for canonical instance-key usage.
    fn canonicalize_instance_arguments_for_key(
        &self,
        static_arguments: Vec<StaticArgument>,
    ) -> Vec<StaticArgument> {
        static_arguments
            .iter()
            .map(|argument| self.canonicalize_instance_argument_for_key(argument))
            .collect()
    }

    /// Canonicalize argument type ids for commit keying when possible.
    fn canonicalize_instance_argument_types_for_commit_key(
        &self,
        static_arguments: Vec<StaticArgument>,
        types: &mut TypeTable,
    ) -> Vec<StaticArgument> {
        static_arguments
            .into_iter()
            .map(|argument| match argument {
                StaticArgument::Evaluated {
                    value: StaticExpression::Type { ty },
                    ..
                } => {
                    let canonical_ty = match types.get_type(ty).clone() {
                        Type::TypeLiteral { value } => types.intern_literal_type(ty, value),
                        _ => ty,
                    };

                    StaticArgument::Evaluated {
                        name: None,
                        value: StaticExpression::Type { ty: canonical_ty },
                    }
                }
                other => other,
            })
            .collect()
    }

    /// Canonicalize one environment for instance commit keying.
    fn canonicalize_instance_environment_for_commit_key(
        &self,
        environment: StaticSubstitutionEnvironment,
        types: &mut TypeTable,
    ) -> Option<StaticSubstitutionEnvironment> {
        let (arguments, parameter_symbols, inherited_arity) = environment.into_parts();
        let arguments = self.canonicalize_instance_arguments_for_key(arguments);
        let arguments = self.canonicalize_instance_argument_types_for_commit_key(arguments, types);
        StaticSubstitutionEnvironment::from_optional_parameter_symbols(
            arguments,
            parameter_symbols,
            inherited_arity,
        )
    }

    /// Query static parameter symbols for one function signature type.
    pub(crate) fn query_signature_static_parameter_symbols(
        &self,
        signature_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Vec<GlobalSymbolId> {
        let Type::Function {
            static_parameters, ..
        } = types.get_type(signature_ty_id)
        else {
            return Vec::new();
        };

        static_parameters
            .iter()
            .filter_map(|parameter| match types.get_type(*parameter) {
                Type::Reference { symbol, .. } => Some(*symbol),
                _ => None,
            })
            .collect()
    }

    /// Query owner static parameter symbols for one member symbol.
    fn query_member_owner_static_parameter_symbols(
        &self,
        module: &Module,
        profile: ProfileId,
        member_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> Vec<GlobalSymbolId> {
        let Some(owner_symbol) =
            self.owner_symbol_for_member_symbol(module, profile, member_symbol, symbols)
        else {
            return Vec::new();
        };

        self.collect_static_parameter_symbols(module, owner_symbol, profile, tree, symbols, types)
            .unwrap_or_default()
    }

    /// Build one substitution environment for one symbol and one static-argument vector.
    pub(crate) fn instance_environment_for_symbol_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol_id: GlobalSymbolId,
        static_arguments: Vec<StaticArgument>,
        inherited_arity: usize,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> Option<StaticSubstitutionEnvironment> {
        let parameter_symbols = self
            .collect_static_parameter_symbols(module, symbol_id, profile, tree, symbols, types)
            .unwrap_or_default();

        StaticSubstitutionEnvironment::from_parameter_symbols(
            static_arguments,
            parameter_symbols,
            inherited_arity,
        )
    }

    /// Compose one member-instance substitution environment from owner and signature facts.
    pub(crate) fn compose_member_instance_environment(
        &self,
        module: &Module,
        profile: ProfileId,
        member_symbol: GlobalSymbolId,
        base_arguments: &[StaticArgument],
        bound_substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        resolved_arguments: &[StaticArgument],
        signature_parameter_symbols: &[GlobalSymbolId],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> Option<StaticSubstitutionEnvironment> {
        // normalize both argument sources before composition
        let base_arguments = base_arguments
            .iter()
            .map(|argument| self.canonicalize_instance_argument_for_key(argument))
            .collect::<Vec<_>>();
        let resolved_arguments = resolved_arguments
            .iter()
            .map(|argument| self.canonicalize_instance_argument_for_key(argument))
            .collect::<Vec<_>>();

        // resolve owner parameter order for inherited arguments
        let owner_parameter_symbols = self.query_member_owner_static_parameter_symbols(
            module,
            profile,
            member_symbol,
            tree,
            symbols,
            types,
        );
        let effective_signature_parameter_symbols = if signature_parameter_symbols.is_empty() {
            self.collect_static_parameter_symbols(
                module,
                member_symbol,
                profile,
                tree,
                symbols,
                types,
            )
            .map(|symbols| {
                symbols
                    .into_iter()
                    .filter(|symbol| !owner_parameter_symbols.contains(symbol))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
        } else {
            signature_parameter_symbols.to_vec()
        };

        // collect arguments and symbols in canonical owner-then-signature order
        let mut arguments = Vec::<StaticArgument>::new();
        let mut parameter_symbols = Vec::<GlobalSymbolId>::new();

        // map owner parameter slots from inherited base arguments or bound substitutions
        for (index, symbol) in owner_parameter_symbols.iter().enumerate() {
            let argument = base_arguments.get(index).cloned().or_else(|| {
                bound_substitutions
                    .get(symbol)
                    .copied()
                    .map(|ty| StaticArgument::value(StaticExpression::Type { ty }))
            });

            let argument = argument?;
            arguments.push(argument);
            parameter_symbols.push(*symbol);
        }

        // split base arguments into owner and member-signature segments
        let base_signature_arguments = if base_arguments.len() > owner_parameter_symbols.len() {
            &base_arguments[owner_parameter_symbols.len()..]
        } else {
            &[]
        };
        if resolved_arguments.len() > effective_signature_parameter_symbols.len() {
            return None;
        }
        if base_signature_arguments.len() > effective_signature_parameter_symbols.len() {
            return None;
        }

        // map signature slots from resolved signature arguments or bound substitutions
        for (index, symbol) in effective_signature_parameter_symbols.iter().enumerate() {
            if owner_parameter_symbols.contains(symbol) {
                continue;
            }

            let argument = resolved_arguments
                .get(index)
                .cloned()
                .or_else(|| base_signature_arguments.get(index).cloned())
                .or_else(|| {
                    bound_substitutions
                        .get(symbol)
                        .copied()
                        .map(|ty| StaticArgument::value(StaticExpression::Type { ty }))
                });

            let argument = argument?;
            arguments.push(argument);
            parameter_symbols.push(*symbol);
        }

        let inherited_arity = owner_parameter_symbols.len();
        StaticSubstitutionEnvironment::from_parameter_symbols(
            arguments,
            parameter_symbols,
            inherited_arity,
        )
    }

    /// Commit one instance fact for one symbol and one substitution environment.
    pub(crate) fn commit_instance_for_symbol_environment(
        &self,
        symbol_id: GlobalSymbolId,
        environment: StaticSubstitutionEnvironment,
        types: &mut TypeTable,
    ) -> Option<LocalInstanceId> {
        let environment =
            self.canonicalize_instance_environment_for_commit_key(environment, types)?;
        let arguments = environment.arguments().to_vec();
        let parameter_symbols = environment.complete_parameter_symbols()?;
        let inherited_arity = environment.inherited_arity();

        match self.query_instance_for_symbol_environment(
            symbol_id,
            &arguments,
            &parameter_symbols,
            inherited_arity,
            types,
        ) {
            Some(InstanceMatch::Exact(existing)) => {
                return Some(existing);
            }
            Some(InstanceMatch::Conflict) => {
                return None;
            }
            None => {}
        }

        let instance =
            Instance::with_environment(symbol_id, arguments, parameter_symbols, inherited_arity)?;
        Some(types.insert_instance(instance))
    }

    /// Commit one instance fact for one symbol when arguments are non-empty.
    pub(crate) fn commit_instance_for_symbol_maybe(
        &self,
        symbol_id: GlobalSymbolId,
        environment: StaticSubstitutionEnvironment,
        types: &mut TypeTable,
    ) -> Option<LocalInstanceId> {
        if environment.arguments().is_empty() {
            return None;
        }
        if !environment.is_committable() {
            return None;
        }

        self.commit_instance_for_symbol_environment(symbol_id, environment, types)
    }

    /// Commit one instance fact for one node.
    pub(crate) fn commit_instance_for_node(
        &self,
        node_id: GlobalNodeIdAny,
        symbol_id: GlobalSymbolId,
        environment: StaticSubstitutionEnvironment,
        types: &mut TypeTable,
    ) -> Option<LocalInstanceId> {
        let instance_id =
            self.commit_instance_for_symbol_environment(symbol_id, environment, types)?;
        types.set_instance_for_node(node_id, instance_id);
        Some(instance_id)
    }

    /// Commit one instance fact for one node when arguments are non-empty.
    pub(crate) fn commit_instance_for_node_maybe(
        &self,
        node_id: GlobalNodeIdAny,
        symbol_id: GlobalSymbolId,
        environment: StaticSubstitutionEnvironment,
        types: &mut TypeTable,
    ) -> Option<LocalInstanceId> {
        if environment.arguments().is_empty() {
            return None;
        }
        if !environment.is_committable() {
            return None;
        }

        self.commit_instance_for_node(node_id, symbol_id, environment, types)
    }

    /// Commit one instance fact for one node from raw arguments in module context.
    pub(crate) fn commit_instance_for_node_arguments_maybe_in_module(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: GlobalNodeIdAny,
        symbol_id: GlobalSymbolId,
        static_arguments: Vec<StaticArgument>,
        inherited_arity: usize,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalInstanceId> {
        let environment = self.instance_environment_for_symbol_arguments(
            module,
            profile,
            symbol_id,
            static_arguments,
            inherited_arity,
            tree,
            symbols,
            types,
        )?;
        self.commit_instance_for_node_maybe(node_id, symbol_id, environment, types)
    }

    /// Collect instance arguments recorded on a member expression.
    pub(crate) fn query_member_instance_arguments_for_call(
        &self,
        module: &Module,
        member_expression_id: LocalNodeId<Expression>,
        member_symbol: Option<GlobalSymbolId>,
        types: &TypeTable,
    ) -> Option<Vec<StaticArgument>> {
        self.query_instance_arguments_for_node(
            member_expression_id.into_global_any(module.id),
            member_symbol,
            types,
        )
    }
}
