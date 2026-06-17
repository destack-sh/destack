use destack_core::closest_string;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, Origin};
use crate::{
    CheckError, CheckWarning, CompilerResult, DiagnosticAnchor, diagnostic_suggestion_distance,
};

#[allow(clippy::too_many_arguments)]
impl CheckState<'_> {
    /// Report a missing annotation at one source node.
    pub(in crate::check) fn report_missing_type_annotation(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::MissingTypeAnnotation { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report a break with no target.
    pub(in crate::check) fn report_break_outside_control_target(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::BreakOutsideControlTarget { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report a continue with no target loop.
    pub(in crate::check) fn report_continue_outside_loop(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::ContinueOutsideLoop { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report a return outside a function body.
    pub(in crate::check) fn report_return_outside_function(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::ReturnOutsideFunction { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report a yield outside a generator.
    pub(in crate::check) fn report_yield_outside_generator(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::YieldOutsideGenerator { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report a yield delegation without a delegated value.
    pub(in crate::check) fn report_yield_delegate_missing_value(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::YieldDelegateMissingValue { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report an await outside an async context.
    pub(in crate::check) fn report_await_outside_async_context(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::AwaitOutsideAsyncContext { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report an invalid static guard at one source node.
    pub(in crate::check) fn report_invalid_static_guard(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidStaticCondition { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report a missing explicit method receiver.
    pub(in crate::check) fn report_missing_explicit_receiver(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::MissingExplicitReceiver { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report an unresolved reference at one source node.
    pub(in crate::check) fn report_unresolved_reference(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::UnresolvedReference {
            anchor,
            module,
            name: self.path_label(module, path),
            suggestion: self.closest_reference_name(module, source, path),
        };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report an ambiguous reference at one source node.
    pub(in crate::check) fn report_ambiguous_reference(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::AmbiguousReference {
            anchor,
            module,
            name: self.path_label(module, path),
        };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report a decorator target that is not a static declaration name.
    pub(in crate::check) fn report_invalid_decorator_target(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidDecoratorTarget { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report an intrinsic marker outside a compiler-recognized language item.
    pub(in crate::check) fn report_invalid_intrinsic_type(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidIntrinsicType { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report a const marker outside an `as const` assertion.
    pub(in crate::check) fn report_invalid_const_type(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidConstType { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report one generic application with too many arguments.
    pub(in crate::check) fn report_wrong_generic_arity(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        name: String,
        expected: usize,
        supplied: usize,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::WrongGenericArity {
            anchor,
            module,
            name,
            expected,
            supplied,
        };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report unreachable code at one source node.
    pub(in crate::check) fn report_unreachable_code(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let warning = CheckWarning::UnreachableCode { anchor, module };

        self.module_mut(module).warnings.push(warning.into());
    }

    /// Report an unavailable this expression.
    pub(in crate::check) fn report_this_outside_receiver(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::ThisOutsideReceiver { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report an unavailable super expression.
    pub(in crate::check) fn report_super_outside_class(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::SuperOutsideClass { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report a let-else fallback that can complete.
    pub(in crate::check) fn report_let_else_branch_can_complete(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::LetElseBranchCanComplete { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report an unsupported tree expression.
    pub(in crate::check) fn report_unsupported_tree_expression(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::UnsupportedTreeExpression { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report an expression pattern that did not close to a literal.
    pub(in crate::check) fn report_expression_pattern_not_literal(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::ExpressionPatternNotLiteral { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report an invalid writable place at one source node.
    pub(in crate::check) fn report_ambient_lifetime_elided(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::AmbientLifetimeElided { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report an invalid assignment target.
    pub(in crate::check) fn report_invalid_assignment_target(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidAssignmentTarget { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Return the diagnostic anchor for one source node.
    pub(in crate::check) fn diagnostic_anchor(
        &self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) -> DiagnosticAnchor {
        // anchors may point into external modules, notably "declared
        // here" labels on imported declarations
        let view = match self.modules.get(&module) {
            Some(state) => state.view(),
            None => self.external_module(module).view(),
        };
        let span = match view.get_span_by_id(source.id) {
            Some(span) => span,
            None => unreachable!("check node {} has no source span", source.id),
        };

        DiagnosticAnchor::from(span)
    }

    /// Return one check origin's diagnostic anchor.
    pub(in crate::check) fn origin_diagnostic_anchor(
        &self,
        origin: Origin,
    ) -> CompilerResult<(ModuleId, DiagnosticAnchor)> {
        let module = origin.module();
        let anchor = match origin {
            Origin::Node(node) => self.diagnostic_anchor(module, node.local_id),
            Origin::Symbol(symbol) => {
                let source = self
                    .module(symbol.module_id)
                    .symbol_declaration_node(symbol.local_id)?;

                self.diagnostic_anchor(module, source)
            }
            Origin::Type(_) => DiagnosticAnchor::from(module),
        };

        Ok((module, anchor))
    }

    /// Return a circular type diagnostic for one origin.
    pub(in crate::check) fn circular_type_error(
        &self,
        origin: Origin,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;

        Ok(CheckError::CircularType { anchor, module })
    }

    /// Return a human readable path label.
    fn path_label(&self, module: ModuleId, path: &dir::Path) -> String {
        let mut label = String::new();

        // join path segments with dot notation
        for (index, segment) in path.segments.iter().enumerate() {
            if index > 0 {
                label.push('.');
            }

            label.push_str(self.module(module).strings.get(*segment));
        }

        label
    }

    /// Return the closest visible name for one unresolved single-segment path.
    fn closest_reference_name(
        &self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
    ) -> Option<String> {
        let [name] = path.segments.as_slice() else {
            return None;
        };
        let name = self.module(module).strings.get(*name).to_string();

        let bindings = self.module(module).binding_table();
        let scope = self.scope_at(module, bindings, source);
        let mut candidates = Vec::new();

        // collect lexical names visible at the source node
        self.collect_reference_names(module, bindings, scope, &mut candidates);

        // collect profile-provided globals visible to unresolved references
        for key in self
            .module(module)
            .resolved
            .imports
            .global_target_by_key
            .keys()
        {
            if let Some(candidate) = self.reference_key_text(module, key) {
                candidates.push(candidate);
            }
        }

        closest_string(&name, candidates, diagnostic_suggestion_distance(&name))
    }

    /// Collect named lexical bindings visible from one scope cursor.
    fn collect_reference_names(
        &self,
        module: ModuleId,
        bindings: &dir::BindingTable<'_>,
        mut scope: dir::LocalScope,
        candidates: &mut Vec<String>,
    ) {
        loop {
            let current = bindings.get_scope(scope);

            // collect names declared before the visible scope mark
            for (key, symbol) in current.named_symbols_up_to(scope.mark) {
                let kind = bindings.get_symbol(symbol).kind;
                if !kind.is_visible_in(dir::SymbolSpace::Value)
                    && !kind.is_visible_in(dir::SymbolSpace::Type)
                {
                    continue;
                }

                if let Some(candidate) = self.reference_key_text(module, &key) {
                    candidates.push(candidate);
                }
            }

            let Some(parent) = current.parent else {
                return;
            };

            scope = parent;
        }
    }

    /// Return source text for an ordinary reference key.
    fn reference_key_text(&self, module: ModuleId, key: &dir::StaticKey) -> Option<String> {
        match key {
            dir::StaticKey::Name(name) => Some(self.module(module).strings.get(*name).to_string()),
            dir::StaticKey::Index(_) | dir::StaticKey::Symbol(_) => None,
        }
    }
}
