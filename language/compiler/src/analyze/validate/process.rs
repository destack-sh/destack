use crate::analyze::common::{AnalyzeIndex, TypeContext};
use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeResult, Compiler, CompilerContext, RequirementError};
use destack_artifact::ArtifactKey;
use destack_dir::{
    Declaration, Decorator, Expression, LocalNodeIdAny, Member, NodeTree, Parameter, Pattern,
    SymbolTable, TypeExpression, TypeTable,
};
use destack_source::ModuleId;
use destack_workspace::{ModuleSource, ProfileId};

impl Compiler {
    /// Ensure analyzed DIR exists for a module.
    pub fn require_dir_analyzed(
        &self,
        revision: destack_workspace::Revision,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), RequirementError> {
        self.require_artifact(revision, ArtifactKey::dir_analyzed(module, profile))
    }

    /// Final pass: run validation checks over committed semantics.
    pub(crate) fn analyze_module_validate(
        &self,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        anchor_node: LocalNodeIdAny,
        module_id: ModuleId,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> AnalyzeResult<()> {
        let _timing = self.timing_scope(tags::ANALYZE_MODULE_VALIDATE);

        // skip validation for non-code modules
        if !context.is_code_module(module_id) {
            return Ok(());
        }

        // skip validation when module language is disabled
        if !self.module_language_allowed_in_context(context, module_id) {
            return Ok(());
        }

        // decide whether declaration modules should skip validation
        let module = context.module(module_id);
        let should_skip_declaration_validation = if module.language_type.is_declaration() {
            let module_checks = context.module_check_options_for_module(module_id);
            module_checks.skip_lib_check || matches!(module.source, ModuleSource::Builtin(_))
        } else {
            false
        };
        let analyze_options = context.analyze_context_options_for_module(module_id);
        let should_check_untrusted_declarations = module.language_type.is_declaration()
            && !matches!(module.source, ModuleSource::Builtin(_))
            && analyze_options.no_untrusted_declarations;

        if should_skip_declaration_validation && !should_check_untrusted_declarations {
            return Ok(());
        }

        let mut should_return_after_validation = false;
        // reject untrusted declaration files when configured
        if should_check_untrusted_declarations {
            let node = anchor_node
                .into_global(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::UntrustedDeclarationDisabled { node });
        }

        if should_skip_declaration_validation {
            should_return_after_validation = true;
        } else {
            let mut ctx = TypeContext::new(
                context,
                module.as_ref(),
                profile,
                &analyze_options,
                tree,
                symbols,
                types,
                AnalyzeIndex::default(),
            );

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

            // validate type-index access resolution with one shared type context
            for (id, expression) in ctx.tree.iter_nodes_of_type::<TypeExpression>() {
                if !self.is_node_active(ctx.tree, ctx.symbols, id.into_any()) {
                    continue;
                }
                if matches!(expression, TypeExpression::Index { .. }) {
                    self.validate_type_index_access(&mut ctx.reborrow(), id);
                }
            }

            // validate decorators
            for (id, annotation) in ctx.tree.iter_nodes_of_type::<Decorator>() {
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

        if should_return_after_validation {
            return Ok(());
        }

        Ok(())
    }
}
