use crate::analyze::common::StaticSubstitutionEnvironment;
use crate::{AnalyzeResult, Compiler};
use destack_dir::{
    Expression, GlobalNodeIdAny, GlobalSymbolId, InferTable, Instance, InstanceCommitObligation,
    InstanceCommitObligationId, LocalInstanceId, LocalNodeId, LocalTypeId, NodeTree,
    StaticArgument, StaticExpression, SymbolTable, SymbolType, Type, TypeTable,
};
use destack_workspace::{Module, ProfileId};
use std::collections::HashMap;

use super::key::InstanceMatch;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Query whether a symbol is instantiable (i.e. can have an instance type).
    pub(crate) fn query_symbol_is_instantiable(&self, symbol: GlobalSymbolId) -> bool {
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

    /// Commit one node instance for one resolved reference type when arguments are present.
    pub(crate) fn commit_instance_for_reference_type_maybe(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: GlobalNodeIdAny,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalInstanceId>> {
        // skip non-instantiable symbols
        if !self.query_symbol_is_instantiable(symbol) {
            return Ok(None);
        }

        // skip references without static arguments
        let Some(static_arguments) = static_arguments else {
            return Ok(None);
        };
        if static_arguments.is_empty() {
            return Ok(None);
        }

        // compose the full environment in declaration order
        let Some(environment) = self.instance_environment_for_symbol_arguments(
            module,
            profile,
            symbol,
            static_arguments.to_vec(),
            0,
            tree,
            symbols,
            types,
        ) else {
            return Ok(None);
        };

        self.commit_instance_for_node_maybe(node_id, symbol, environment, infer, types)
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
    ) -> AnalyzeResult<LocalInstanceId> {
        let scope = "commit_instance_for_symbol_environment";
        let environment =
            self.normalize_instance_environment_for_commit(scope, symbol_id, environment)?;

        match self.query_instance_for_symbol_environment(
            symbol_id,
            &environment.arguments,
            &environment.parameter_symbols,
            environment.inherited_arity,
            types,
        ) {
            Some(InstanceMatch::Exact(existing)) => return Ok(existing),
            Some(InstanceMatch::Conflict) => {
                return Err(self.internal_analyze_error_for_scope(
                    scope,
                    format!(
                        "conflicting canonical instance environment for symbol {symbol_id:?}: arguments={:?}, parameters={:?}, inherited_arity={}",
                        environment.arguments,
                        environment.parameter_symbols,
                        environment.inherited_arity,
                    ),
                ));
            }
            None => {}
        }

        let arguments = environment.arguments;
        let parameter_symbols = environment.parameter_symbols;
        let inherited_arity = environment.inherited_arity;
        let instance =
            Instance::with_environment(symbol_id, arguments, parameter_symbols, inherited_arity)
                .map_err(|error| {
                    self.internal_analyze_error_for_scope(
                scope,
                format!(
                    "invalid committed instance environment for symbol {symbol_id:?}: {error:?}"
                ),
            )
                })?;
        Ok(types.insert_instance(instance))
    }

    /// Commit one instance fact for one symbol and return an obligation when needed.
    pub(crate) fn commit_instance_for_symbol_maybe_with_obligation(
        &self,
        symbol_id: GlobalSymbolId,
        environment: StaticSubstitutionEnvironment,
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<(Option<LocalInstanceId>, Option<InstanceCommitObligationId>)> {
        if environment.arguments().is_empty() {
            return Ok((None, None));
        }
        if !environment.is_committable() {
            return Ok((None, None));
        }

        let scope = "commit_instance_for_symbol_maybe_with_obligation";
        let environment =
            self.normalize_instance_environment_for_commit(scope, symbol_id, environment)?;

        match self.query_instance_for_symbol_environment(
            symbol_id,
            &environment.arguments,
            &environment.parameter_symbols,
            environment.inherited_arity,
            types,
        ) {
            Some(InstanceMatch::Exact(existing)) => Ok((Some(existing), None)),
            Some(InstanceMatch::Conflict) => Err(self.internal_analyze_error_for_scope(
                scope,
                format!(
                    "conflicting canonical instance environment for symbol {symbol_id:?}: arguments={:?}, parameters={:?}, inherited_arity={}",
                    environment.arguments,
                    environment.parameter_symbols,
                    environment.inherited_arity,
                ),
            )),
            None => {
                let obligation = InstanceCommitObligation {
                    symbol_id,
                    static_arguments: environment.arguments.clone(),
                    static_parameter_symbols: environment.parameter_symbols.clone(),
                    inherited_static_argument_count: environment.inherited_arity,
                };
                let obligation_id = infer.upsert_instance_commit_obligation(obligation);
                Ok((None, Some(obligation_id)))
            }
        }
    }

    /// Commit one instance fact for one node.
    pub(crate) fn commit_instance_for_node(
        &self,
        node_id: GlobalNodeIdAny,
        symbol_id: GlobalSymbolId,
        environment: StaticSubstitutionEnvironment,
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalInstanceId>> {
        let scope = "commit_instance_for_node";
        let environment =
            self.normalize_instance_environment_for_commit(scope, symbol_id, environment)?;

        match self.query_instance_for_symbol_environment(
            symbol_id,
            &environment.arguments,
            &environment.parameter_symbols,
            environment.inherited_arity,
            types,
        ) {
            Some(InstanceMatch::Exact(existing)) => {
                types.set_instance_for_node(node_id, existing);
                Ok(Some(existing))
            }
            Some(InstanceMatch::Conflict) => Err(self.internal_analyze_error_for_scope(
                scope,
                format!(
                    "conflicting canonical instance environment for symbol {symbol_id:?}: arguments={:?}, parameters={:?}, inherited_arity={}",
                    environment.arguments,
                    environment.parameter_symbols,
                    environment.inherited_arity,
                ),
            )),
            None => {
                let obligation = InstanceCommitObligation {
                    symbol_id,
                    static_arguments: environment.arguments.clone(),
                    static_parameter_symbols: environment.parameter_symbols.clone(),
                    inherited_static_argument_count: environment.inherited_arity,
                };
                let obligation_id = infer.upsert_instance_commit_obligation(obligation);
                infer.set_instance_commit_obligation_for_node(node_id, obligation_id);
                Ok(None)
            }
        }
    }

    /// Commit one instance fact for one node when arguments are non-empty.
    pub(crate) fn commit_instance_for_node_maybe(
        &self,
        node_id: GlobalNodeIdAny,
        symbol_id: GlobalSymbolId,
        environment: StaticSubstitutionEnvironment,
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalInstanceId>> {
        if environment.arguments().is_empty() {
            return Ok(None);
        }
        if !environment.is_committable() {
            return Ok(None);
        }

        self.commit_instance_for_node(node_id, symbol_id, environment, infer, types)
    }

    /// Collect instance arguments recorded on a member expression.
    pub(crate) fn query_member_instance_arguments_for_call(
        &self,
        module: &Module,
        member_expression_id: LocalNodeId<Expression>,
        member_symbol: Option<GlobalSymbolId>,
        infer: &InferTable,
        types: &TypeTable,
    ) -> Option<Vec<StaticArgument>> {
        self.query_instance_arguments_for_node_infer(
            member_expression_id.into_global_any(module.id),
            member_symbol,
            infer,
            types,
        )
    }
}
