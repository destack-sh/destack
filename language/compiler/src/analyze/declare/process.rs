use crate::analyze::common::{AnalyzeIndex, TypeContext};
use crate::timing::tags;
use crate::{
    AnalyzeError, AnalyzeResult, Compiler, CompilerContext, ModuleCheckOptions,
    RequirementCollector, RequirementError,
};
use destack_artifact::{ArtifactKey, DirResolved};
use destack_builtin::BuiltinLibraryKind;
use destack_dir::{
    CaptureTable, Declaration, GlobalSymbolId, LocalSymbolId, LocalTypeId, Member, NodeType,
    SymbolTable, SymbolType, Type, TypeTable,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ModuleSource, ProfileId, Revision};
use std::collections::{HashMap, HashSet, VecDeque};

impl Compiler {
    /// Ensure declared DIR exists for a module.
    pub fn require_dir_declared(
        &self,
        revision: Revision,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), RequirementError> {
        // skip ambient builtin declarations when libs are disabled
        let module_ref = self.cache_module_snapshot(revision, module).ok();
        let Some(module_ref) = module_ref else {
            return Ok(());
        };
        let module_ref = module_ref.as_ref();
        if !self.options.load_libraries
            && matches!(
                module_ref.source,
                ModuleSource::Builtin(BuiltinLibraryKind::Library)
            )
        {
            return Ok(());
        }

        // avoid self dependency while declaring one module
        if self.current_artifact_key() == Some(ArtifactKey::dir_declared(module, profile)) {
            return Ok(());
        }

        self.require_artifact(revision, ArtifactKey::dir_declared(module, profile))
    }

    /// Phase 1: Evaluate declarations.
    pub(crate) fn analyze_module_declare(
        &self,
        resolved: &DirResolved,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
        captures: &mut CaptureTable,
        module_id: ModuleId,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> AnalyzeResult<()> {
        let _timing = self.timing_scope(tags::ANALYZE_MODULE_DECLARE);

        if !context.is_code_module(module_id) {
            return Ok(());
        }

        // load module state and resolved directory inputs
        let module = context.module(module_id);

        // skip analysis when module language is disabled
        if !self.module_language_allowed_in_context(context, module_id) {
            return Ok(());
        }

        // ensure builtins are resolved before declaring symbols
        self.require_language_environment(context.revision(), profile)
            .map_err(AnalyzeError::from)?;

        // ensure ambient libs are declared before user modules
        self.ensure_ambient_libs_declared(module.as_ref(), profile, context)?;

        // declaration inputs
        let tree = resolved.tree.as_ref();
        let roots = resolved.roots.as_ref();
        let mut collector = RequirementCollector::new();
        let module_checks = context.module_check_options_for_module(module.id);
        let options = context.analyze_context_options_for_module(module.id);
        let mut ctx = TypeContext::new(
            context,
            module.as_ref(),
            profile,
            &options,
            tree,
            symbols,
            types,
            AnalyzeIndex::default(),
        );

        {
            let _timing = self.timing_scope(tags::ANALYZE_DECLARE_DECLARATIONS);
            self.collect(
                &mut collector,
                self.collect_module_declarations(&mut ctx.reborrow(), roots.as_ref()),
            );

            // validate declaration implemented-contract conformance in declaration phase
            self.collect(
                &mut collector,
                self.check_declaration_contract_conformance(&mut ctx.reborrow()),
            );
        }

        // index declaration static parameter metadata
        self.index_static_parameter_metadata(&mut ctx.reborrow());

        // yield after declaration metadata writes
        if let Some(requirement) = collector.try_into_requirement() {
            return Err(AnalyzeError::Yield { requirement });
        }

        let mut collector = RequirementCollector::new();
        {
            let _timing = self.timing_scope(tags::ANALYZE_DECLARE_TYPES);

            // publish declare-owned static constant values before resolving types that depend on them
            self.collect(
                &mut collector,
                self.publish_declared_static_constant_values_for_module(&mut ctx.reborrow()),
            );
        }

        // yield after static constant publication
        if let Some(requirement) = collector.try_into_requirement() {
            return Err(AnalyzeError::Yield { requirement });
        }

        let mut collector = RequirementCollector::new();
        {
            let _timing = self.timing_scope(tags::ANALYZE_DECLARE_TYPES);

            // evaluate remaining unevaluated types after declaration metadata is available
            if self.should_eager_evaluate_declared_types(context, module.as_ref(), module_checks) {
                let has_dependency = self
                    .evaluate_unevaluated_types_to_fixpoint(&mut ctx.reborrow(), &mut collector);
                if has_dependency {
                    if let Some(requirement) = collector.try_into_requirement() {
                        return Err(AnalyzeError::Yield { requirement });
                    }

                    return Ok(());
                }
            }
        }

        // publish declared static parameter constraints
        let mut publish_collector = RequirementCollector::new();
        {
            self.collect(
                &mut publish_collector,
                self.record_artifact_static_parameter_constraints(&mut ctx),
            );
        }

        {
            let _timing = self.timing_scope(tags::ANALYZE_DECLARE_ALIASES);

            // publish all local alias targets from declared type metadata
            self.collect(
                &mut publish_collector,
                self.publish_declared_module_alias_targets(&mut ctx.reborrow()),
            );
        }

        // yield after static-constraint publication
        if let Some(requirement) = publish_collector.try_into_requirement() {
            return Err(AnalyzeError::Yield { requirement });
        }

        drop(ctx);

        // drop the read guard before taking a mutable lock for decorators
        {
            let _timing = self.timing_scope(tags::ANALYZE_DECLARE_DECORATORS);

            // attach well known decorator metadata to symbols
            self.collect(
                &mut collector,
                self.register_symbol_decorators(
                    module.as_ref(),
                    profile,
                    context,
                    tree,
                    symbols,
                    captures,
                ),
            );
        }

        // cache well-known intrinsics after decorator registration
        if module.is_user() {
            self.collect(
                &mut collector,
                self.resolve_intrinsic_environment(profile, context),
            );
        }
        // yield on any yields
        if let Some(requirement) = collector.try_into_requirement() {
            return Err(AnalyzeError::Yield { requirement });
        }

        // return the collected result
        Ok(())
    }

    /// Validate declaration implemented-contract conformance after declaration collection.
    fn check_declaration_contract_conformance(
        &self,
        ctx: &mut TypeContext<'_>,
    ) -> AnalyzeResult<()> {
        for (id, declaration) in ctx.tree.iter_nodes_of_type::<Declaration>() {
            let symbol = ctx.symbols.get_symbol(declaration.symbol());
            if !symbol.is_active() {
                continue;
            }

            match declaration {
                Declaration::Struct(declaration) => {
                    let declaration_symbol = declaration.symbol.into_global(ctx.module.id);
                    self.validate_declaration_contract_conformance(
                        &mut ctx.reborrow(),
                        id.into_any(),
                        declaration_symbol,
                        &declaration.implements_types,
                        false,
                    )?;
                }
                Declaration::Class(declaration) => {
                    let allows_deferred_associated = declaration.is_abstract;
                    let declaration_symbol = declaration.symbol.into_global(ctx.module.id);
                    self.validate_declaration_contract_conformance(
                        &mut ctx.reborrow(),
                        id.into_any(),
                        declaration_symbol,
                        &declaration.implements_types,
                        allows_deferred_associated,
                    )?;
                }
                Declaration::Enum(declaration) => {
                    let declaration_symbol = declaration.symbol.into_global(ctx.module.id);
                    self.validate_declaration_contract_conformance(
                        &mut ctx.reborrow(),
                        id.into_any(),
                        declaration_symbol,
                        &declaration.implements_types,
                        false,
                    )?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Ensure ambient libs are declared before analyzing a user module.
    fn ensure_ambient_libs_declared(
        &self,
        module: &Module,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> AnalyzeResult<()> {
        if self.options.load_libraries && module.is_user() {
            // resolve library environment before declaring ambient modules
            self.require_library_environment(context.revision(), profile)
                .map_err(AnalyzeError::from)?;

            let mut collector = RequirementCollector::new();
            for lib_module_id in self.library_environment_modules(profile) {
                if lib_module_id == module.id {
                    continue;
                }
                if let Err(error) =
                    self.require_dir_declared(context.revision(), lib_module_id, profile)
                    && let Some(error) = collector.try_collect::<(), _>(Err(error))
                {
                    return Err(AnalyzeError::from(error));
                }
            }

            if let Some(requirement) = collector.try_into_requirement() {
                return Err(AnalyzeError::Yield { requirement });
            }
        }

        Ok(())
    }

    /// Decide whether declared types should be eagerly evaluated for a module.
    fn should_eager_evaluate_declared_types(
        &self,
        context: &CompilerContext<'_>,
        module: &Module,
        _module_checks: ModuleCheckOptions,
    ) -> bool {
        // intrinsic declarations still need lazy declared alias materialization
        if matches!(
            module.source,
            ModuleSource::Builtin(BuiltinLibraryKind::Intrinsic)
        ) {
            return false;
        }

        // eager declare evaluation only works when declaration collection did not defer types
        if module.is_builtin() {
            return true;
        }

        !self.should_defer_declaration_types(context, module)
    }

    /// Evaluate unevaluated types to a fixed point.
    ///
    /// Returns true if evaluation yielded dependencies.
    fn evaluate_unevaluated_types_to_fixpoint(
        &self,
        ctx: &mut TypeContext<'_>,
        collector: &mut RequirementCollector,
    ) -> bool {
        let mut skipped_alias_targets = HashSet::new();
        for symbol_id in 0..ctx.symbols.symbol_count() {
            let raw_symbol = LocalSymbolId::new(symbol_id).into_global(ctx.module.id);
            let symbol_entry = ctx.symbols.get_symbol(raw_symbol.local_id);
            if !matches!(symbol_entry.ty, SymbolType::TypeAlias | SymbolType::Newtype) {
                continue;
            }

            let typed_symbol = GlobalSymbolId::new(
                ctx.module.id,
                raw_symbol.local_id.with_type(symbol_entry.ty),
            );
            if let Some(type_id) = ctx
                .types
                .get_alias_target_type_id(typed_symbol)
                .or_else(|| ctx.types.get_alias_target_type_id(raw_symbol))
            {
                skipped_alias_targets.insert(type_id);
            }
        }

        // seed the worklist
        let mut pending: VecDeque<LocalTypeId> = (0..ctx.types.type_count())
            .map(LocalTypeId::new)
            .filter(|ty_id| {
                matches!(ctx.types.get_type(*ty_id), Type::Unevaluated(_))
                    && !skipped_alias_targets.contains(ty_id)
            })
            .collect();
        let mut did_change = true;

        while did_change && !pending.is_empty() {
            did_change = false;
            let mut next_pending = VecDeque::new();
            let type_count = ctx.types.type_count();

            // evaluate pending types
            while let Some(ty_id) = pending.pop_front() {
                if !matches!(ctx.types.get_type(ty_id), Type::Unevaluated(_)) {
                    continue;
                }

                match self.resolve_declared_type(&mut ctx.reborrow(), ty_id) {
                    Ok(()) => {}
                    Err(AnalyzeError::Yield { requirement }) => {
                        collector.try_collect::<(), _>(Err(AnalyzeError::Yield { requirement }));
                        next_pending.push_back(ty_id);
                        continue;
                    }
                    Err(AnalyzeError::UnsatisfiedRequirement { .. }) => {
                        next_pending.push_back(ty_id);
                        continue;
                    }
                    Err(error) => {
                        self.error(error);
                    }
                }

                if collector.has_requirements() {
                    return true;
                }

                if matches!(ctx.types.get_type(ty_id), Type::Unevaluated(_)) {
                    next_pending.push_back(ty_id);
                } else {
                    did_change = true;
                }
            }

            // add newly created unevaluated types
            let new_type_count = ctx.types.type_count();
            if new_type_count > type_count {
                for i in type_count..new_type_count {
                    let ty_id = LocalTypeId::new(i);
                    if matches!(ctx.types.get_type(ty_id), Type::Unevaluated(_))
                        && !skipped_alias_targets.contains(&ty_id)
                    {
                        next_pending.push_back(ty_id);
                    }
                }
            }

            pending = next_pending;
        }

        false
    }

    /// Publish alias targets for all local alias declarations in one pass.
    fn publish_declared_module_alias_targets(
        &self,
        ctx: &mut TypeContext<'_>,
    ) -> AnalyzeResult<()> {
        for symbol_id in 0..ctx.symbols.symbol_count() {
            let raw_symbol = LocalSymbolId::new(symbol_id).into_global(ctx.module.id);
            let symbol_entry = ctx.symbols.get_symbol(raw_symbol.local_id);
            if !matches!(symbol_entry.ty, SymbolType::TypeAlias | SymbolType::Newtype) {
                continue;
            }

            let typed_symbol = GlobalSymbolId::new(
                ctx.module.id,
                raw_symbol.local_id.with_type(symbol_entry.ty),
            );
            let expression_id = symbol_entry
                .primary_declaration
                .and_then(
                    |primary_declaration| match primary_declaration.local_id.ty {
                        NodeType::Declaration => {
                            let declaration_id =
                                primary_declaration.local_id.into_typed::<Declaration>();
                            let Declaration::Type(declaration) = ctx.tree.get(declaration_id)
                            else {
                                return None;
                            };

                            Some(declaration.value)
                        }
                        NodeType::Member => {
                            let member_id = primary_declaration.local_id.into_typed::<Member>();
                            let Member::AssociatedType { value, .. } = ctx.tree.get(member_id)
                            else {
                                return None;
                            };

                            *value
                        }
                        _ => None,
                    },
                );
            let has_declared_target = expression_id.is_some();
            if !has_declared_target
                && ctx.types.get_alias_target_type_id(typed_symbol).is_none()
                && ctx.types.get_alias_target_type_id(raw_symbol).is_none()
            {
                continue;
            }

            let alias_target_id = if let Some(alias_target_id) = ctx
                .types
                .get_alias_target_type_id(typed_symbol)
                .or_else(|| ctx.types.get_alias_target_type_id(raw_symbol))
            {
                alias_target_id
            } else if let Some(expression_id) = expression_id {
                let global_id = expression_id.into_global_any(ctx.module.id);
                let Some(alias_target_id) = ctx.types.get_declared_type_id(global_id) else {
                    continue;
                };
                alias_target_id
            } else {
                continue;
            };

            ctx.types
                .set_alias_target_type_id(typed_symbol, alias_target_id);

            if let Some(expression_id) = expression_id {
                self.report_declared_alias_cycle_if_any(
                    &mut ctx.reborrow(),
                    typed_symbol,
                    expression_id,
                    alias_target_id,
                )?;
            }

            if matches!(ctx.types.get_type(alias_target_id), Type::Unevaluated(_)) {
                continue;
            }

            let mut cache = HashMap::new();
            let materialized =
                self.materialize_static_arguments_in_type(ctx, alias_target_id, &mut cache);
            if materialized != alias_target_id {
                ctx.types
                    .set_alias_target_type_id(typed_symbol, materialized);
            }
        }

        Ok(())
    }
}
