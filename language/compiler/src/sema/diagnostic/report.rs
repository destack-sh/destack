use destack_artifact::{DiagnosticBuilder, DiagnosticControl};
use destack_core::{FxIndexSet, NameMatch};
use destack_dir as dir;
use destack_source::{
    Applicability, DiagnosticSuggestion, FilePatch, ModuleId, Patch, PatchSet, Span,
};

use crate::sema::{
    BoundSide, CauseKind, CheckFailure, CheckState, FailedCheck, MixedObjectSignature,
    ObligationFailure, OperatorOperands, Origin, Relation, SignatureRejection, TypeBound,
    UncoveredValue, ValueUse, Variance,
};
use crate::{CheckError, CheckWarning, CompilerError, CompilerResult, DiagnosticAnchor};

impl CheckState<'_> {
    /// Record one checked error.
    pub(in crate::sema) fn report(
        &mut self,
        module: ModuleId,
        diagnostic: impl Into<DiagnosticBuilder<CheckError>>,
    ) {
        let diagnostic = diagnostic.into();
        let diagnostics = &mut self.module_mut(module).diagnostics;

        // collapse identical re-reports from repeated declaration passes
        if diagnostics.contains(&diagnostic) {
            return;
        }
        diagnostics.push(diagnostic);
    }

    /// Report one interval type with a missing bound.
    pub(in crate::sema) fn report_unbounded_interval_type(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::UnboundedIntervalType { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report one interval type with non-discrete or mixed bounds.
    pub(in crate::sema) fn report_invalid_interval_domain(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidIntervalDomain { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a missing annotation at one source node.
    pub(in crate::sema) fn report_missing_type_annotation(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::MissingTypeAnnotation { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a constructor result annotation.
    pub(in crate::sema) fn report_constructor_result_annotation(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::ConstructorResultAnnotation { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a constructor return value.
    pub(in crate::sema) fn report_constructor_return_value(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::ConstructorReturnValue { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report an export whose type needs another module.
    pub(in crate::sema) fn report_export_type_not_derivable(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::ExportTypeNotDerivable { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report an exported binding without a written type.
    pub(in crate::sema) fn report_missing_export_binding_type(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::MissingExportBindingType { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a break with no target.
    pub(in crate::sema) fn report_break_outside_control_target(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::BreakOutsideControlTarget { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a break value outside a value-carrying target.
    pub(in crate::sema) fn report_break_value_outside_loop(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::BreakValueOutsideLoop { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a continue with no target loop.
    pub(in crate::sema) fn report_continue_outside_loop(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::ContinueOutsideLoop { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a return outside a function body.
    pub(in crate::sema) fn report_return_outside_function(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::ReturnOutsideFunction { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a try propagation outside a function body.
    pub(in crate::sema) fn report_try_outside_function(&mut self, source: dir::GlobalNodeIdAny) {
        let (module, anchor) = self.source_anchor(source);
        let diagnostic = CheckError::TryOutsideFunction { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a yield outside a generator.
    pub(in crate::sema) fn report_yield_outside_generator(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::YieldOutsideGenerator { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a yield delegation without a delegated value.
    pub(in crate::sema) fn report_yield_delegate_missing_value(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::YieldDelegateMissingValue { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a for-of source that has no iterable implementation.
    pub(in crate::sema) fn report_for_of_source_not_iterable(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let diagnostic = CheckError::ForOfSourceNotIterable { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report an await outside an async context.
    pub(in crate::sema) fn report_await_outside_async_context(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::AwaitOutsideAsyncContext { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a non-boolean static guard at one source node.
    pub(in crate::sema) fn report_non_boolean_static_guard(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidStaticCondition { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a malformed static guard invocation at one source node.
    pub(in crate::sema) fn report_invalid_static_if_invocation(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<()> {
        let anchor = self.node_anchor(module, source)?;
        let diagnostic = CheckError::InvalidStaticIfInvocation { anchor, module };

        self.report(module, diagnostic);

        Ok(())
    }

    /// Report a static guard that cannot decide statically.
    pub(in crate::sema) fn report_undecidable_static_guard(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::UndecidableStaticCondition { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a static value expression that cannot decide statically.
    pub(in crate::sema) fn report_undecidable_static_value(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::UndecidableStaticValue { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a missing explicit method receiver.
    pub(in crate::sema) fn report_missing_explicit_receiver(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::MissingExplicitReceiver { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report an unresolved reference at one source node.
    pub(in crate::sema) fn reject_unresolved_reference(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
    ) {
        // retain the failed path so quickfixes can plan imports
        let node = source.into_global(module);
        self.module_mut(module)
            .resolutions
            .set_unresolved_reference(node, path.clone());

        self.report_unresolved_reference(module, source, path);
    }

    /// Report one argument leaving a value-consumed parameter unfixed.
    pub(in crate::sema) fn report_argument_not_exact_value(
        &mut self,
        source: dir::GlobalNodeIdAny,
        argument: dir::GlobalTypeId,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.source_anchor(source);
        let error = CheckError::ArgumentNotExactValue {
            anchor,
            module,
            argument: self.format_type(argument),
            parameter: self.parameter_label(parameter),
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one body read of a parameter value the signature never fixes.
    pub(in crate::sema) fn report_value_read_not_fixed(
        &mut self,
        source: dir::GlobalNodeIdAny,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.source_anchor(source);
        let error = CheckError::ValueReadNotFixed {
            anchor,
            module,
            parameter: self.parameter_label(parameter),
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one overload group referenced without a selecting call.
    pub(in crate::sema) fn report_ambiguous_overload(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.source_anchor(source);
        let error = CheckError::AmbiguousOverload {
            anchor,
            module,
            name: self.format_symbol(symbol),
        };
        self.report(module, error);

        Ok(())
    }

    /// Return one parameter's reported name.
    fn parameter_label(&self, parameter: dir::GlobalGenericParameterId) -> String {
        self.generic_parameter(parameter)
            .and_then(|binding| binding.symbol)
            .map(|symbol| self.format_symbol(symbol))
            .unwrap_or_else(|| "the parameter".to_string())
    }

    /// Report one path naming no visible declaration, suggesting the closest name in scope.
    fn report_unresolved_reference(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let name = self.path_label(path);
        let best = self.closest_reference_name(module, source, path);
        let error = CheckError::UnresolvedReference {
            anchor: anchor.clone(),
            module,
            name: name.clone(),
            suggestion: best.as_ref().map(|best| best.candidate.clone()),
        };

        // attach the name suggestion and any sibling module declaration
        let mut diagnostic = DiagnosticBuilder::new(error);
        if let Some(suggestion) = best
            .as_ref()
            .and_then(|best| self.rename_suggestion(&anchor, best))
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        if let Some(sibling) = self.declaring_sibling_module(module, path) {
            diagnostic = diagnostic
                .label(
                    DiagnosticAnchor::Module(sibling),
                    format!("'{name}' is declared in this module"),
                )
                .help(format!("import '{name}' from that module"));
        }

        self.report(module, diagnostic);
    }

    /// Report an ambiguous reference at one source node.
    pub(in crate::sema) fn report_ambiguous_reference(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let error = CheckError::AmbiguousReference {
            anchor,
            module,
            name: self.path_label(path),
        };

        // point at each conflicting candidate declaration
        let mut diagnostic = DiagnosticBuilder::new(error);
        if let Some(dir::Reference::Ambiguous(candidates)) = self
            .module_maybe(module)
            .and_then(|state| state.resolved.references.get(source.into_global(module)))
        {
            for target in candidates.clone().iter().take(4) {
                let dir::ReferenceTarget::Symbol(symbol) = target else {
                    continue;
                };
                let Ok(declaration) = self.symbol_source(*symbol) else {
                    continue;
                };
                let (_, candidate) = self.source_anchor(declaration);
                diagnostic = diagnostic.label(candidate, "one candidate is declared here");
            }
        }

        self.report(module, diagnostic);
    }

    /// Report a local binding read before assignment.
    pub(in crate::sema) fn report_use_before_assigned(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let error = CheckError::UseBeforeAssigned {
            anchor,
            module,
            name: self.format_symbol(symbol),
        };
        let diagnostic = self.label_binding_declaration(DiagnosticBuilder::new(error), symbol);

        self.report(module, diagnostic);
    }

    /// Report a decorator target that does not name one newtype declaration.
    pub(in crate::sema) fn report_invalid_decorator_target(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidDecoratorTarget { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report decorator arguments that do not prove one unique backing.
    pub(in crate::sema) fn report_ambiguous_decorator(
        &mut self,
        origin: Origin,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let diagnostic = CheckError::AmbiguousDecorator { anchor, module };

        self.report(module, diagnostic);

        Ok(())
    }

    /// Report decorator arguments rejected by every backing alternative.
    pub(in crate::sema) fn report_no_matching_decorator(
        &mut self,
        origin: Origin,
        rejections: &[String],
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::NoMatchingDecorator { anchor, module };
        let mut diagnostic = DiagnosticBuilder::new(error);
        for rejection in rejections {
            diagnostic = diagnostic.note(rejection.clone());
        }

        self.report(module, diagnostic);

        Ok(())
    }

    /// Report an intrinsic marker outside a compiler-recognized language item.
    pub(in crate::sema) fn report_invalid_intrinsic_type(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidIntrinsicType { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a const marker outside an `as const` assertion.
    pub(in crate::sema) fn report_invalid_const_type(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidConstType { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report one object type that mixes named properties with a signature.
    pub(in crate::sema) fn report_mixed_object_type(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        conflict: MixedObjectSignature,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::MixedObjectType {
            anchor,
            module,
            conflict,
        };

        self.report(module, diagnostic);
    }

    /// Report one generic application with too many arguments.
    pub(in crate::sema) fn report_wrong_generic_arity(
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

        self.report(module, diagnostic);
    }

    /// Report an invalid `typeof` type query operand.
    pub(in crate::sema) fn report_invalid_type_query(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidTypeQuery { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report one `infer` declaration outside a conditional extends clause.
    pub(in crate::sema) fn report_infer_outside_conditional(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InferOutsideConditional { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report one cast whose target equals its operand type.
    pub(in crate::sema) fn report_redundant_cast(
        &mut self,
        node: dir::GlobalNodeIdAny,
        value: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) {
        let (module, anchor) = self.source_anchor(node);
        let warning = CheckWarning::RedundantCast {
            anchor,
            module,
            ty: self.format_type_at(module, ty),
        };

        // offer to delete the cast suffix when both spans share a file
        let mut diagnostic = DiagnosticBuilder::new(warning);
        let state = self.module(module);
        if let (Some(node_span), Some(value_span)) = (
            state.diagnostic_span(node.local_id),
            state.diagnostic_span(value.local_id),
        ) && node_span.file == value_span.file
            && value_span.end < node_span.end
        {
            let suffix = Span::new(node_span.file, value_span.end, node_span.end);
            let patches = PatchSet::from_files(vec![FilePatch {
                file: suffix.file,
                patches: vec![Patch::replace(suffix, "")],
            }]);
            diagnostic = diagnostic.suggestion(DiagnosticSuggestion::new(
                "remove the cast",
                patches,
                Applicability::Automatic,
            ));
        }

        self.module_mut(module).warnings.push(diagnostic);
    }

    /// Report one constant shift amount past the shifted width.
    pub(in crate::sema) fn report_shift_out_of_range(
        &mut self,
        origin: Origin,
        amount: i64,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let warning = CheckWarning::ShiftOutOfRange {
            anchor,
            module,
            amount,
            ty: self.format_type_at(module, ty),
        };
        self.module_mut(module).warnings.push(warning.into());

        Ok(())
    }

    /// Report unreachable code at one source node.
    pub(in crate::sema) fn report_unreachable_code(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let warning = CheckWarning::UnreachableCode { anchor, module };

        self.module_mut(module).warnings.push(warning.into());
    }

    /// Report an unavailable this expression.
    pub(in crate::sema) fn report_this_outside_receiver(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::ThisOutsideReceiver { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report an unavailable super expression.
    pub(in crate::sema) fn report_super_outside_class(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::SuperOutsideClass { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a let-else fallback that can complete.
    pub(in crate::sema) fn report_let_else_branch_can_complete(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::LetElseBranchCanComplete { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a tree expression without an active builder.
    pub(in crate::sema) fn report_missing_tree_builder(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::MissingTreeBuilder { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report one tree tag missing from its builder's rows.
    pub(in crate::sema) fn report_unknown_tree_tag(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        tag: dir::StringId,
        builder: dir::GlobalTypeId,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::UnknownTreeTag {
            anchor,
            module,
            tag: self.strings().get(tag).to_string(),
            builder: self.format_type(builder),
        };

        self.report(module, diagnostic);
    }

    /// Report one tree attribute outside the declared row.
    pub(in crate::sema) fn report_unknown_tree_attribute(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        key: dir::StringId,
        row: dir::GlobalTypeId,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::UnknownTreeAttribute {
            anchor,
            module,
            key: self.strings().get(key).to_string(),
            row: self.format_type(row),
        };

        self.report(module, diagnostic);
    }

    /// Report one tree spread child over a dynamically sized operand.
    pub(in crate::sema) fn report_tree_spread_not_tuple(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::TreeSpreadNotTuple {
            anchor,
            module,
            ty: self.format_type(ty),
        };

        self.report(module, diagnostic);
    }

    /// Report one required tree attribute without a written value.
    pub(in crate::sema) fn report_missing_tree_attribute(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        key: &dir::StaticKey,
        row: dir::GlobalTypeId,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::MissingTreeAttribute {
            anchor,
            module,
            key: self.format_static_key(key),
            row: self.format_type(row),
        };

        self.report(module, diagnostic);
    }

    /// Report an expression pattern that did not close to a literal.
    pub(in crate::sema) fn report_expression_pattern_not_literal(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::ExpressionPatternNotLiteral { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report an invalid assignment target.
    pub(in crate::sema) fn report_invalid_assignment_target(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidAssignmentTarget { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report one source occurrence whose type could not be inferred.
    pub(in crate::sema) fn report_cannot_infer_type(
        &mut self,
        origin: Origin,
        variable: Option<dir::TypeVariableId>,
        reported: &mut FxIndexSet<(ModuleId, DiagnosticAnchor)>,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        if !self.is_own_module(module) || !reported.insert((module, anchor.clone())) {
            return Ok(());
        }
        let error = CheckError::CannotInferType {
            anchor: anchor.clone(),
            module,
        };
        let mut diagnostic = DiagnosticBuilder::new(error);

        // show where the collected bounds came from
        if let Some(variable) = variable {
            for (side, bound) in self.variable_bound_list(variable)? {
                let bound_origin = self.cause_origin(bound.cause);
                let (_, bound_anchor) = self.origin_node_anchor(bound_origin)?;
                if bound_anchor == anchor {
                    continue;
                }
                let ty = self.format_type_at(module, bound.ty);
                let message = match (side, bound.relation) {
                    (_, Relation::Equal) => format!("it must equal '{ty}' here"),
                    (BoundSide::Lower, _) => format!("'{ty}' flows into it here"),
                    (BoundSide::Upper, _) => format!("it must satisfy '{ty}' here"),
                };
                diagnostic = diagnostic.label(bound_anchor, message);
            }
        }
        self.report(module, diagnostic);

        Ok(())
    }

    /// Collect one variable's bounds on both sides, capped for display.
    fn variable_bound_list(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<Vec<(BoundSide, TypeBound)>> {
        let mut bounds = Vec::new();
        for side in [BoundSide::Lower, BoundSide::Upper] {
            bounds.extend(
                self.infer
                    .variables
                    .side_bounds(variable, side)?
                    .map(|bound| (side, bound)),
            );
        }
        bounds.truncate(4);

        Ok(bounds)
    }

    /// Report one source node whose type could not be inferred.
    pub(in crate::sema) fn report_cannot_infer_node(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.source_anchor(source);
        let error = CheckError::CannotInferType { anchor, module };
        self.report(module, error);

        Ok(())
    }

    /// Report one borrow expression whose access the source never grants.
    pub(in crate::sema) fn report_borrow_access_not_granted(
        &mut self,
        origin: Origin,
        access: dir::Access,
        granted: Option<dir::Access>,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::BorrowAccessNotGranted {
            anchor,
            module,
            access: access.text().to_string(),
            source: self.format_type_at(module, source),
        };
        let mut diagnostic = DiagnosticBuilder::new(error);
        if let Some(granted) = granted {
            diagnostic = diagnostic.note(format!(
                "the source grants at most '{}' access",
                granted.text()
            ));
        }
        let diagnostic =
            diagnostic.help("request the granted access or use a source that grants more");
        self.report(module, diagnostic);

        Ok(())
    }

    /// Report one missing member with the closest visible suggestion.
    pub(in crate::sema) fn report_missing_member(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: String,
        key_span: Option<Span>,
        best: Option<NameMatch<String>>,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;

        // anchor an exact key span when the access supplies one
        let anchor = match key_span {
            Some(span) if span.len() as usize == key.len() => DiagnosticAnchor::Span(span),
            _ => anchor,
        };
        let receiver_text = self.format_type(receiver);
        let error = CheckError::MissingMember {
            anchor: anchor.clone(),
            module,
            key,
            receiver: receiver_text,
            suggestion: best.as_ref().map(|best| best.candidate.clone()),
        };

        let mut diagnostic = DiagnosticBuilder::new(error);
        if key_span.is_some()
            && let Some(best) = best
            && let Some(suggestion) = self.rename_suggestion(&anchor, &best)
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        self.report(module, diagnostic);

        Ok(())
    }

    /// Report one member access whose target is overloaded.
    pub(in crate::sema) fn report_ambiguous_member(
        &mut self,
        origin: Origin,
        key: String,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::AmbiguousMember {
            anchor,
            module,
            key,
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one read through a write-only member.
    pub(in crate::sema) fn report_write_only_member(
        &mut self,
        origin: Origin,
        member: String,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::CannotReadWriteOnlyMember {
            anchor,
            module,
            member,
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one instance method read as a value.
    pub(in crate::sema) fn report_bound_method_extraction(
        &mut self,
        origin: Origin,
        member: String,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::CannotExtractBoundMethod {
            anchor,
            module,
            member,
        };
        let diagnostic = DiagnosticBuilder::new(error)
            .help("wrap the read in a closure to make its receiver capture explicit");
        self.report(module, diagnostic);

        Ok(())
    }

    /// Report one write through a readonly member.
    pub(in crate::sema) fn report_readonly_member(
        &mut self,
        origin: Origin,
        member: String,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::CannotAssignReadonlyMember {
            anchor,
            module,
            member,
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one value that is not callable.
    pub(in crate::sema) fn report_not_callable(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::NotCallable {
            anchor,
            module,
            ty: self.format_type(ty),
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one call whose arguments match no overload.
    pub(in crate::sema) fn report_no_matching_call(
        &mut self,
        origin: Origin,
        arguments: &[dir::GlobalTypeId],
        rejections: &[String],
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::NoMatchingCall {
            anchor,
            module,
            arguments: self.format_types(arguments),
        };
        let mut diagnostic = DiagnosticBuilder::new(error);
        for rejection in rejections {
            diagnostic = diagnostic.note(rejection.clone());
        }
        self.report(module, diagnostic);

        Ok(())
    }

    /// Describe why one candidate signature rejected an invocation.
    pub(in crate::sema) fn describe_signature_rejection(
        &self,
        module: ModuleId,
        candidate: dir::GlobalTypeId,
        rejection: &SignatureRejection,
    ) -> CompilerResult<String> {
        let candidate = self.format_type_at(module, candidate);
        let reason = match rejection {
            SignatureRejection::Inapplicable => "does not apply".to_string(),
            SignatureRejection::Arity {
                required,
                total,
                has_rest,
                supplied,
            } => {
                let expected = Self::format_argument_count(*required, *total, *has_rest);

                format!("takes {expected}, got {supplied}")
            }
            SignatureRejection::Mismatch {
                cause,
                relation,
                source,
                target,
                ..
            } => {
                let source = self.format_type_at(module, *source);
                let target = self.format_type_at(module, *target);

                // describe arguments by their authored position
                match self.root_cause(*cause).kind {
                    CauseKind::Argument { index, .. } => format!(
                        "rejects argument {index}: '{source}' is not assignable to '{target}'"
                    ),
                    _ => match relation {
                        Relation::Equal => format!("requires '{source}' to equal '{target}'"),
                        Relation::Satisfies => {
                            format!("requires '{source}' to satisfy '{target}'")
                        }
                        _ => format!("rejects '{source}' as '{target}'"),
                    },
                }
            }
            SignatureRejection::Receiver { source, target } => format!(
                "rejects the receiver: '{}' is not assignable to '{}'",
                self.format_type_at(module, *source),
                self.format_type_at(module, *target),
            ),
        };

        Ok(format!("the candidate '{candidate}' {reason}"))
    }

    /// Report one construction whose arguments match no constructor.
    pub(in crate::sema) fn report_no_matching_construct(
        &mut self,
        origin: Origin,
        arguments: &[dir::GlobalTypeId],
        rejections: &[String],
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::NoMatchingConstruct {
            anchor,
            module,
            arguments: self.format_types(arguments),
        };
        let mut diagnostic = DiagnosticBuilder::new(error);
        for rejection in rejections {
            diagnostic = diagnostic.note(rejection.clone());
        }
        self.report(module, diagnostic);

        Ok(())
    }

    /// Report one argument that does not name a derivable interface.
    pub(in crate::sema) fn report_invalid_derive_interface(
        &mut self,
        origin: Origin,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::InvalidDeriveInterface { anchor, module };
        self.report(module, error);

        Ok(())
    }

    /// Report one derive interface selected more than once for a declaration.
    pub(in crate::sema) fn report_duplicate_derive_interface(
        &mut self,
        origin: Origin,
        interface: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::DuplicateDeriveInterface {
            anchor,
            module,
            interface: self.format_symbol(interface),
        };
        self.report(module, error);

        Ok(())
    }

    /// Report a capture decorator without a declared function value.
    pub(in crate::sema) fn report_invalid_capture_target(
        &mut self,
        source: dir::GlobalNodeId<dir::Decorator>,
    ) -> CompilerResult<()> {
        let module = source.module_id;
        let anchor = self.node_anchor(module, source.local_id.into_any())?;
        let error = CheckError::InvalidCaptureTarget { anchor, module };
        self.report(module, error);

        Ok(())
    }

    /// Report more than one capture decorator for one function value.
    pub(in crate::sema) fn report_duplicate_capture_decorator(
        &mut self,
        source: dir::GlobalNodeId<dir::Decorator>,
        previous: dir::GlobalNodeId<dir::Decorator>,
    ) -> CompilerResult<()> {
        let module = source.module_id;
        let anchor = self.node_anchor(module, source.local_id.into_any())?;
        let error = CheckError::DuplicateCaptureDecorator { anchor, module };
        let previous = self.node_anchor(module, previous.local_id.into_any())?;
        let diagnostic =
            DiagnosticBuilder::new(error).label(previous, "first capture directive here");
        self.report(module, diagnostic);

        Ok(())
    }

    /// Report a diagnostic control that overrides an enclosing forbid.
    pub(in crate::sema) fn report_forbidden_diagnostic_override(
        &mut self,
        module: ModuleId,
        control: &DiagnosticControl,
        forbidden: &DiagnosticControl,
    ) {
        let diagnostic = self.strings().get(control.diagnostic).to_string();
        let error = CheckError::ForbiddenDiagnosticOverride {
            anchor: DiagnosticAnchor::Span(control.source),
            module,
            diagnostic,
        };
        let forbidden = DiagnosticAnchor::Span(forbidden.source);
        let diagnostic = DiagnosticBuilder::new(error).label(forbidden, "forbidden here");

        self.report(module, diagnostic);
    }

    /// Report an unknown diagnostic id.
    pub(in crate::sema) fn report_unknown_diagnostic_control(
        &mut self,
        module: ModuleId,
        source: Span,
        diagnostic: String,
    ) {
        let error = CheckError::UnknownDiagnostic {
            anchor: DiagnosticAnchor::Span(source),
            module,
            diagnostic,
        };

        self.report(module, error);
    }

    /// Report a diagnostic that cannot be controlled.
    pub(in crate::sema) fn report_uncontrollable_diagnostic(
        &mut self,
        module: ModuleId,
        source: Span,
        diagnostic: String,
    ) {
        let error = CheckError::UncontrollableDiagnostic {
            anchor: DiagnosticAnchor::Span(source),
            module,
            diagnostic,
        };

        self.report(module, error);
    }

    /// Report one call with the wrong argument count.
    pub(in crate::sema) fn report_wrong_argument_count(
        &mut self,
        origin: Origin,
        expected: String,
        supplied: usize,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::WrongArgumentCount {
            anchor,
            module,
            expected,
            supplied,
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one final signature rejection.
    pub(in crate::sema) fn report_signature_rejection(
        &mut self,
        origin: Origin,
        rejection: SignatureRejection,
    ) -> CompilerResult<()> {
        match rejection {
            // skip imprecise candidate failures
            SignatureRejection::Inapplicable => {}

            // report arity mismatch on the call itself
            SignatureRejection::Arity {
                required,
                total,
                has_rest,
                supplied,
            } => {
                let expected = Self::format_argument_count(required, total, has_rest);
                self.report_wrong_argument_count(origin, expected, supplied)?;
            }

            // report the selected mismatch at its authored cause
            SignatureRejection::Mismatch {
                cause,
                relation,
                use_,
                source,
                target,
                failure,
                ..
            } => {
                self.record_failure(FailedCheck {
                    cause,
                    relation,
                    use_,
                    source,
                    target,
                    failure,
                    is_provisional: false,
                })?;
            }

            // report receiver mismatch on the call itself
            SignatureRejection::Receiver { source, target } => {
                let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
                let error = CheckError::ReceiverNotAssignable {
                    anchor,
                    module,
                    source: self.format_type_at(module, source),
                    target: self.format_type_at(module, target),
                };
                self.report(module, error);
            }
        }

        Ok(())
    }

    /// Format one argument count phrase.
    fn format_argument_count(required: usize, total: usize, has_rest: bool) -> String {
        let phrase = match (has_rest, required == total) {
            (true, _) => format!("at least {required}"),
            (false, true) => format!("{total}"),
            (false, false) => format!("{required} to {total}"),
        };
        let noun = if phrase.ends_with('1') && !phrase.ends_with("11") {
            "argument"
        } else {
            "arguments"
        };

        format!("{phrase} {noun}")
    }

    /// Report one abstract class construction.
    pub(in crate::sema) fn report_cannot_construct_abstract_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::CannotConstructAbstractType {
            anchor,
            module,
            ty: self.format_type(ty),
        };
        let diagnostic =
            DiagnosticBuilder::new(error).help("construct a concrete subclass instead");
        self.report(module, diagnostic);

        Ok(())
    }

    /// Report one value that cannot be constructed with `new`.
    pub(in crate::sema) fn report_not_constructible(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
        hint: &str,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::NotConstructible {
            anchor,
            module,
            ty: self.format_type(target),
            hint: hint.to_string(),
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one inferred construction target that is not a concrete newtype.
    pub(in crate::sema) fn report_invalid_inferred_construct_target(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::InvalidInferredConstructTarget {
            anchor,
            module,
            ty: self.format_type(target),
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one member read through a possibly nullish value.
    pub(in crate::sema) fn report_possibly_nullish(
        &mut self,
        origin: Origin,
        nullish: String,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::PossiblyNullish {
            anchor,
            module,
            nullish,
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one duplicate property slot owned by another visible extension.
    pub(in crate::sema) fn report_duplicate_extension_member(
        &mut self,
        source: dir::GlobalNodeIdAny,
        key: &dir::StaticKey,
        target: String,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let member = self.format_static_key(key);
        let error = CheckError::DuplicateMember {
            anchor,
            module,
            member,
            target: Some(target),
        };
        self.report(module, error);
    }

    /// Report one extension member redeclaring a member of its root declaration.
    pub(in crate::sema) fn report_inherent_member_redeclared(
        &mut self,
        source: dir::GlobalNodeIdAny,
        key: &dir::StaticKey,
        target: String,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let member = self.format_static_key(key);
        let error = CheckError::InherentMemberRedeclared {
            anchor,
            module,
            member,
            target,
        };
        self.report(module, error);
    }

    /// Report one operator application that matches no overload.
    pub(in crate::sema) fn report_no_matching_operator(
        &mut self,
        origin: Origin,
        operator: String,
        operands: OperatorOperands<'_>,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let operands = self.format_operator_operands(operands);
        let error = CheckError::NoMatchingOperator {
            anchor,
            module,
            operator,
            operands,
        };
        self.report(module, error);

        Ok(())
    }

    /// Format the operand phrase for an operator rejection.
    fn format_operator_operands(&self, operands: OperatorOperands<'_>) -> String {
        match operands {
            OperatorOperands::Types(operands) => operands
                .iter()
                .map(|operand| format!("'{}'", self.format_type(*operand)))
                .collect::<Vec<_>>()
                .join(" and "),
            OperatorOperands::Place => "place".to_string(),
        }
    }

    /// Report one spread expression whose source has no fields.
    pub(in crate::sema) fn report_spread_not_object(
        &mut self,
        origin: Origin,
        spread: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::SpreadNotObject {
            anchor,
            module,
            source: self.format_type(spread),
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one sequence pattern with a non-sequence source.
    pub(in crate::sema) fn report_pattern_source_not_sequence_shaped(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::PatternSourceNotSequenceShaped {
            anchor,
            module,
            source: self.format_type(source),
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one rest pattern that appears before another field.
    pub(in crate::sema) fn report_rest_pattern_not_last(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let error = CheckError::RestPatternNotLast { anchor, module };

        self.report(module, error);
    }

    /// Report one extra rest pattern in the same field list.
    pub(in crate::sema) fn report_multiple_rest_patterns(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let error = CheckError::MultipleRestPatterns { anchor, module };

        self.report(module, error);
    }

    /// Report one object pattern with a non-object source.
    pub(in crate::sema) fn report_pattern_source_not_object_shaped(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::PatternSourceNotObjectShaped {
            anchor,
            module,
            source: self.format_type(source),
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one tuple pattern with a non-tuple source.
    pub(in crate::sema) fn report_pattern_source_not_tuple_shaped(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::PatternSourceNotTupleShaped {
            anchor,
            module,
            source: self.format_type(source),
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one bare arm pattern that shadows a type.
    pub(in crate::sema) fn report_pattern_shadows_type(
        &mut self,
        origin: Origin,
        name: String,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::PatternShadowsType {
            anchor,
            module,
            name,
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one missing pattern field.
    pub(in crate::sema) fn report_pattern_field_missing(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: String,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::PatternFieldMissing {
            anchor,
            module,
            key,
            receiver: self.format_type(receiver),
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one pattern member that is not a field.
    pub(in crate::sema) fn report_pattern_member_not_field(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: String,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::PatternMemberNotField {
            anchor,
            module,
            key,
            receiver: self.format_type(receiver),
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one repeated pattern field.
    pub(in crate::sema) fn report_duplicate_pattern_field(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        first_source: dir::LocalNodeIdAny,
        key: String,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let first = self.diagnostic_anchor(module, first_source);
        let error = CheckError::DuplicatePatternField {
            anchor,
            module,
            key,
        };
        let diagnostic = DiagnosticBuilder::new(error).label(first, "first matched here");

        self.report(module, diagnostic);
    }

    /// Report one repeated enum runtime value.
    pub(in crate::sema) fn report_duplicate_enum_variant_value(
        &mut self,
        source: dir::GlobalNodeIdAny,
        previous: dir::GlobalNodeIdAny,
        value: String,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let (_, previous) = self.source_anchor(previous);
        let error = CheckError::DuplicateEnumVariantValue {
            anchor,
            module,
            value,
        };
        let diagnostic = DiagnosticBuilder::new(error).label(previous, "first declared here");

        self.report(module, diagnostic);
    }

    /// Report one repeated pattern binding.
    pub(in crate::sema) fn report_duplicate_pattern_binding(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        name: String,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let error = CheckError::DuplicatePatternBinding {
            anchor,
            module,
            name,
        };

        self.report(module, error);
    }

    /// Report one computed pattern key that cannot select a field.
    pub(in crate::sema) fn report_computed_pattern_key_not_valid(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let error = CheckError::ComputedPatternKeyNotValid { anchor, module };

        self.report(module, error);
    }

    /// Report one pattern whose tag is not nominal.
    pub(in crate::sema) fn report_invalid_pattern_tag(
        &mut self,
        origin: Origin,
        tag: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::InvalidPatternTag {
            anchor,
            module,
            ty: self.format_type(tag),
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one variant pattern whose owner does not match the input.
    pub(in crate::sema) fn report_pattern_variant_not_in_type(
        &mut self,
        origin: Origin,
        variant: String,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let source = self.format_type(source);
        let error = CheckError::PatternVariantNotInType {
            anchor,
            module,
            variant,
            source,
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one variant pattern that names no case on its owner.
    pub(in crate::sema) fn report_pattern_variant_missing(
        &mut self,
        origin: Origin,
        variant: String,
        owner: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let owner = self.format_type(owner);
        let error = CheckError::PatternVariantMissing {
            anchor,
            module,
            variant,
            owner,
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one strict comparison over a value type without identity.
    pub(in crate::sema) fn report_no_strict_identity(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let ty = self.format_type(ty);
        let error = CheckError::NoStrictIdentity { anchor, module, ty };
        self.report(module, error);

        Ok(())
    }

    /// Report one strict comparison over non-overlapping operands.
    pub(in crate::sema) fn report_invalid_strict_equality(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let left = self.format_type(left);
        let right = self.format_type(right);
        let error = CheckError::InvalidStrictEquality {
            anchor,
            module,
            left,
            right,
        };
        self.report(module, error);

        Ok(())
    }

    /// Report an `instanceof` target that cannot select one class declaration.
    pub(in crate::sema) fn report_instanceof_target_not_class(
        &mut self,
        target: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        let module = target.module_id;
        let anchor = self.diagnostic_anchor(module, target.local_id);
        let error = CheckError::InstanceOfTargetNotClass { anchor, module };
        self.report(module, error);

        Ok(())
    }

    /// Report a runtime predicate target that has no executable representation.
    pub(in crate::sema) fn report_runtime_predicate_not_testable(
        &mut self,
        target_node: dir::GlobalNodeIdAny,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let module = target_node.module_id;
        let anchor = self.diagnostic_anchor(module, target_node.local_id);
        let target = self.format_type(target);
        let error = CheckError::RuntimePredicateNotTestable {
            anchor,
            module,
            target,
        };
        self.report(module, error);

        Ok(())
    }

    /// Emit one failed closed check.
    pub(in crate::sema) fn emit_failure(&mut self, check: &FailedCheck) -> CompilerResult<bool> {
        let FailedCheck {
            cause,
            relation,
            use_: value_use,
            source,
            target,
            failure,
            ..
        } = *check;

        // skip reporting once source or target already reported an error
        if self.any_error_operand(&[source, target])? {
            return Ok(false);
        }

        // anchor the diagnostic at the site the cause names
        let origin = self.cause_origin(cause);
        let root_kind = self.root_cause(cause).kind;
        let (module, anchor) = match (root_kind, value_use) {
            // anchor initializers and stores at their own node
            (CauseKind::Initializer { .. }, _) | (_, Some(ValueUse::Store)) => {
                self.origin_node_anchor(origin)?
            }
            // anchor every other cause at its diagnostic site
            _ => self.origin_diagnostic_anchor(origin)?,
        };

        // re-walk failed closed relations to their mismatched leaf
        let blame = match failure {
            CheckFailure::Relation => self.blame_relation(origin, relation, source, target)?,
            _ => None,
        };
        let is_place_relabel = matches!(failure, CheckFailure::Relation)
            && self.is_place_relabel(origin, source, target)?;

        // render both operands at the reporting module
        let source = self.format_type_at(module, source);
        let target = self.format_type_at(module, target);

        // translate the selected failure reason
        let diagnostic = match failure {
            // report the relation itself
            CheckFailure::Relation => {
                let error = self.constraint_relation_error(
                    anchor.clone(),
                    module,
                    relation,
                    value_use,
                    source,
                    target,
                );
                let diagnostic = DiagnosticBuilder::new(error);

                // explain that a value keeps the storage placement it was created in
                if is_place_relabel {
                    diagnostic.note("a value never changes its space").help(
                        "use a value in the destination placement or create a new value there",
                    )
                }
                // otherwise report the mismatch on its own
                else {
                    diagnostic
                }
            }
            // ask for the annotation that decides an ambiguous relation
            CheckFailure::Undecided => {
                let error = self.constraint_relation_error(
                    anchor.clone(),
                    module,
                    relation,
                    value_use,
                    source,
                    target,
                );

                DiagnosticBuilder::new(error)
                    .note("inference cannot decide this relation")
                    .help("annotate the type explicitly")
            }
            // report a source that cannot erase behind its erased target
            CheckFailure::NotErasable => {
                let error = CheckError::NotErasable {
                    anchor: anchor.clone(),
                    module,
                    source,
                    target,
                };

                DiagnosticBuilder::new(error)
            }
            // report a value converting to several represented union cases
            CheckFailure::AmbiguousUnionCoercion => {
                let error = CheckError::AmbiguousUnionCoercion {
                    anchor: anchor.clone(),
                    module,
                    source,
                    target,
                };

                DiagnosticBuilder::new(error)
            }
            // report the required key the literal missed
            CheckFailure::MissingRequiredProperty { key } => {
                let error = CheckError::MissingRequiredProperty {
                    anchor: anchor.clone(),
                    module,
                    key: self.format_static_key(&key),
                    target,
                };

                DiagnosticBuilder::new(error)
            }
            // report the unknown key the literal supplied
            CheckFailure::ExcessProperty { key } => {
                let error = CheckError::ExcessProperty {
                    anchor: anchor.clone(),
                    module,
                    key: self.format_static_key(&key),
                    target,
                };

                DiagnosticBuilder::new(error)
                    .note("object literals may only specify known properties")
            }
            // report the writable index signature the source misses
            CheckFailure::WritableIndexRequiresIndexSet { signature } => {
                let error = CheckError::WritableIndexRequiresIndexSet {
                    anchor: anchor.clone(),
                    module,
                    source,
                    key: self.format_type_at(module, signature.key_type),
                    value: self.format_type_at(module, signature.value_type),
                };

                DiagnosticBuilder::new(error)
            }
            // the inner site reported the failure
            CheckFailure::Reported => return Ok(false),
        };

        // explain the cause chain and report the failure once
        let diagnostic = self.explain_cause(diagnostic, cause, &anchor, blame.as_ref())?;
        self.report(module, diagnostic);

        Ok(true)
    }

    /// Build the diagnostic for one unstable overwrite.
    fn overwrite_stability_diagnostic(
        anchor: DiagnosticAnchor,
        module: ModuleId,
        ty: String,
    ) -> DiagnosticBuilder<CheckError> {
        let error = CheckError::OverwriteStabilityNotSatisfied { anchor, module, ty };

        DiagnosticBuilder::new(error)
            .note("overwriting may invalidate live borrows of the old value")
            .help("write through an exclusive or owned path or store an overwrite-stable type")
    }

    /// Report one failed obligation.
    pub(in crate::sema) fn report_obligation_failure(
        &mut self,
        failure: ObligationFailure,
    ) -> CompilerResult<()> {
        match failure {
            ObligationFailure::NonExhaustivePattern { source, missing } => {
                let (module, anchor) = self.source_anchor(source);
                let missing = self.format_uncovered_value(missing);
                let error = CheckError::NonExhaustivePattern {
                    anchor,
                    module,
                    missing,
                };
                let diagnostic = error.help("cover the remaining values or add a wildcard '_' arm");
                self.report(module, diagnostic);
            }
            ObligationFailure::RefutablePattern { source, missing } => {
                let (module, anchor) = self.source_anchor(source);
                let missing = self.format_uncovered_value(missing);
                let error = CheckError::RefutablePattern {
                    anchor,
                    module,
                    missing,
                };
                let diagnostic = error.help("handle the uncovered values with 'if let' or 'match'");
                self.report(module, diagnostic);
            }
            ObligationFailure::RefutableCatchPattern { source, missing } => {
                let (module, anchor) = self.source_anchor(source);
                let missing = self.format_uncovered_value(missing);
                let error = CheckError::RefutableCatchPattern {
                    anchor,
                    module,
                    missing,
                };
                let diagnostic = error.help("catch bindings must handle every failure value");
                self.report(module, diagnostic);
            }
            ObligationFailure::ForInSourceNotObjectShaped { source } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::ForInSourceNotObjectShaped { anchor, module };
                self.report(module, error);
            }
            ObligationFailure::IncompatibleRangeEndpoints { source, element } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::IncompatibleRangeEndpoints {
                    anchor,
                    module,
                    element: self.format_type(element),
                };
                self.report(module, error);
            }
            ObligationFailure::ImpossibleIs {
                source,
                value,
                target,
            } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::ImpossibleIs {
                    anchor,
                    module,
                    source: self.format_type(value),
                    target: self.format_type(target),
                };
                self.report(module, error);
            }
            ObligationFailure::ImpossibleInstanceOf {
                source,
                value,
                target,
            } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::ImpossibleInstanceOf {
                    anchor,
                    module,
                    source: self.format_type(value),
                    target: self.format_symbol(target),
                };
                self.report(module, error);
            }
            ObligationFailure::InvalidInPredicate {
                source,
                key,
                receiver,
            } => {
                let (module, anchor) = self.source_anchor(source);
                let key = self.format_type(key);
                let receiver = self.format_type(receiver);
                let error = CheckError::NoMatchingOperator {
                    anchor,
                    module,
                    operator: "in".to_string(),
                    operands: format!("'{key}' and '{receiver}'"),
                };
                self.report(module, error);
            }
            ObligationFailure::CannotAssignImportedBinding { source, symbol } => {
                let (module, anchor) = self.source_anchor(source);
                let name = self.format_assignment_binding(source, symbol);
                let error = CheckError::CannotAssignImportedBinding {
                    anchor,
                    module,
                    name,
                };
                let diagnostic =
                    self.label_binding_declaration(DiagnosticBuilder::new(error), symbol);
                self.report(module, diagnostic);
            }
            ObligationFailure::CannotAssignImmutableBinding { source, symbol } => {
                let (module, anchor) = self.source_anchor(source);
                let name = self.format_assignment_binding(source, symbol);
                let error = CheckError::CannotAssignImmutableBinding {
                    anchor,
                    module,
                    name,
                };
                let diagnostic =
                    self.label_binding_declaration(DiagnosticBuilder::new(error), symbol);
                self.report(module, diagnostic);
            }
            ObligationFailure::CannotAssignReadonlyMember { source, member } => {
                let (module, anchor) = self.source_anchor(source);
                let member = self.format_readonly_member(&member)?;
                let error = CheckError::CannotAssignReadonlyMember {
                    anchor,
                    module,
                    member,
                };
                self.report(module, error);
            }
            ObligationFailure::CannotAssignStructuralIndex { source, receiver } => {
                let (module, anchor) = self.source_anchor(source);
                let receiver = self.format_type(receiver);
                let error = CheckError::CannotAssignStructuralIndex {
                    anchor,
                    module,
                    receiver,
                };
                self.report(module, error);
            }
            ObligationFailure::OverwriteStabilityNotSatisfied { source, ty } => {
                let (module, anchor) = self.source_anchor(source);
                let ty = self.format_type(ty);
                let diagnostic = Self::overwrite_stability_diagnostic(anchor, module, ty);
                self.report(module, diagnostic);
            }
            ObligationFailure::CircularType { source } => {
                let error = self.circular_type_error(Origin::Node(source, None))?;
                self.report(source.module_id, error);
            }
            ObligationFailure::LocalReferenceInSharedStorage { source } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::LocalReferenceInSharedStorage { anchor, module };
                let diagnostic = DiagnosticBuilder::new(error)
                    .note("managed, owned, and borrowed references retain their referent")
                    .help(
                        "place the referenced value in shared space or keep the destination local",
                    );
                self.report(module, diagnostic);
            }
            ObligationFailure::InvalidIndexReceiver { source, receiver } => {
                let (module, anchor) = self.source_anchor(source);
                let receiver = self.format_type(receiver);
                let error = CheckError::InvalidIndexReceiver {
                    anchor,
                    module,
                    receiver,
                };
                self.report(module, error);
            }
            ObligationFailure::InvalidIndexKey {
                source,
                receiver,
                key,
            } => {
                let (module, anchor) = self.source_anchor(source);
                let receiver = self.format_type(receiver);
                let key = self.format_type(key);
                let error = CheckError::InvalidIndexKey {
                    anchor,
                    module,
                    receiver,
                    key,
                };
                self.report(module, error);
            }
            ObligationFailure::ConflictingDeclarationPlacement {
                source,
                symbol,
                written,
                declared,
            } => {
                let (module, anchor) = self.source_anchor(source);
                let (_, declaration_anchor) = self.source_anchor(self.symbol_source(symbol)?);
                let error = CheckError::PlacementConflict {
                    anchor,
                    module,
                    written: written.text().to_string(),
                    declared: declared.text().to_string(),
                };
                let help = match written {
                    dir::Space::Local => "remove 'local' or use a local type",
                    dir::Space::Shared => "remove 'shared' or use a shared type",
                };
                let diagnostic = DiagnosticBuilder::new(error)
                    .label(
                        declaration_anchor,
                        format!("'{}' is {}", self.format_symbol(symbol), declared.text()),
                    )
                    .help(help);
                self.report(module, diagnostic);
            }
            ObligationFailure::AutoInterfaceNotSatisfied {
                source,
                ty,
                interface,
            } => {
                self.report_auto_interface_failure(source, ty, interface);
            }
            ObligationFailure::InterfaceNotImplemented {
                source,
                ty,
                interface,
            } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::InterfaceNotImplemented {
                    anchor,
                    module,
                    source: self.format_type(ty),
                    target: self.format_type(interface),
                };
                self.report(module, error);
            }
            ObligationFailure::NonLocalImplementation {
                source,
                interface,
                root,
            } => {
                self.report_non_local_implementation(source, interface, root);
            }
            ObligationFailure::ForeignBlanketImplementation { source, interface } => {
                self.report_foreign_blanket_implementation(source, interface);
            }
            ObligationFailure::UnanchoredBlanketMember { source, member } => {
                self.report_unanchored_blanket_member(source, member);
            }
            ObligationFailure::UnconstrainedExtensionParameter { source, parameter } => {
                self.report_unconstrained_extension_parameter(source, parameter);
            }
            ObligationFailure::UnnamedExportedNonlocalExtension { source, target } => {
                self.report_unnamed_exported_nonlocal_extension(source, target);
            }
            ObligationFailure::DuplicateExtensionMember {
                source,
                member,
                target,
            } => {
                self.report_duplicate_extension_member(source, &member, target);
            }
            ObligationFailure::InherentMemberRedeclared {
                source,
                member,
                target,
            } => {
                self.report_inherent_member_redeclared(source, &member, target);
            }
            ObligationFailure::ConflictingImplementation {
                source,
                conflict,
                interface,
                ty,
            } => {
                self.report_conflicting_implementation(source, conflict, interface, ty)?;
            }
            ObligationFailure::ConflictingHeritage {
                source,
                symbol,
                target,
            } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::ConflictingHeritage {
                    anchor,
                    module,
                    source: self.format_symbol(symbol),
                    target: self.format_symbol(target),
                };
                self.report(module, error);
            }
            ObligationFailure::CircularHeritage { source, symbol } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::CircularHeritage {
                    anchor,
                    module,
                    source: self.format_symbol(symbol),
                };
                self.report(module, error);
            }
            ObligationFailure::ConflictingHeritagePlacement {
                source,
                symbol,
                conflict_source,
                conflict,
            } => {
                let (module, anchor) = self.source_anchor(source);
                let (_, conflict_anchor) = self.source_anchor(conflict_source);
                let (_, declaration_anchor) = self.source_anchor(self.symbol_source(symbol)?);
                let (_, conflict_declaration_anchor) =
                    self.source_anchor(self.symbol_source(conflict)?);
                let error = CheckError::HeritagePlacementConflict { anchor, module };
                let diagnostic = DiagnosticBuilder::new(error)
                    .label(conflict_anchor, "conflicting placement")
                    .label(
                        declaration_anchor,
                        format!("'{}' is declared here", self.format_symbol(symbol)),
                    )
                    .label(
                        conflict_declaration_anchor,
                        format!("'{}' is declared here", self.format_symbol(conflict)),
                    )
                    .help("make every base and implemented interface use the same placement");
                self.report(module, diagnostic);
            }
            ObligationFailure::InvalidOverride { source, member } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::InvalidOverride {
                    anchor,
                    module,
                    member: self.format_static_key(&member),
                };
                self.report(module, error);
            }
            ObligationFailure::OverrideNotVirtual { source, member } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::OverrideNotVirtual {
                    anchor,
                    module,
                    member: self.format_static_key(&member),
                };
                let diagnostic = error.help("declare the inherited member 'virtual' or 'abstract'");
                self.report(module, diagnostic);
            }
            ObligationFailure::IncompatibleOverride {
                source,
                member,
                source_ty,
                target_ty,
            } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::IncompatibleOverride {
                    anchor,
                    module,
                    member: self.format_static_key(&member),
                    source: self.format_type(source_ty),
                    target: self.format_type(target_ty),
                };
                self.report(module, error);
            }
            ObligationFailure::MissingOverride { source, member } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::MissingOverride {
                    anchor,
                    module,
                    member: self.format_static_key(&member),
                };
                let diagnostic = error.help("add the 'override' modifier");
                self.report(module, diagnostic);
            }
            ObligationFailure::AbstractMemberInConcreteClass { source, member } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::AbstractMemberInConcreteClass {
                    anchor,
                    module,
                    member: self.format_static_key(&member),
                };
                self.report(module, error);
            }
            ObligationFailure::UnimplementedAbstractMember { source, member } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::UnimplementedAbstractMember {
                    anchor,
                    module,
                    member: self.format_static_key(&member),
                };
                let diagnostic = error.help("implement the member or declare the class 'abstract'");
                self.report(module, diagnostic);
            }
            ObligationFailure::FinalClassExtended { source, base } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::FinalClassExtended {
                    anchor,
                    module,
                    ty: self.format_symbol(base),
                };
                self.report(module, error);
            }
            ObligationFailure::UnusedGenericParameter { source, parameter } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::UnusedGenericParameter {
                    anchor,
                    module,
                    name: self.format_symbol(parameter),
                };
                let diagnostic =
                    error.help("declare explicit variance like 'out T' to keep a marker parameter");
                self.report(module, diagnostic);
            }
            ObligationFailure::VarianceConflict {
                source,
                parameter,
                derived,
                declared,
            } => {
                let (module, anchor) = self.source_anchor(source);
                let usage = match derived {
                    Variance::Covariant => "covariantly",
                    Variance::Contravariant => "contravariantly",
                    Variance::Invariant | Variance::Bivariant => "invariantly",
                };
                let error = CheckError::VarianceConflict {
                    anchor,
                    module,
                    name: self.format_symbol(parameter),
                    usage: usage.to_string(),
                    declared: declared.as_str().to_string(),
                };
                self.report(module, error);
            }
            ObligationFailure::FieldNotDefinitelyInitialized { source, field } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::FieldNotDefinitelyInitialized {
                    anchor,
                    module,
                    field: self.format_symbol(field),
                };
                self.report(module, error);
            }
        }

        Ok(())
    }

    /// Report one failed auto-interface obligation.
    fn report_auto_interface_failure(
        &mut self,
        source: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let diagnostic: DiagnosticBuilder<CheckError> = match interface {
            dir::AutoInterface::DynamicSafe => {
                let ty = self.format_type(ty);
                let error = CheckError::DynamicSafetyNotSatisfied { anchor, module, ty };

                error.into()
            }
            dir::AutoInterface::OverwriteStable => {
                let ty = self.format_type(ty);
                Self::overwrite_stability_diagnostic(anchor, module, ty)
            }
            dir::AutoInterface::AtomicSafe
            | dir::AutoInterface::Integer
            | dir::AutoInterface::IntegerDomain
            | dir::AutoInterface::Float
            | dir::AutoInterface::FloatDomain
            | dir::AutoInterface::Concrete
            | dir::AutoInterface::Copy
            | dir::AutoInterface::Clone
            | dir::AutoInterface::Debug
            | dir::AutoInterface::Default
            | dir::AutoInterface::Display
            | dir::AutoInterface::Hash
            | dir::AutoInterface::Equal
            | dir::AutoInterface::PartialEqual
            | dir::AutoInterface::Compare
            | dir::AutoInterface::PartialCompare
            | dir::AutoInterface::Serialize
            | dir::AutoInterface::Deserialize
            | dir::AutoInterface::SharedSafe
            | dir::AutoInterface::StrictEqual
            | dir::AutoInterface::Unpin
            | dir::AutoInterface::Zeroable => {
                let error = CheckError::ConstraintNotSatisfied {
                    anchor,
                    module,
                    source: self.format_type(ty),
                    target: interface.name().to_string(),
                };

                error.into()
            }
        };

        self.report(module, diagnostic);
    }

    /// Return a display name for one projected field.
    fn format_field_target(&self, field: dir::FieldTarget) -> String {
        match field {
            dir::FieldTarget::Structural { key, .. } => self.format_static_key(&key),
            dir::FieldTarget::Member { symbol, .. } => self.format_symbol(symbol),
        }
    }

    /// Format one selected readonly member.
    fn format_readonly_member(&self, member: &dir::MemberTarget) -> CompilerResult<String> {
        match member {
            dir::MemberTarget::Field(field) => Ok(self.format_field_target(field.target)),
            dir::MemberTarget::Index(index) => {
                Ok(format!("[{}]", self.format_type(index.key_type)))
            }
            _ => Err(CompilerError::Internal {
                message: format!("readonly diagnostic has non-storage target {member:?}"),
            }),
        }
    }

    /// Format one uncovered pattern value.
    fn format_uncovered_value(&self, value: UncoveredValue) -> String {
        match value {
            UncoveredValue::Type(ty) => self.format_type(ty),
            UncoveredValue::VariantCase { ty, key } => self.format_variant_case(ty, key),
        }
    }

    /// Add a declaration label to a binding diagnostic when the declaration is local.
    fn label_binding_declaration(
        &self,
        diagnostic: DiagnosticBuilder<CheckError>,
        symbol: dir::GlobalSymbolId,
    ) -> DiagnosticBuilder<CheckError> {
        let declaration = self
            .binding_table(symbol.module_id)
            .get_symbol_maybe(symbol.local_id)
            .and_then(|binding| binding.declaration)
            .filter(|declaration| declaration.module_id == symbol.module_id);

        match declaration {
            Some(declaration) => {
                let anchor = self.diagnostic_anchor(symbol.module_id, declaration.local_id);

                diagnostic.label(anchor, "declared here")
            }
            None => diagnostic,
        }
    }

    /// Return the diagnostic for one ordinary relation failure.
    fn constraint_relation_error(
        &self,
        anchor: DiagnosticAnchor,
        module: ModuleId,
        relation: Relation,
        value_use: Option<ValueUse>,
        source: String,
        target: String,
    ) -> CheckError {
        match (relation, value_use) {
            // report equality requirements with their normalized operands
            (Relation::Equal, _) => CheckError::EqualityRequirementNotSatisfied {
                anchor,
                module,
                left: source,
                right: target,
            },
            // report explicit casts with their own failure shape
            (Relation::Castable, _) => CheckError::InvalidCast {
                anchor,
                module,
                source,
                target,
            },
            // report check-only relations by relation kind
            (Relation::Satisfies, _) => CheckError::ConstraintNotSatisfied {
                anchor,
                module,
                source,
                target,
            },
            (Relation::Extends, _) => CheckError::DoesNotExtend {
                anchor,
                module,
                source,
                target,
            },
            // report satisfies checks with their own failure shape
            (_, Some(ValueUse::Satisfies)) => CheckError::ConstraintNotSatisfied {
                anchor,
                module,
                source,
                target,
            },
            // specialize assignability diagnostics by value role
            (_, Some(ValueUse::Condition)) => CheckError::NonBooleanCondition {
                anchor,
                module,
                actual: source,
            },
            (_, Some(ValueUse::Argument | ValueUse::Const)) => CheckError::ArgumentNotAssignable {
                anchor,
                module,
                source,
                target,
            },
            (_, Some(ValueUse::Output)) => CheckError::ReturnNotAssignable {
                anchor,
                module,
                source,
                target,
            },
            (_, None | Some(ValueUse::Store | ValueUse::Operand)) => CheckError::NotAssignable {
                anchor,
                module,
                source,
                target,
            },
        }
    }

    /// Report one failed static evaluation.
    pub(in crate::sema) fn report_static_operation(
        &mut self,
        origin: Origin,
        message: &str,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::InvalidStaticOperation {
            anchor,
            module,
            message: message.to_string(),
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one circular type expansion.
    pub(in crate::sema) fn report_circular_type(&mut self, origin: Origin) -> CompilerResult<()> {
        let error = self.circular_type_error(origin)?;
        let module = origin.module();

        self.report(module, error);

        Ok(())
    }

    /// Report one type instantiation that nests past the followed depth.
    pub(in crate::sema) fn report_excessive_type_instantiation(
        &mut self,
        origin: Origin,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::ExcessiveTypeInstantiation { anchor, module };

        self.report(module, error);

        Ok(())
    }

    /// Report one elided borrow inside a declaration that names its lifetimes.
    pub(in crate::sema) fn report_elided_lifetime_in_named_declaration(
        &mut self,
        declaration: dir::GlobalNodeIdAny,
        site: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.source_anchor(site);
        let symbol = self
            .module(declaration.module_id)
            .declaration_symbol(declaration.local_id);
        let source = match symbol {
            Some(symbol) => self.format_symbol(symbol),
            None => "this declaration".to_string(),
        };

        let error = CheckError::ElidedLifetimeInNamedDeclaration {
            anchor,
            module,
            source,
        };
        self.report(module, error);

        Ok(())
    }

    /// Report one non-local implementation warning.
    pub(in crate::sema) fn report_non_local_implementation(
        &mut self,
        source: dir::GlobalNodeIdAny,
        interface: dir::GlobalSymbolId,
        root: dir::TypeRoot,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let ty = match root {
            dir::TypeRoot::Declaration(symbol) => self.format_symbol(symbol),
            dir::TypeRoot::Primitive(primitive) => primitive.as_str().to_string(),
            dir::TypeRoot::Tuple => "tuple".to_string(),
        };
        let warning = CheckWarning::NonLocalImplementation {
            anchor,
            module,
            interface: self.format_symbol(interface),
            ty,
        };

        self.module_mut(module).warnings.push(warning.into());
    }

    /// Report one blanket implementation for an interface outside the package.
    pub(in crate::sema) fn report_foreign_blanket_implementation(
        &mut self,
        source: dir::GlobalNodeIdAny,
        interface: dir::GlobalSymbolId,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let error = CheckError::ForeignBlanketImplementation {
            anchor,
            module,
            interface: self.format_symbol(interface),
        };

        self.report(module, error);
    }

    /// Report one extension target without a root declaration.
    pub(in crate::sema) fn report_invalid_extension_target(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::InvalidExtensionTarget {
            anchor,
            module,
            ty: self.format_type(ty),
        };
        let diagnostic = DiagnosticBuilder::new(error).note(
            "an extension targets a declaration, a primitive, a tuple, array, slice, or function \
             type, or a bounded type parameter",
        );
        self.report(module, diagnostic);

        Ok(())
    }

    /// Report one blanket extension member outside its declared interfaces.
    pub(in crate::sema) fn report_unanchored_blanket_member(
        &mut self,
        source: dir::GlobalNodeIdAny,
        member: String,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let error = CheckError::UnanchoredBlanketMember {
            anchor,
            module,
            member,
        };

        self.report(module, error);
    }

    /// Report one extension parameter its target and conformances leave unconstrained.
    pub(in crate::sema) fn report_unconstrained_extension_parameter(
        &mut self,
        source: dir::GlobalNodeIdAny,
        parameter: String,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let error = CheckError::UnconstrainedExtensionParameter {
            anchor,
            module,
            parameter,
        };

        self.report(module, error);
    }

    /// Report one exported anonymous extension on a nonlocal target.
    pub(in crate::sema) fn report_unnamed_exported_nonlocal_extension(
        &mut self,
        source: dir::GlobalNodeIdAny,
        target: dir::GlobalTypeId,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let error = CheckError::UnnamedExportedNonlocalExtension {
            anchor,
            module,
            target: self.format_type(target),
        };

        self.report(module, error);
    }

    /// Report one implementation that overlaps an existing implementation.
    pub(in crate::sema) fn report_conflicting_implementation(
        &mut self,
        source: dir::GlobalNodeIdAny,
        conflict: dir::GlobalSymbolId,
        interface: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.source_anchor(source);
        let conflict = self.symbol_source(conflict)?;
        let (_, conflict) = self.source_anchor(conflict);
        let error = CheckError::ConflictingImplementation {
            anchor,
            module,
            interface: self.format_symbol(interface),
            ty: self.format_type(ty),
        };
        let diagnostic =
            DiagnosticBuilder::new(error).label(conflict, "conflicting implementation");

        self.report(module, diagnostic);

        Ok(())
    }

    /// Report one concrete callable declaration without an implementation body.
    pub(in crate::sema) fn report_missing_declaration_body(
        &mut self,
        source: dir::GlobalNodeIdAny,
        member: String,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let error = CheckError::MissingDeclarationBody {
            anchor,
            module,
            name: member,
        };
        self.report(module, error);
    }

    /// Report one interface inheritance clause that names a non-interface symbol.
    pub(in crate::sema) fn report_interface_base_not_interface_symbol(
        &mut self,
        symbol: dir::GlobalSymbolId,
        target: dir::GlobalSymbolId,
        target_source: dir::GlobalNodeIdAny,
    ) {
        let (module, anchor) = self.source_anchor(target_source);
        let error = CheckError::InterfaceBaseNotInterface {
            anchor,
            module,
            source: self.format_symbol(symbol),
            target: self.format_symbol(target),
        };
        self.report(module, error);
    }

    /// Report one interface inheritance clause that names a non-interface type.
    pub(in crate::sema) fn report_interface_base_not_interface_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        target: dir::GlobalTypeId,
        target_source: dir::GlobalNodeIdAny,
    ) {
        let (module, anchor) = self.source_anchor(target_source);
        let error = CheckError::InterfaceBaseNotInterface {
            anchor,
            module,
            source: self.format_symbol(symbol),
            target: self.format_type(target),
        };
        self.report(module, error);
    }

    /// Report one implementation clause that names a non-interface symbol.
    pub(in crate::sema) fn report_implementation_target_not_interface_symbol(
        &mut self,
        source: String,
        target: dir::GlobalSymbolId,
        target_source: dir::GlobalNodeIdAny,
    ) {
        let (module, anchor) = self.source_anchor(target_source);
        let error = CheckError::ImplementationTargetNotInterface {
            anchor,
            module,
            source,
            target: self.format_symbol(target),
        };
        self.report(module, error);
    }

    /// Report one implementation clause that names a non-interface type.
    pub(in crate::sema) fn report_implementation_target_not_interface_type(
        &mut self,
        source: String,
        target: dir::GlobalTypeId,
        target_source: dir::GlobalNodeIdAny,
    ) {
        let (module, anchor) = self.source_anchor(target_source);
        let error = CheckError::ImplementationTargetNotInterface {
            anchor,
            module,
            source,
            target: self.format_type(target),
        };
        self.report(module, error);
    }

    /// Report one class inheritance clause that does not extend its target symbol.
    pub(in crate::sema) fn report_does_not_extend_symbol(
        &mut self,
        source: dir::GlobalTypeId,
        target: dir::GlobalSymbolId,
        target_source: dir::GlobalNodeIdAny,
    ) {
        let (module, anchor) = self.source_anchor(target_source);
        let error = CheckError::DoesNotExtend {
            anchor,
            module,
            source: self.format_type(source),
            target: self.format_symbol(target),
        };
        self.report(module, error);
    }

    /// Report one class inheritance clause that does not extend its target type.
    pub(in crate::sema) fn report_does_not_extend_type(
        &mut self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        target_source: dir::GlobalNodeIdAny,
    ) {
        let (module, anchor) = self.source_anchor(target_source);
        let error = CheckError::DoesNotExtend {
            anchor,
            module,
            source: self.format_type(source),
            target: self.format_type(target),
        };
        self.report(module, error);
    }

    /// Report one where clause bounding no parameter of its declaration.
    pub(in crate::sema) fn report_where_clause_without_parameter(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        let anchor = self.diagnostic_anchor(source.module_id, source.local_id);
        let error = CheckError::WhereClauseWithoutParameter {
            anchor,
            module: source.module_id,
        };
        self.report(source.module_id, error);

        Ok(())
    }

    /// Report one lifetime bound written as a union.
    pub(in crate::sema) fn report_disjunctive_lifetime_bound(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        let anchor = self.diagnostic_anchor(source.module_id, source.local_id);
        let error = CheckError::DisjunctiveLifetimeBound {
            anchor,
            module: source.module_id,
        };
        self.report(source.module_id, error);

        Ok(())
    }

    /// Report one overload an earlier overload of the same owner already accepts.
    pub(in crate::sema) fn report_unreachable_overload(
        &mut self,
        source: dir::GlobalNodeIdAny,
        key: &dir::StaticKey,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let key = self.format_static_key(key);
        let warning = CheckWarning::UnreachableOverload {
            anchor,
            module,
            key,
        };

        self.module_mut(module).warnings.push(warning.into());
    }

    /// Report one repeated definition member.
    pub(in crate::sema) fn report_duplicate_definition_member(
        &mut self,
        source: dir::GlobalNodeIdAny,
        key: &dir::StaticKey,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let member = self.format_static_key(key);
        let error = CheckError::DuplicateMember {
            target: None,
            anchor,
            module,
            member,
        };

        self.report(module, error);
    }

    /// Report one accessor used as a struct field initializer.
    pub(in crate::sema) fn report_invalid_struct_accessor(&mut self, source: dir::GlobalNodeIdAny) {
        let (module, anchor) = self.source_anchor(source);
        let error = CheckError::InvalidStructAccessor { anchor, module };

        self.report(module, error);
    }
}
