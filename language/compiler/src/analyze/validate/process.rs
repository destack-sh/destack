use std::sync::Arc;

use crate::analyze::common::TypeContext;
use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeResult, BuildKey, BuildRequirementError, Compiler};
use destack_dir::{Annotation, Declaration, Expression, Member, Parameter, Pattern};
use destack_source::{CacheKind, ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{ArtifactKey, ModuleDir, ModuleSource, ProfileId};

impl Compiler {
    /// Ensure analyzed DIR exists for a module.
    pub fn require_dir_analyzed(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), BuildRequirementError> {
        self.require_build_key(BuildKey::Artifact(ArtifactKey::DirAnalyzed {
            module,
            profile,
        }))
    }

    /// Final pass: run validation checks over committed semantics.
    pub(crate) fn analyze_module_validate(
        &self,
        dir: &mut ModuleDir,
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

        // skip validation for non-code modules
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        // skip validation when module language is disabled
        if !self.module_language_allowed(module_id) {
            return Ok(());
        }

        // decide whether declaration modules should skip validation
        let module = self.program.modules.get(module_id);
        let module = module.as_ref();
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
            return Ok(());
        }

        // resolve cache handle for the final analyzed artifact write
        let cache_handle =
            self.cache_handle_for_module(module_id, Some(profile), None, CacheKind::DirAnalyzed);

        let module = self.program.modules.get(module_id);
        let module = module.as_ref();
        let mut should_return_after_validation = false;
        {
            let ModuleDir {
                tree,
                symbols,
                types,
                ..
            } = dir;
            let tree = tree.as_ref();
            let symbols = symbols.as_ref();

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
                let types = Arc::make_mut(types);
                let mut ctx =
                    TypeContext::new(&module, profile, &analyze_options, tree, symbols, types);

                // NOTE #Performance: validation still runs as multiple passes over the tree
                // validate binding identifiers
                self.validate_binding_names(&ctx);

                // validate declarations
                for (id, declaration) in ctx.tree.iter_nodes_of_type::<Declaration>() {
                    let symbol = ctx.symbols.get_symbol(declaration.symbol());
                    if !symbol.is_active() {
                        continue;
                    }
                    self.validate_declaration(&mut ctx.reborrow(), id, declaration);
                }

                // validate parameters
                for (id, parameter) in ctx.tree.iter_nodes_of_type::<Parameter>() {
                    if !self.is_node_active(ctx.tree, ctx.symbols, id.into_any()) {
                        continue;
                    }
                    self.validate_parameter(&mut ctx.reborrow(), id, parameter);
                }

                // validate members
                for (id, member) in ctx.tree.iter_nodes_of_type::<Member>() {
                    let symbol = ctx.symbols.get_symbol(member.symbol());
                    if !symbol.is_active() {
                        continue;
                    }
                    self.validate_member(&ctx, analyze_options, id, member);
                }

                // validate expressions
                for (id, expression) in ctx.tree.iter_nodes_of_type::<Expression>() {
                    if !self.is_node_active(ctx.tree, ctx.symbols, id.into_any()) {
                        continue;
                    }
                    self.validate_expression(&mut ctx.reborrow(), analyze_options, id, expression);
                }

                // validate type-index access resolution with one shared ctx context
                for (id, expression) in ctx.tree.iter_nodes_of_type::<Expression>() {
                    if !self.is_node_active(ctx.tree, ctx.symbols, id.into_any()) {
                        continue;
                    }
                    if matches!(expression, Expression::TypeIndex { .. }) {
                        self.validate_type_index_access(&mut ctx.reborrow(), id);
                    }
                }

                // validate annotations
                for (id, annotation) in ctx.tree.iter_nodes_of_type::<Annotation>() {
                    if let Some(parent) = ctx.tree.get_parent(id.id)
                        && !self.is_node_active(ctx.tree, ctx.symbols, parent)
                    {
                        continue;
                    }
                    self.validate_annotation(&ctx, id, annotation);
                }

                // validate patterns
                for (id, pattern) in ctx.tree.iter_nodes_of_type::<Pattern>() {
                    if !self.is_node_active(ctx.tree, ctx.symbols, id.into_any()) {
                        continue;
                    }
                    self.validate_pattern(&mut ctx.reborrow(), pattern);
                }

                // validate option dependent checks
                self.validate_strict_checks(&mut ctx.reborrow(), analyze_options);
                self.validate_restriction_checks(&mut ctx.reborrow(), analyze_options);
            }
        }

        if should_return_after_validation {
            return Ok(());
        }

        // write analyzed DIR to cache
        if let Some(cache) = cache_handle.as_ref() {
            if let Err(error) = cache.write_dir_analyzed(dir.clone()) {
                tracing::debug!(?module_id, ?profile, ?error, "analyze.module.cache.write");
            }
        }

        Ok(())
    }
}
