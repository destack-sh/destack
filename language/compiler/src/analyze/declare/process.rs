use std::collections::{HashMap, VecDeque};

use indexmap::IndexMap;

use crate::analyze::common::{ModuleTreeView, TypeContext};
use crate::timing::tags;
use crate::{
    AnalyzeError, AnalyzeResult, BuildKey, BuildRequirementCollector, BuildRequirementError,
    Compiler, ModuleCheckOptions,
};
use destack_builtin::BuiltinLibKind;
use destack_dir::{
    Declaration, DeclarationAbstraction, Export, GlobalSymbolId, LocalTypeId, StaticKey,
    SymbolSpace, SymbolType, Type,
};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{ArtifactKey, Module, ModuleSource, ProfileId};

impl Compiler {
    /// Ensure declared DIR exists for a module.
    pub fn require_dir_declared(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), BuildRequirementError> {
        // avoid self dependency when already declaring this module
        if self.current_build_key()
            == Some(BuildKey::Artifact(ArtifactKey::DirDeclared {
                module,
                profile,
            }))
        {
            return Ok(());
        }

        // skip ambient builtin declarations when libs are disabled
        if !self.options.load_libs {
            let module = self.program.modules.get(module);
            let module = module.read();
            if matches!(module.source, ModuleSource::Builtin(BuiltinLibKind::Lib)) {
                return Ok(());
            }
        }

        self.require_build_key(BuildKey::Artifact(ArtifactKey::DirDeclared {
            module,
            profile,
        }))
    }

    /// Phase 1: Evaluate declarations.
    pub(crate) fn analyze_module_declare(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> AnalyzeResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<AnalyzeError>(
            module_id,
            module_version,
            profile,
            profile_version,
        )?;
        let _timing = self.timing_scope(tags::ANALYZE_MODULE_DECLARE);

        self.require_dir_resolved(module_id, profile)?;
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        // load module state and dir ctx
        let module = self.program.modules.get(module_id);
        let module = module.read();

        // skip analysis when module language is disabled
        if !self.module_language_allowed(module_id) {
            return Ok(());
        }

        // ensure builtins are resolved before declaring symbols
        self.require_language_environment(profile)
            .map_err(AnalyzeError::from)?;

        // ensure ambient libs are declared before user modules
        self.ensure_ambient_libs_declared(&module, profile)?;

        // snapshot the module dir ctx for analysis
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let mut types = dir.types.write();
        let symbols = dir.symbols.read();
        let mut collector = BuildRequirementCollector::new();
        let module_checks = self.module_check_options_for_module(module.id);
        let options = self.analyze_context_options_for_module(module.id);
        let mut ctx = TypeContext::new(&module, profile, &options, &tree, &symbols, &mut types);

        {
            let _timing = self.timing_scope(tags::ANALYZE_DECLARE_DECLARATIONS);
            self.collect(
                &mut collector,
                self.collect_module_declarations(&mut ctx.reborrow()),
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

        let mut collector = BuildRequirementCollector::new();
        {
            let _timing = self.timing_scope(tags::ANALYZE_DECLARE_TYPES);

            // evaluate remaining unevaluated types after declaration metadata is available
            if self.should_eager_evaluate_declared_types(&module, module_checks) {
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
        let mut publish_collector = BuildRequirementCollector::new();
        {
            self.collect(
                &mut publish_collector,
                self.record_artifact_static_parameter_constraints(&mut ctx),
            );
        }

        // publish declare-owned static constant values for cross-module static evaluation
        self.collect(
            &mut publish_collector,
            self.publish_declared_static_constant_values_for_module(&mut ctx.reborrow()),
        );

        {
            let _timing = self.timing_scope(tags::ANALYZE_DECLARE_ALIASES);

            // publish exported alias targets from declared type metadata
            let exported_symbols = dir.exported_symbols.read();
            let binding_exports = dir.module_binding_exports.read();
            self.collect(
                &mut publish_collector,
                self.publish_declared_alias_targets(&mut ctx.reborrow(), &exported_symbols),
            );
            for binding in binding_exports.values() {
                self.collect(
                    &mut publish_collector,
                    self.publish_declared_alias_targets(&mut ctx.reborrow(), &binding.exports),
                );
            }
        }

        // yield after static-constraint publication
        if let Some(requirement) = publish_collector.try_into_requirement() {
            return Err(AnalyzeError::Yield { requirement });
        }

        // drop the read guard before taking a mutable lock for decorators
        drop(symbols);

        {
            let _timing = self.timing_scope(tags::ANALYZE_DECLARE_DECORATORS);

            // attach well known decorator metadata to symbols
            let mut symbols = dir.symbols.write();
            let mut captures = dir.captures.write();
            self.collect(
                &mut collector,
                self.register_symbol_decorators(
                    ModuleTreeView::new(&module, profile, &tree),
                    &mut symbols,
                    &mut captures,
                ),
            );
            drop(symbols);
            drop(captures);
        }

        // cache well-known intrinsics after decorator registration
        if module.is_user() {
            self.collect(&mut collector, self.resolve_intrinsic_environment(profile));
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
                Declaration::Struct { heritage, .. } => {
                    let declaration_symbol = declaration.symbol().into_global(ctx.module.id);
                    self.validate_declaration_contract_conformance(
                        &mut ctx.reborrow(),
                        id.into_any(),
                        declaration_symbol,
                        heritage.implements_types.as_deref().unwrap_or(&[]),
                        false,
                    )?;
                }
                Declaration::Class {
                    descriptor,
                    heritage,
                    ..
                } => {
                    let allows_deferred_associated =
                        descriptor.abstraction == DeclarationAbstraction::Abstract;
                    let declaration_symbol = declaration.symbol().into_global(ctx.module.id);
                    self.validate_declaration_contract_conformance(
                        &mut ctx.reborrow(),
                        id.into_any(),
                        declaration_symbol,
                        heritage.implements_types.as_deref().unwrap_or(&[]),
                        allows_deferred_associated,
                    )?;
                }
                Declaration::Enum { heritage, .. } => {
                    let declaration_symbol = declaration.symbol().into_global(ctx.module.id);
                    self.validate_declaration_contract_conformance(
                        &mut ctx.reborrow(),
                        id.into_any(),
                        declaration_symbol,
                        heritage.implements_types.as_deref().unwrap_or(&[]),
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
    ) -> AnalyzeResult<()> {
        if self.options.load_libs && module.is_user() {
            // resolve lib environment before declaring ambient modules
            self.require_lib_environment(profile)
                .map_err(AnalyzeError::from)?;

            let mut collector = BuildRequirementCollector::new();
            for lib_module_id in self.lib_environment_modules(profile) {
                if lib_module_id == module.id {
                    continue;
                }
                if let Err(error) = self.require_dir_declared(lib_module_id, profile)
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
        _module: &Module,
        _module_checks: ModuleCheckOptions,
    ) -> bool {
        // declared type commitments back cross-module declared reads
        true
    }

    /// Evaluate unevaluated types to a fixed point.
    ///
    /// Returns true if evaluation yielded dependencies.
    fn evaluate_unevaluated_types_to_fixpoint(
        &self,
        ctx: &mut TypeContext<'_>,
        collector: &mut BuildRequirementCollector,
    ) -> bool {
        // seed the worklist
        let mut pending: VecDeque<LocalTypeId> = (0..ctx.types.type_count())
            .map(LocalTypeId::new)
            .filter(|ty_id| matches!(ctx.types.get_type(*ty_id), Type::Unevaluated(_)))
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

                self.collect(
                    collector,
                    self.resolve_declared_type(&mut ctx.reborrow(), ty_id),
                );

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
                    if matches!(ctx.types.get_type(ty_id), Type::Unevaluated(_)) {
                        next_pending.push_back(ty_id);
                    }
                }
            }

            pending = next_pending;
        }

        false
    }

    /// Publish exported alias targets from declared local type metadata.
    fn publish_declared_alias_targets(
        &self,
        ctx: &mut TypeContext<'_>,
        exports: &IndexMap<(SymbolSpace, StaticKey), Export>,
    ) -> AnalyzeResult<()> {
        for export in exports.values() {
            // skip non-type exports
            if export.space != SymbolSpace::Type {
                continue;
            }

            // skip unresolved or remote exports
            let Some(export_symbol) = export.target.resolved() else {
                continue;
            };
            if export_symbol.module_id != ctx.module.id {
                continue;
            }

            // skip non-alias symbols
            let symbol_entry = ctx.symbols.get_symbol(export_symbol.local_id);
            if !matches!(symbol_entry.ty, SymbolType::TypeAlias | SymbolType::Newtype) {
                continue;
            }

            // load the declared alias target and ensure it is evaluated
            let typed_symbol = GlobalSymbolId::new(
                ctx.module.id,
                export_symbol.local_id.with_type(symbol_entry.ty),
            );
            let Some(alias_target_id) = ctx.types.get_alias_target_type_id(typed_symbol) else {
                continue;
            };
            self.ensure_type_evaluated(&mut ctx.reborrow(), alias_target_id)?;

            // materialize static arguments before publishing
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
