use super::*;
use destack_dir::{InferTable, LocalInstanceId};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Extend substitutions with owner-parameter slots derived from inherited arguments.
    pub(crate) fn extend_owner_substitutions_from_inherited(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        member_symbol: GlobalSymbolId,
        inherited_arguments: &[StaticArgument],
        substitutions: &mut HashMap<GlobalSymbolId, LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) {
        // skip when no inherited arguments are available
        if inherited_arguments.is_empty() {
            return;
        }

        // resolve owner static parameters for the member symbol
        let Some(owner_symbol) =
            self.owner_symbol_for_member_symbol(module, profile, member_symbol, symbols)
        else {
            return;
        };
        let Some(owner_parameters) = self.collect_static_parameter_symbols(
            module,
            owner_symbol,
            profile,
            tree,
            symbols,
            types,
        ) else {
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
            let argument_type_id = self.convert_static_argument_type(argument, source_id, types);
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
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        member_symbol: GlobalSymbolId,
        inherited: &InheritedStaticArguments,
        extension_context: Option<&ExtensionMemberContext>,
        resolved_arguments: &[StaticArgument],
        signature_parameter_symbols: &[GlobalSymbolId],
        tree: &NodeTree,
        symbols: &SymbolTable,
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalInstanceId>> {
        let substitutions = self.merge_member_substitutions(inherited, extension_context);
        let base_instance_arguments = if let Some(context) = extension_context {
            context.arguments.clone()
        } else {
            inherited.arguments.clone()
        };
        let environment = self.compose_member_instance_environment(
            module,
            profile,
            member_symbol,
            &base_instance_arguments,
            &substitutions,
            resolved_arguments,
            signature_parameter_symbols,
            tree,
            symbols,
            types,
        );
        let Some(environment) = environment else {
            return Ok(None);
        };

        self.record_node_provisional_instance(
            expression_id.into_global_any(module.id),
            member_symbol,
            environment,
            infer,
            types,
        )
    }

    /// Resolve extension arguments and substitutions for a member lookup.
    pub(crate) fn resolve_extension_member_context(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        member_symbol: GlobalSymbolId,
        inherited_arguments: &[StaticArgument],
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<ExtensionMemberContext>> {
        // locate the extension symbol that owns the member
        let extension_symbol =
            self.extension_symbol_for_member(module, profile, member_symbol, symbols)?;
        let Some(extension_symbol) = extension_symbol else {
            return Ok(None);
        };

        // ensure extension instance types are available for parameter kind resolution
        if types.get_instance_type_id(extension_symbol).is_none()
            && extension_symbol.module_id != module.id
        {
            self.import_instance_type_for_symbol(profile, source_id, extension_symbol, types)?;
        }

        // resolve extension static parameter symbols
        let extension_parameters = self
            .collect_static_parameter_symbols(
                module,
                extension_symbol,
                profile,
                tree,
                symbols,
                types,
            )
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
            extension_symbol,
            &extension_parameters,
            inherited_arguments,
            profile,
        );
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
                module,
                profile,
                source_id,
                extension_symbol,
                Some(&positional_arguments),
                true,
                options,
                tree,
                symbols,
                types,
            )?;
        let resolved_arguments = resolved_arguments.unwrap_or_default();

        // build substitutions for extension type parameters
        let substitutions = self.build_type_parameter_substitutions_for_symbol(
            module,
            profile,
            extension_symbol,
            source_id,
            &resolved_arguments,
            tree,
            symbols,
            types,
        );

        Ok(Some(ExtensionMemberContext {
            arguments: resolved_arguments,
            substitutions,
        }))
    }

    /// Map receiver static arguments into extension parameter order.
    pub(crate) fn map_extension_inherited_arguments(
        &self,
        extension_symbol: GlobalSymbolId,
        extension_parameters: &[GlobalSymbolId],
        inherited_arguments: &[StaticArgument],
        profile: ProfileId,
    ) -> Vec<StaticArgument> {
        // skip mapping when no parameters are declared
        if extension_parameters.is_empty() {
            return Vec::new();
        }

        // resolve the target type argument mapping from the extension declaration
        let target_mapping =
            self.extension_target_argument_mapping(extension_symbol, extension_parameters, profile);
        let Some(target_mapping) = target_mapping else {
            return Vec::new();
        };

        // map receiver arguments into extension parameter order
        let mut reordered = vec![None; extension_parameters.len()];
        for (target_index, parameter_index) in target_mapping.into_iter().enumerate() {
            if parameter_index >= reordered.len() {
                return Vec::new();
            }
            if reordered[parameter_index].is_some() {
                return Vec::new();
            }

            reordered[parameter_index] = Some(target_index);
        }

        let mut mapped = Vec::with_capacity(reordered.len());
        for maybe_target_index in reordered {
            let Some(target_index) = maybe_target_index else {
                return Vec::new();
            };

            if let Some(argument) = inherited_arguments.get(target_index) {
                mapped.push(argument.clone());
            } else {
                return Vec::new();
            }
        }

        mapped
    }

    /// Resolve the extension symbol that owns a member symbol.
    pub(crate) fn extension_symbol_for_member(
        &self,
        module: &Module,
        profile: ProfileId,
        member_symbol: GlobalSymbolId,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // locate the scope owner for the member symbol
        self.with_module_symbols_or_local_at_stage(
            module,
            profile,
            member_symbol.module_id,
            symbols,
            AnalyzeDependencyStage::Declare,
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
