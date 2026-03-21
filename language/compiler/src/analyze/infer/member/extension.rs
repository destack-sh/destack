use super::*;
use crate::analyze::common::{InferContext, ModuleSymbolView, TypeContext};
use destack_dir::LocalInstanceId;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Extend substitutions with owner-parameter slots derived from inherited arguments.
    pub(crate) fn extend_owner_substitutions_from_inherited(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        member_symbol: GlobalSymbolId,
        inherited_arguments: &[StaticArgument],
        substitutions: &mut HashMap<GlobalSymbolId, LocalTypeId>,
    ) {
        // skip when no inherited arguments are available
        if inherited_arguments.is_empty() {
            return;
        }

        // resolve owner static parameters for the member symbol
        let Some(owner_symbol) = self
            .query_owner_symbol_for_member_symbol(ctx.module_symbol_view(), member_symbol)
            .ok()
            .flatten()
        else {
            return;
        };
        let Some(owner_parameters) =
            self.collect_static_parameter_symbols(ctx.type_view(), owner_symbol)
        else {
            return;
        };
        if owner_parameters.is_empty() {
            return;
        }

        // bind inherited arguments into missing owner parameter substitutions by position
        for (parameter_symbol, argument) in owner_parameters.iter().zip(inherited_arguments.iter())
        {
            if substitutions.contains_key(parameter_symbol) {
                continue;
            }
            let argument_type_id =
                self.convert_static_argument_type(argument, source_id, ctx.types);
            substitutions.insert(*parameter_symbol, argument_type_id);
        }
    }

    /// Merge inherited and extension substitutions for member lookup.
    pub(crate) fn merge_member_substitutions(
        &self,
        inherited: &InheritedStaticArguments,
        extension_context: Option<&ExtensionMemberContext>,
    ) -> HashMap<GlobalSymbolId, LocalTypeId> {
        let mut substitutions = inherited.substitutions.clone();
        if let Some(context) = extension_context {
            for (symbol, ty_id) in &context.substitutions {
                substitutions.insert(*symbol, *ty_id);
            }
        }
        substitutions
    }

    /// Record instance arguments for a resolved member symbol.
    pub(crate) fn record_member_instance_for_arguments(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        member_symbol: GlobalSymbolId,
        inherited: &InheritedStaticArguments,
        extension_context: Option<&ExtensionMemberContext>,
        resolved_arguments: &[StaticArgument],
        signature_parameter_symbols: &[GlobalSymbolId],
    ) -> AnalyzeResult<Option<LocalInstanceId>> {
        let substitutions = self.merge_member_substitutions(inherited, extension_context);
        let base_instance_arguments = if let Some(context) = extension_context {
            context.arguments.clone()
        } else {
            inherited.arguments.clone()
        };
        let environment = self.compose_member_instance_environment(
            &ctx.reborrow(),
            member_symbol,
            &base_instance_arguments,
            &substitutions,
            resolved_arguments,
            signature_parameter_symbols,
        );
        let Some(environment) = environment else {
            return Ok(None);
        };

        self.record_node_provisional_instance(
            expression_id.into_global_any(ctx.module.id),
            member_symbol,
            environment,
            ctx.infer,
            ctx.types,
        )
    }

    /// Resolve extension arguments and substitutions for a member lookup.
    pub(crate) fn resolve_extension_member_context(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        member_symbol: GlobalSymbolId,
        inherited_arguments: &[StaticArgument],
    ) -> AnalyzeResult<Option<ExtensionMemberContext>> {
        // locate the extension symbol that owns the member
        let extension_symbol =
            self.extension_symbol_for_member(ctx.module_symbol_view(), member_symbol)?;
        let Some(extension_symbol) = extension_symbol else {
            return Ok(None);
        };

        // ensure extension instance types are available for parameter kind resolution
        if ctx.types.get_instance_type_id(extension_symbol).is_none()
            && extension_symbol.module_id != ctx.module.id
        {
            self.import_instance_type_for_symbol(
                &ctx.index,
                ctx.profile,
                source_id,
                extension_symbol,
                ctx.types,
            )?;
        }

        // resolve extension static parameter symbols
        let extension_parameters = self
            .collect_static_parameter_symbols(ctx.type_view(), extension_symbol)
            .unwrap_or_default();

        // skip argument resolution when the extension has no parameters
        if extension_parameters.is_empty() {
            return Ok(Some(ExtensionMemberContext {
                arguments: Vec::new(),
                substitutions: HashMap::new(),
            }));
        }

        // avoid defaulting unresolved extension parameters to unknown
        if inherited_arguments.is_empty() {
            return Ok(Some(ExtensionMemberContext {
                arguments: Vec::new(),
                substitutions: HashMap::new(),
            }));
        }

        // map inherited arguments to extension parameters using the target type argument order
        let mut positional_arguments = self.map_extension_inherited_arguments(
            ctx.module,
            ctx.tree,
            ctx.symbols,
            extension_symbol,
            &extension_parameters,
            inherited_arguments,
            ctx.profile,
        )?;
        if positional_arguments.is_empty() {
            positional_arguments = inherited_arguments.to_vec();
        }

        // normalize inherited arguments for positional mapping
        for argument in positional_arguments.iter_mut() {
            if let StaticArgument::Evaluated { name, .. } = argument {
                *name = None;
            }
        }

        // resolve arguments and defaults against extension parameters
        let resolved_arguments: Option<Vec<StaticArgument>> = self
            .resolve_type_reference_static_arguments(
                &mut ctx.reborrow(),
                source_id,
                extension_symbol,
                Some(positional_arguments.as_slice()),
                true,
            )?;
        let resolved_arguments = resolved_arguments.unwrap_or_default();

        // build substitutions for extension type parameters
        let substitutions = self.build_type_parameter_substitutions_for_symbol(
            &mut ctx.reborrow(),
            extension_symbol,
            source_id,
            &resolved_arguments,
        );

        Ok(Some(ExtensionMemberContext {
            arguments: resolved_arguments,
            substitutions,
        }))
    }

    /// Map receiver static arguments into extension parameter order.
    pub(crate) fn map_extension_inherited_arguments(
        &self,
        module: &Module,
        tree: &NodeTree,
        symbols: &SymbolTable,
        extension_symbol: GlobalSymbolId,
        extension_parameters: &[GlobalSymbolId],
        inherited_arguments: &[StaticArgument],
        profile: ProfileId,
    ) -> AnalyzeResult<Vec<StaticArgument>> {
        // skip mapping when no parameters are declared
        if extension_parameters.is_empty() {
            return Ok(Vec::new());
        }

        // resolve the target type argument mapping from the extension declaration
        let target_mapping = self.extension_target_argument_mapping(
            module,
            extension_symbol,
            extension_parameters,
            profile,
            tree,
            symbols,
        )?;
        let Some(target_mapping) = target_mapping else {
            return Ok(Vec::new());
        };

        // map receiver arguments into extension parameter order
        let mut reordered = vec![None; extension_parameters.len()];
        for (target_index, parameter_index) in target_mapping.into_iter().enumerate() {
            if parameter_index >= reordered.len() {
                return Ok(Vec::new());
            }
            if reordered[parameter_index].is_some() {
                return Ok(Vec::new());
            }

            reordered[parameter_index] = Some(target_index);
        }

        let mut mapped = Vec::with_capacity(reordered.len());
        for maybe_target_index in reordered {
            let Some(target_index) = maybe_target_index else {
                return Ok(Vec::new());
            };

            if let Some(argument) = inherited_arguments.get(target_index) {
                mapped.push(argument.clone());
            } else {
                return Ok(Vec::new());
            }
        }

        Ok(mapped)
    }

    /// Resolve the extension symbol that owns a member symbol.
    pub(crate) fn extension_symbol_for_member(
        &self,
        view: ModuleSymbolView<'_>,
        member_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // locate the scope owner for the member symbol
        self.with_module_symbols_or_local_for_artifact(
            view.module,
            view.profile,
            member_symbol.module_id,
            view.symbols,
            destack_workspace::ArtifactKey::dir_declared,
            |owner_module, owner_symbols| {
                let member_entry = owner_symbols.get_symbol(member_symbol.local_id);
                let scope = owner_symbols.get_scope_by_id(member_entry.scope.0);
                let owner_id = scope.owner_id?;
                let owner_entry = owner_symbols.get_symbol(owner_id);
                if owner_entry.ty != SymbolType::Extension {
                    return None;
                }

                let extension_id = owner_id.with_type(owner_entry.ty);
                Some(extension_id.into_global(owner_module.id))
            },
        )
        .map_err(AnalyzeError::from)
    }
}
