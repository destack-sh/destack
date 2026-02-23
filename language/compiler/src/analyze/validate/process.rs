use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeResult, Compiler, TaskDependencyError};
use destack_dir::{Annotation, Declaration, Expression, Member, Parameter, Pattern};
use destack_source::{CacheKind, ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{ModuleDir, ModuleSource, ProfileId};

impl Compiler {
    /// Ensure a module has been validated after analysis.
    pub fn require_analyze_module_validate(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        use crate::AnalyzeTask;
        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleValidate { module, profile })
    }

    /// Ensure a module has been fully analyzed (including checks).
    pub fn require_analyze_module(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        use crate::AnalyzeTask;
        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleValidate { module, profile })
    }

    /// Final pass: run validation checks over committed semantics.
    pub(crate) fn analyze_module_validate(
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
        let _timing = self.timing_scope(tags::ANALYZE_MODULE_VALIDATE);

        // update signature for non-code modules and skip validation
        if !self.is_code_module(module_id) {
            self.update_module_signature(module_id, profile, module_version, profile_version)?;
            return Ok(());
        }

        // skip validation when module language is disabled
        if !self.module_language_allowed(module_id) {
            return Ok(());
        }

        // decide whether declaration modules should skip validation
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let should_skip_declaration_validation = if module.language_type.is_declaration() {
            let module_checks = self.module_check_options_for_module(module_id);
            module_checks.skip_lib_check || matches!(module.source, ModuleSource::Builtin(_))
        } else {
            false
        };
        let analyze_options = self.analyze_context_options_for_module(module_id);
        let should_check_untrusted_declarations = module.language_type.is_declaration()
            && !matches!(module.source, ModuleSource::Builtin(_))
            && analyze_options.no_untrusted_declarations;

        if should_skip_declaration_validation && !should_check_untrusted_declarations {
            self.update_module_signature(module_id, profile, module_version, profile_version)?;
            return Ok(());
        }

        // resolve cache handle
        let cache_handle =
            self.cache_handle_for_module(module_id, Some(profile), None, CacheKind::DirAnalyzed);

        // try to load analyzed DIR from cache
        if let Some(cache) = cache_handle.as_ref()
            && let Ok(Some(entry)) = cache.read_dir_analyzed()
        {
            self.ensure_module_profile_matches::<AnalyzeError>(
                module_id,
                module_version,
                profile,
                profile_version,
            )?;
            let dir = ModuleDir::from_data(entry.payload);
            let module = self.program.modules.get(module_id);
            let mut module = module.write();
            self.ensure_module_profile_matches_guard::<AnalyzeError>(
                &module,
                module_version,
                profile,
                profile_version,
            )?;
            module.set_dir(profile, dir);
            self.update_module_signature(module_id, profile, module_version, profile_version)?;
            tracing::trace!(?module_id, ?profile, "analyze.module.cache");
            return Ok(());
        }

        // ensure analyze dependencies are ready
        self.require_analyze_module_capture(module_id, profile)?;

        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let mut should_return_after_validation = false;
        {
            let tree = dir.tree.read();
            let symbols = dir.symbols.read();
            let mut types = dir.types.write();

            // reject untrusted declaration files when configured
            if should_check_untrusted_declarations {
                let node = dir
                    .anchor_node
                    .into_global(module.id)
                    .into_anchored(Some(profile));
                self.error(AnalyzeError::UntrustedDeclarationDisabled { node });
            }

            if should_skip_declaration_validation {
                should_return_after_validation = true;
            } else {
                // TODO #Performance: consolidate validation passes into a single tree walk
                // validate binding identifiers
                self.validate_binding_names(&module, profile, &tree, &symbols);

                // validate declarations
                for (id, declaration) in tree.iter_nodes_of_type::<Declaration>() {
                    let symbol = symbols.get_symbol(declaration.symbol());
                    if !symbol.is_active() {
                        continue;
                    }
                    self.validate_declaration(
                        &module,
                        profile,
                        &tree,
                        &symbols,
                        &mut types,
                        id,
                        declaration,
                    );
                }

                // validate parameters
                for (id, parameter) in tree.iter_nodes_of_type::<Parameter>() {
                    if !self.is_node_active(&tree, &symbols, id.into_any()) {
                        continue;
                    }
                    self.validate_parameter(
                        &module, profile, &tree, &symbols, &types, id, parameter,
                    );
                }

                // validate members
                for (id, member) in tree.iter_nodes_of_type::<Member>() {
                    let symbol = symbols.get_symbol(member.symbol());
                    if !symbol.is_active() {
                        continue;
                    }
                    self.validate_member(&module, profile, &tree, analyze_options, id, member);
                }

                // validate expressions
                for (id, expression) in tree.iter_nodes_of_type::<Expression>() {
                    if !self.is_node_active(&tree, &symbols, id.into_any()) {
                        continue;
                    }
                    self.validate_expression(
                        &module,
                        profile,
                        &tree,
                        &symbols,
                        &mut types,
                        analyze_options,
                        id,
                        expression,
                    );
                }

                // validate annotations
                for (id, annotation) in tree.iter_nodes_of_type::<Annotation>() {
                    if let Some(parent) = tree.get_parent(id.id)
                        && !self.is_node_active(&tree, &symbols, parent)
                    {
                        continue;
                    }
                    self.validate_annotation(&module, profile, &tree, id, annotation);
                }

                // validate patterns
                for (id, pattern) in tree.iter_nodes_of_type::<Pattern>() {
                    if !self.is_node_active(&tree, &symbols, id.into_any()) {
                        continue;
                    }
                    self.validate_pattern(&module, profile, &tree, pattern);
                }

                // validate option dependent checks
                self.validate_strict_checks(
                    &module,
                    profile,
                    &tree,
                    &symbols,
                    &types,
                    analyze_options,
                );
                self.validate_restriction_checks(&module, profile, &types, analyze_options);
            }
        }

        if should_return_after_validation {
            self.update_module_signature(module_id, profile, module_version, profile_version)?;
            return Ok(());
        }

        // update module signature after validation
        self.update_module_signature(module_id, profile, module_version, profile_version)?;

        // write analyzed DIR to cache
        if let Some(cache) = cache_handle.as_ref() {
            let payload = dir.to_data();
            if let Err(error) = cache.write_dir_analyzed(payload) {
                tracing::debug!(?module_id, ?profile, ?error, "analyze.module.cache.write");
            }
        }

        Ok(())
    }
}
