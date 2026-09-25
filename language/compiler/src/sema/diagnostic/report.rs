use tspp_artifact::{DiagnosticBuilder, DiagnosticControl};
use tspp_core::{FxIndexSet, NameMatch};
use tspp_dir as dir;
use tspp_source::{
    Applicability, DiagnosticSuggestion, FilePatch, ModuleId, Patch, PatchSet, Span,
};

use crate::sema::materialize::INSTANCE_DEPTH_LIMIT;
use crate::sema::{
    Bound, BoundSide, CauseKind, CheckFailure, CheckState, FailedCheck, MixedObjectSignature,
    ObligationFailure, OperatorOperands, Origin, Pass, Relation, SignatureRejection,
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
        // leave decisions unreported
        if self.infer.is_deciding() {
            return;
        }

        // read the module's diagnostic list
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

    /// Report one construction whose storage the constructor receiver refuses.
    pub(in crate::sema) fn report_construct_receiver_not_assignable(
        &mut self,
        node: dir::GlobalNodeIdAny,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) {
        let module = node.module_id;
        let anchor = self.diagnostic_anchor(module, node.local_id);
        let diagnostic = CheckError::ReceiverNotAssignable {
            anchor,
            module,
            source: self.format_type_at(module, source),
            target: self.format_type_at(module, target),
        };

        self.report(module, diagnostic);
    }

    /// Report one space modifier on a type alias.
    pub(in crate::sema) fn report_space_on_alias(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::SpaceOnAlias { anchor, module };

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

    /// Report a parameter initializer on an ambient signature.
    pub(in crate::sema) fn report_parameter_initializer_outside_implementation(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::ParameterInitializerOutsideImplementation { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a written visibility modifier on one interface member.
    pub(in crate::sema) fn report_interface_member_visibility(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InterfaceMemberVisibility { anchor, module };

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

    /// Report a constructor receiver other than a borrow.
    pub(in crate::sema) fn report_constructor_receiver_not_borrow(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::ConstructorReceiverNotBorrow { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a this expression ahead of the super call.
    pub(in crate::sema) fn report_this_before_super(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::ThisBeforeSuper { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a derived class constructor without a super call.
    pub(in crate::sema) fn report_missing_super_call(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::MissingSuperCall { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a super call outside a derived class constructor.
    pub(in crate::sema) fn report_super_call_outside_constructor(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::SuperCallOutsideConstructor { anchor, module };

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

    /// Report a parking call outside the parking protocol.
    pub(in crate::sema) fn report_park_outside_protocol(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::ParkOutsideProtocol { anchor, module };

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

    /// Report a source that has no iterable implementation.
    pub(in crate::sema) fn report_source_not_iterable(&mut self, source: dir::GlobalNodeIdAny) {
        let (module, anchor) = self.source_anchor(source);
        let diagnostic = CheckError::SourceNotIterable { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report an awaited value that has no awaitable implementation.
    pub(in crate::sema) fn report_source_not_awaitable(&mut self, source: dir::GlobalNodeIdAny) {
        let (module, anchor) = self.source_anchor(source);
        let diagnostic = CheckError::SourceNotAwaitable { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a rest pattern its sequence supports no rest for.
    pub(in crate::sema) fn report_sequence_rest_unsupported(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let diagnostic = CheckError::SequenceRestUnsupported { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report one template span whose value implements no display protocol.
    pub(in crate::sema) fn report_template_span_not_displayable(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let diagnostic = CheckError::TemplateSpanNotDisplayable { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report a using resource that implements no disposal protocol.
    pub(in crate::sema) fn report_using_resource_not_disposable(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let diagnostic = CheckError::UsingResourceNotDisposable { anchor, module };

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

    /// Report a value binding written in type position.
    pub(in crate::sema) fn report_value_used_as_type(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        name: String,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::ValueUsedAsType {
            anchor,
            module,
            name,
        };

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

    /// Report an unresolved reference at one source node, storing its failed path.
    pub(in crate::sema) fn report_unresolved_reference(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
    ) -> CompilerResult<()> {
        // retain the failed path for code actions
        let node = source.into_global(module);
        self.module_mut(module)
            .resolutions
            .set_unresolved_reference(node, path.clone());

        self.emit_unresolved_reference(module, source, path)
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
            parameter: self.parameter_label(parameter)?,
        };

        self.report(module, error);

        Ok(())
    }

    /// Report a declaration read as an ordinary value.
    pub(in crate::sema) fn report_invalid_value_reference(
        &mut self,
        source: dir::GlobalNodeIdAny,
        name: String,
        help: Option<&str>,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let error = CheckError::InvalidValueReference {
            anchor,
            module,
            name,
        };

        // explain the construction form supplied by a type declaration
        let mut diagnostic = DiagnosticBuilder::new(error);
        if let Some(help) = help {
            diagnostic = diagnostic.help(help);
        }

        self.report(module, diagnostic);
    }

    /// Report one overload group referenced without a selecting call.
    pub(in crate::sema) fn report_ambiguous_overload(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbols: &[dir::GlobalSymbolId],
    ) -> CompilerResult<()> {
        let (module, anchor) = self.source_anchor(source);
        let Some(symbol) = symbols.first() else {
            return Err(CompilerError::Internal {
                message: format!("overload reference at {source:?} has no candidates"),
            });
        };
        let error = CheckError::AmbiguousOverload {
            anchor,
            module,
            name: self.format_symbol(*symbol),
        };

        // retain the candidate declarations selected by name resolution
        let mut diagnostic = DiagnosticBuilder::new(error);
        for symbol in symbols.iter().take(4) {
            diagnostic = diagnostic.declaration(*symbol, "one candidate is declared here");
        }

        self.report(module, diagnostic);

        Ok(())
    }

    /// Return one parameter's reported name.
    fn parameter_label(&self, parameter: dir::GlobalGenericParameterId) -> CompilerResult<String> {
        Ok(self
            .generic_parameter(parameter)?
            .and_then(|binding| binding.symbol)
            .map(|symbol| self.format_symbol(symbol))
            .unwrap_or_else(|| "the parameter".to_string()))
    }

    /// Emit one path naming no visible declaration, suggesting the closest name in scope.
    fn emit_unresolved_reference(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
    ) -> CompilerResult<()> {
        let anchor = self.diagnostic_anchor(module, source);
        let name = self.path_label(path);
        let best = self.closest_reference_name(module, source, path);
        let error = CheckError::UnresolvedReference {
            anchor: anchor.clone(),
            module,
            name: name.clone(),
            suggestion: best.as_ref().map(|best| best.candidate.clone()),
        };

        // attach the closest visible name
        let mut diagnostic = DiagnosticBuilder::new(error);
        if let Some(suggestion) = best
            .as_ref()
            .and_then(|best| self.rename_suggestion(&anchor, best))
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        // attach an import candidate
        if let Some(declaration) = self.find_import_candidate(module, path)? {
            diagnostic = diagnostic
                .declaration(declaration, format!("'{name}' is declared here"))
                .help(format!("import '{name}' from its module"));
        }

        self.report(module, diagnostic);

        Ok(())
    }

    /// Report an ambiguous reference at one source node.
    pub(in crate::sema) fn report_ambiguous_reference(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
    ) -> CompilerResult<()> {
        let anchor = self.diagnostic_anchor(module, source);
        let error = CheckError::AmbiguousReference {
            anchor,
            module,
            name: self.path_label(path),
        };

        // read the exact conflicting targets retained by resolution
        let reference = self
            .module(module)
            .resolved
            .references
            .get(source.into_global(module))
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("ambiguous source {source:?} has no resolved reference"),
            })?;

        // point at each live candidate declaration
        let mut diagnostic = DiagnosticBuilder::new(error);
        match reference {
            dir::Reference::Bound(symbols) => {
                for symbol in self.present_symbols(&symbols).into_iter().take(4) {
                    diagnostic = diagnostic.declaration(symbol, "one candidate is declared here");
                }
            }
            dir::Reference::Ambiguous(targets) => {
                for target in targets.into_iter().take(4) {
                    diagnostic = diagnostic.reference(target, "one candidate is declared here");
                }
            }
            reference => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "ambiguous source {source:?} has non-conflicting reference {reference:?}"
                    ),
                });
            }
        }

        self.report(module, diagnostic);

        Ok(())
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
        let diagnostic = DiagnosticBuilder::new(error).declaration(symbol, "declared here");

        self.report(module, diagnostic);
    }

    /// Report a decorator target naming no newtype declaration.
    pub(in crate::sema) fn report_invalid_decorator_target(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidDecoratorTarget { anchor, module };

        self.report(module, diagnostic);
    }

    /// Report decorator arguments proving no unique backing.
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

        // note why each candidate refused
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
            state.source_span(node.local_id),
            state.source_span(value.local_id),
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

    /// Report a repeated array element that cannot be copied.
    pub(in crate::sema) fn report_repeated_element_not_copyable(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let diagnostic = CheckError::RepeatedElementNotCopyable { anchor, module };

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

    /// Report an expression pattern whose value stays open.
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

    /// Report one source occurrence whose type stays open.
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

        // build the diagnostic at the occurrence
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
    ) -> CompilerResult<Vec<(BoundSide, Bound)>> {
        // collect both sides, then cap the list for display
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

    /// Report one source node whose type stays open.
    pub(in crate::sema) fn report_cannot_infer_node(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.source_anchor(source);
        let error = CheckError::CannotInferType { anchor, module };

        self.report(module, error);

        Ok(())
    }

    /// Report one borrow expression asking more access than its source grants.
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

        // name the access the source grants
        let mut diagnostic = DiagnosticBuilder::new(error);
        if let Some(granted) = granted {
            diagnostic = diagnostic.note(format!(
                "the source grants at most '{}' access",
                granted.text()
            ));
        }

        // suggest a source that grants the access
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

        // offer the closest visible name when the access spans an exact key
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

    /// Report one member access denied by its declared visibility.
    pub(in crate::sema) fn report_inaccessible_member(
        &mut self,
        origin: Origin,
        key: String,
        visibility: dir::Visibility,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::InaccessibleMember {
            anchor,
            module,
            key,
            visibility: visibility.label().to_string(),
        };

        self.report(module, error);

        Ok(())
    }

    /// Report one newtype use denied by its declared backing visibility.
    pub(in crate::sema) fn report_inaccessible_newtype_backing(
        &mut self,
        origin: Origin,
        name: String,
        visibility: dir::Visibility,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::InaccessibleNewtypeBacking {
            anchor,
            module,
            name,
            visibility: visibility.label().to_string(),
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

    /// Report one value used as a callable.
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

        // note why each candidate refused
        let mut diagnostic = DiagnosticBuilder::new(error);
        for rejection in rejections {
            diagnostic = diagnostic.note(rejection.clone());
        }

        self.report(module, diagnostic);

        Ok(())
    }

    /// Format why one candidate signature refused an invocation.
    pub(in crate::sema) fn format_signature_rejection(
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

        // note why each candidate refused
        let mut diagnostic = DiagnosticBuilder::new(error);
        for rejection in rejections {
            diagnostic = diagnostic.note(rejection.clone());
        }

        self.report(module, diagnostic);

        Ok(())
    }

    /// Report one argument naming no derivable interface.
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
                self.push_failure(FailedCheck {
                    cause,
                    relation,
                    use_,
                    source,
                    target,
                    failure,
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

        // agree the noun with the count the phrase ends on
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
        form: &str,
        hint: &str,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::NotConstructible {
            anchor,
            module,
            ty: self.format_type(target),
            form: form.to_string(),
            hint: hint.to_string(),
        };

        self.report(module, error);

        Ok(())
    }

    /// Report one declaration kind refused inside a function body.
    pub(in crate::sema) fn report_declaration_not_nestable(
        &mut self,
        origin: Origin,
        kind: &str,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::DeclarationNotNestable {
            anchor,
            module,
            kind: kind.to_string(),
        };

        self.report(module, error);

        Ok(())
    }

    /// Report one inferred construction target outside the concrete newtypes.
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

    /// Report one pattern member that resolves to something other than a field.
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

    /// Report one pattern whose tag names no nominal declaration.
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

    /// Report one variant pattern whose owner differs from the input.
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
        let origin = self.cause_origin(cause);
        let resolved_source = self.deeply_resolve(origin, source)?;
        let resolved_target = self.deeply_resolve(origin, target)?;
        if self.has_error_operand(&[resolved_source, resolved_target])? {
            return Ok(false);
        }

        // report a receiver access shortfall in its own vocabulary
        let root_kind = self.root_cause(cause).kind;
        if root_kind == CauseKind::Receiver
            && let dir::Type::Literal(dir::Literal::String(requested)) = self.ty(resolved_source)?
            && let Some(required) = dir::Access::from_text(self.strings().get(requested))
            && let dir::Type::Literal(dir::Literal::String(taken)) = self.ty(resolved_target)?
            && let Some(granted) = dir::ReceiverMode::from_text(self.strings().get(taken))
        {
            self.report_receiver_access_not_granted(origin, required, granted)?;

            return Ok(true);
        }

        // anchor the diagnostic at the site the cause names
        let value_use = value_use.or(match root_kind {
            CauseKind::Argument { .. } => Some(ValueUse::Argument),
            CauseKind::Return { .. } | CauseKind::ReturnSlot => Some(ValueUse::Output),
            CauseKind::Initializer { .. }
            | CauseKind::Field { .. }
            | CauseKind::Element { .. }
            | CauseKind::Write { .. } => Some(ValueUse::Store),
            _ => None,
        });
        let failure = match failure {
            CheckFailure::Relation if self.is_dynamic_safe_target(target)? => {
                CheckFailure::NotErasable
            }
            failure => failure,
        };
        let (module, anchor) = match (root_kind, value_use) {
            // anchor converted values at their whole node
            (_, Some(_)) => self.origin_node_anchor(origin)?,
            // anchor every other cause at its diagnostic site
            _ => self.origin_diagnostic_anchor(origin)?,
        };

        // re-walk failed closed relations to their mismatched leaf
        let blame = match failure {
            CheckFailure::Relation => self.blame_relation(origin, relation, source, target)?,
            _ => None,
        };

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
                DiagnosticBuilder::new(error)
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
            // the nested check already reported the failure
            CheckFailure::Reported => return Ok(false),
        };

        // explain the cause chain and report the failure once
        let diagnostic = self.format_cause(diagnostic, cause, &anchor, blame.as_ref())?;

        self.report(module, diagnostic);

        Ok(true)
    }

    /// Report one failed obligation.
    pub(in crate::sema) fn report_obligation_failure(
        &mut self,
        failure: ObligationFailure,
    ) -> CompilerResult<()> {
        match failure {
            ObligationFailure::InvalidRestParameter { source, ty } => {
                let (module, anchor) = self.source_anchor(source);
                let ty = self.format_type(ty);
                let error = CheckError::InvalidRestParameter { anchor, module, ty };

                self.report(module, error);
            }
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
                let diagnostic = DiagnosticBuilder::new(error).declaration(symbol, "declared here");

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
                let diagnostic = DiagnosticBuilder::new(error).declaration(symbol, "declared here");

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
                witness,
            } => {
                self.report_conflicting_implementation(source, conflict, interface, witness)?;
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
            ObligationFailure::ConflictingHeritageSpace {
                source,
                symbol,
                conflict_source,
                conflict,
            } => {
                let (module, anchor) = self.source_anchor(source);
                let (_, conflict_anchor) = self.source_anchor(conflict_source);
                let error = CheckError::HeritageSpaceConflict { anchor, module };
                let diagnostic = DiagnosticBuilder::new(error)
                    .label(conflict_anchor, "conflicting space")
                    .declaration(
                        symbol,
                        format!("'{}' is declared here", self.format_symbol(symbol)),
                    )
                    .declaration(
                        conflict,
                        format!("'{}' is declared here", self.format_symbol(conflict)),
                    )
                    .help("make every base and implemented interface use the same space");

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
            ObligationFailure::StaticFieldMissingInitializer { source, field } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::StaticFieldMissingInitializer {
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
            | dir::AutoInterface::Drop
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

    /// Report one callable body requiring more receiver access than its slot takes.
    pub(in crate::sema) fn report_receiver_access_not_granted(
        &mut self,
        origin: Origin,
        required: dir::Access,
        granted: dir::ReceiverMode,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::ReceiverAccessNotGranted {
            anchor,
            module,
            access: required.text().to_string(),
            granted: granted.text().to_string(),
        };

        self.report(module, DiagnosticBuilder::new(error));

        Ok(())
    }

    /// Format one uncovered pattern value.
    fn format_uncovered_value(&self, value: UncoveredValue) -> String {
        match value {
            UncoveredValue::Type(ty) => self.format_type(ty),
            UncoveredValue::VariantCase { ty, key } => self.format_variant_case(ty, key),
        }
    }

    /// Return whether one relation target is the dynamic safety interface.
    fn is_dynamic_safe_target(&mut self, target: dir::GlobalTypeId) -> CompilerResult<bool> {
        let target = self.shallow_resolve(target)?;
        let Some((_, instance)) = self.nominal_application_maybe(target)? else {
            return Ok(false);
        };
        let interface = self
            .language_item(instance.symbol)?
            .and_then(dir::AutoInterface::from_language_item);

        Ok(interface == Some(dir::AutoInterface::DynamicSafe))
    }

    /// Return the diagnostic one relation failure takes from its role.
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
            (_, Some(ValueUse::Cast)) => CheckError::InvalidCast {
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
            // report bound requirements and predicates as constraints
            (Relation::Subtype, None | Some(ValueUse::Operand | ValueUse::Const)) => {
                CheckError::ConstraintNotSatisfied {
                    anchor,
                    module,
                    source,
                    target,
                }
            }
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

    /// Report one template literal type that expands past the member bound.
    pub(in crate::sema) fn report_template_literal_too_complex(
        &mut self,
        origin: Origin,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::TemplateLiteralTooComplex { anchor, module };

        self.report(module, error);

        Ok(())
    }

    /// Report one type instantiation that nests past the followed depth.
    pub(in crate::sema) fn report_excessive_type_instantiation(
        &mut self,
        origin: Origin,
    ) -> CompilerResult<()> {
        // report once, from the checking pass alone
        if self.pass != Pass::Check {
            return Ok(());
        }
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

        // report the elision against the declaration that names its lifetimes
        let error = CheckError::ElidedLifetimeInNamedDeclaration {
            anchor,
            module,
            source,
        };

        self.report(module, error);

        Ok(())
    }

    /// Report one instantiation chain reaching past the depth limit.
    pub(in crate::sema) fn report_instantiation_depth_exceeded(
        &mut self,
        site: dir::GlobalNodeIdAny,
        template: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.source_anchor(site);
        let error = CheckError::InstantiationDepthExceeded {
            anchor,
            module,
            source: self.format_symbol(template),
            limit: INSTANCE_DEPTH_LIMIT,
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
        witness: String,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.source_anchor(source);
        let error = CheckError::ConflictingImplementation {
            anchor,
            module,
            interface: self.format_symbol(interface),
            ty: witness,
        };
        let diagnostic =
            DiagnosticBuilder::new(error).declaration(conflict, "conflicting implementation");

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

    /// Report one negative implementation clause that names an interface without a compiler rule.
    pub(in crate::sema) fn report_negative_implementation_not_auto(
        &mut self,
        source: String,
        target: dir::GlobalSymbolId,
        target_source: dir::GlobalNodeIdAny,
    ) {
        let (module, anchor) = self.source_anchor(target_source);
        let error = CheckError::NegativeImplementationNotAuto {
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

    /// Report one class inheritance clause that misses its target symbol.
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

    /// Report one class inheritance clause that misses its target type.
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
