use crate::analyze::StaticSubstitutionEnvironment;
use crate::analyze::common::InferContext;
use crate::{AnalyzeResult, Compiler};
use destack_dir::{
    GlobalNodeIdAny, GlobalSymbolId, InferTable, InstanceCommitObligation,
    InstanceCommitObligationId, LocalInstanceId, LocalTypeId, StaticArgument, StaticExpression,
    SymbolType, Type, TypeTable,
};
use std::collections::HashMap;

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

    /// Record one node instance for one resolved reference type when arguments are present.
    pub(crate) fn record_reference_provisional_instance(
        &self,
        ctx: &mut InferContext<'_>,
        node_id: GlobalNodeIdAny,
        symbol: GlobalSymbolId,
        generic_arguments: Option<&[StaticArgument]>,
    ) -> AnalyzeResult<Option<LocalInstanceId>> {
        // skip non-instantiable symbols
        if !self.query_symbol_is_instantiable(symbol) {
            return Ok(None);
        }

        // skip references without static arguments
        let Some(generic_arguments) = generic_arguments else {
            return Ok(None);
        };
        if generic_arguments.is_empty() {
            return Ok(None);
        }

        // compose the full environment in declaration order
        let Some(environment) = self.instance_environment_for_symbol_arguments(
            ctx,
            symbol,
            generic_arguments.to_vec(),
            0,
        ) else {
            return Ok(None);
        };

        self.record_node_provisional_instance(node_id, symbol, environment, ctx.infer, ctx.types)
    }

    /// Query static parameter symbols for one function signature type.
    pub(crate) fn query_signature_static_parameter_symbols(
        &self,
        signature_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Vec<GlobalSymbolId> {
        let Type::Function {
            generic_parameters, ..
        } = types.get_type(signature_ty_id)
        else {
            return Vec::new();
        };

        self.generic_parameter_symbols_for_type_ids(generic_parameters, types)
    }

    /// Query owner static parameter symbols for one member symbol.
    fn query_member_owner_static_parameter_symbols(
        &self,
        ctx: &InferContext<'_>,
        member_symbol: GlobalSymbolId,
    ) -> Vec<GlobalSymbolId> {
        let Some(owner_symbol) = self
            .query_owner_symbol_for_member_symbol(ctx.module_symbol_view(), member_symbol)
            .ok()
            .flatten()
        else {
            return Vec::new();
        };

        self.collect_static_parameter_symbols(ctx.type_view(), owner_symbol)
            .unwrap_or_default()
    }

    /// Build one substitution environment for one symbol and one static-argument vector.
    pub(crate) fn instance_environment_for_symbol_arguments(
        &self,
        ctx: &InferContext<'_>,
        symbol_id: GlobalSymbolId,
        generic_arguments: Vec<StaticArgument>,
        inherited_arity: usize,
    ) -> Option<StaticSubstitutionEnvironment> {
        let parameter_symbols = self
            .collect_static_parameter_symbols(ctx.type_view(), symbol_id)
            .unwrap_or_default();

        StaticSubstitutionEnvironment::from_parameter_symbols(
            generic_arguments,
            parameter_symbols,
            inherited_arity,
        )
    }

    /// Compose one member-instance substitution environment from owner and signature metadata.
    pub(crate) fn compose_member_instance_environment(
        &self,
        ctx: &InferContext<'_>,
        member_symbol: GlobalSymbolId,
        base_arguments: &[StaticArgument],
        bound_substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        resolved_arguments: &[StaticArgument],
        signature_parameter_symbols: &[GlobalSymbolId],
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
        let owner_parameter_symbols =
            self.query_member_owner_static_parameter_symbols(ctx, member_symbol);
        let effective_signature_parameter_symbols = if signature_parameter_symbols.is_empty() {
            self.collect_static_parameter_symbols(ctx.type_view(), member_symbol)
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

    /// Record one instance for one symbol and return an obligation when needed.
    pub(crate) fn record_symbol_provisional_instance_with_obligation(
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

        let environment = self.normalize_instance_environment_for_commit(symbol_id, environment)?;
        let existing_instance = self.query_instance_for_symbol_environment(
            symbol_id,
            &environment.arguments,
            &environment.parameter_symbols,
            environment.inherited_arity,
            types,
        )?;
        if let Some(existing_instance) = existing_instance {
            return Ok((Some(existing_instance), None));
        }

        let obligation = InstanceCommitObligation {
            symbol_id,
            generic_arguments: environment.arguments.clone(),
            generic_parameter_symbols: environment.parameter_symbols.clone(),
            inherited_static_argument_count: environment.inherited_arity,
        };
        let obligation_id = infer.upsert_instance_commit_obligation(obligation);
        Ok((None, Some(obligation_id)))
    }

    /// Record one instance for one node.
    pub(crate) fn record_provisional_instance_for_node(
        &self,
        node_id: GlobalNodeIdAny,
        symbol_id: GlobalSymbolId,
        environment: StaticSubstitutionEnvironment,
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalInstanceId>> {
        let environment = self.normalize_instance_environment_for_commit(symbol_id, environment)?;
        let existing_instance = self.query_instance_for_symbol_environment(
            symbol_id,
            &environment.arguments,
            &environment.parameter_symbols,
            environment.inherited_arity,
            types,
        )?;
        if let Some(existing_instance) = existing_instance {
            infer.set_provisional_instance_for_node(node_id, existing_instance);
            return Ok(Some(existing_instance));
        }

        let obligation = InstanceCommitObligation {
            symbol_id,
            generic_arguments: environment.arguments.clone(),
            generic_parameter_symbols: environment.parameter_symbols.clone(),
            inherited_static_argument_count: environment.inherited_arity,
        };
        let obligation_id = infer.upsert_instance_commit_obligation(obligation);
        infer.set_instance_commit_obligation_for_node(node_id, obligation_id);
        Ok(None)
    }

    /// Record one instance for one node when arguments are non-empty.
    pub(crate) fn record_node_provisional_instance(
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

        self.record_provisional_instance_for_node(node_id, symbol_id, environment, infer, types)
    }
}
