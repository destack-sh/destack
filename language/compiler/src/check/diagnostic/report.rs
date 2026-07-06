use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;

use crate::check::{
    CheckState, ConstraintFailure, ObligationFailure, OperatorOperands, Origin, Relation,
    SignatureRejection, UncoveredValue, ValueUse,
};
use crate::{CheckError, CheckWarning, CompilerResult, DiagnosticAnchor};

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

    /// Report a try propagation outside a function body.
    pub(in crate::check) fn report_try_outside_function(&mut self, source: dir::GlobalNodeIdAny) {
        let (module, anchor) = self.source_anchor(source);
        let diagnostic = CheckError::TryOutsideFunction { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report a return type that cannot accept a propagated try failure.
    pub(in crate::check) fn report_try_propagation_not_implemented(
        &mut self,
        source: dir::GlobalNodeIdAny,
        value: dir::GlobalTypeId,
        return_type: dir::GlobalTypeId,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let source = self.format_type(return_type);
        let target = format!("FromResidual<{}>", self.format_type(value));
        let error = CheckError::InterfaceNotImplemented {
            anchor,
            module,
            source,
            target,
        };

        self.module_mut(module).diagnostics.push(
            error.note("the '?' operator propagates failures into the enclosing return type"),
        );
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

    /// Report a for-of source that has no iterable implementation.
    pub(in crate::check) fn report_for_of_source_not_iterable(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let diagnostic = CheckError::ForOfSourceNotIterable { anchor, module };

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

    /// Report a static guard that cannot decide statically.
    pub(in crate::check) fn report_undecidable_static_guard(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::UndecidableStaticCondition { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());
    }

    /// Report a static value expression that cannot decide statically.
    pub(in crate::check) fn report_undecidable_static_value(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::UndecidableStaticValue { anchor, module };

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

    /// Report a local binding read before assignment.
    pub(in crate::check) fn report_use_before_assigned(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::UseBeforeAssigned {
            anchor,
            module,
            name: self.format_symbol(symbol),
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

    /// Report an invalid `typeof` type query operand.
    pub(in crate::check) fn report_invalid_type_query(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidTypeQuery { anchor, module };

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

    /// Report a tree expression without an active builder.
    pub(in crate::check) fn report_missing_tree_builder(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::MissingTreeBuilder { anchor, module };

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

    /// Report one source occurrence whose type could not be inferred.
    pub(in crate::check) fn report_cannot_infer_type(
        &mut self,
        origin: Origin,
        reported: &mut IndexSet<(ModuleId, DiagnosticAnchor)>,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        if self.modules.contains_key(&module) && reported.insert((module, anchor.clone())) {
            let error = CheckError::CannotInferType { anchor, module };
            self.module_mut(module).diagnostics.push(error.into());
        }

        Ok(())
    }

    /// Report one source node whose type could not be inferred.
    pub(in crate::check) fn report_cannot_infer_node(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.source_anchor(source);
        let error = CheckError::CannotInferType { anchor, module };
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one missing member with the closest visible suggestion.
    pub(in crate::check) fn report_missing_member(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: String,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let receiver_text = self.format_type(receiver);
        let suggestion = self.closest_member_key(receiver, &key)?;
        let error = CheckError::MissingMember {
            anchor,
            module,
            key,
            receiver: receiver_text,
            suggestion,
        };
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one member access whose target is overloaded.
    pub(in crate::check) fn report_ambiguous_member(
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
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one read through a write-only member.
    pub(in crate::check) fn report_write_only_member(
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
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one write through a readonly member.
    pub(in crate::check) fn report_readonly_member(
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
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one value that is not callable.
    pub(in crate::check) fn report_not_callable(
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
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one call whose arguments match no overload.
    pub(in crate::check) fn report_no_matching_call(
        &mut self,
        origin: Origin,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::NoMatchingCall {
            anchor,
            module,
            arguments: self.format_types(arguments),
        };
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one construction whose arguments match no constructor.
    pub(in crate::check) fn report_no_matching_construct(
        &mut self,
        origin: Origin,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::NoMatchingConstruct {
            anchor,
            module,
            arguments: self.format_types(arguments),
        };
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one call with the wrong argument count.
    pub(in crate::check) fn report_wrong_argument_count(
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
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one final signature rejection.
    pub(in crate::check) fn report_signature_rejection(
        &mut self,
        origin: Origin,
        module: ModuleId,
        arguments: &[dir::LocalNodeId<dir::Argument>],
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

            // report argument mismatch on the failing value
            SignatureRejection::Argument {
                index,
                source,
                target,
            } => {
                let origin = arguments
                    .get(index)
                    .and_then(|argument| self.argument_value_node(module, *argument))
                    .map(|node| self.origin_at(origin, node))
                    .unwrap_or(origin);
                let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
                let error = CheckError::ArgumentNotAssignable {
                    anchor,
                    module,
                    source: self.format_type_at(module, source),
                    target: self.format_type_at(module, target),
                };
                self.module_mut(module).diagnostics.push(error.into());
            }

            // report generic bound mismatch on the supplied or inferred argument source
            SignatureRejection::Bound {
                source_node,
                source,
                target,
            } => {
                let (module, anchor) =
                    self.origin_diagnostic_anchor(self.origin_at(origin, source_node))?;
                let error = CheckError::ConstraintNotSatisfied {
                    anchor,
                    module,
                    source: self.format_type_at(module, source),
                    target: self.format_type_at(module, target),
                };
                self.module_mut(module).diagnostics.push(error.into());
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
                self.module_mut(module).diagnostics.push(error.into());
            }

            // report missing writable index support on the supplied or inferred argument source
            SignatureRejection::WritableIndex {
                source_node,
                source,
                key,
                value,
            } => {
                let (module, anchor) =
                    self.origin_diagnostic_anchor(self.origin_at(origin, source_node))?;
                let error = CheckError::WritableIndexRequiresIndexSet {
                    anchor,
                    module,
                    source: self.format_type_at(module, source),
                    key: self.format_type_at(module, key),
                    value: self.format_type_at(module, value),
                };
                self.module_mut(module).diagnostics.push(error.into());
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
    pub(in crate::check) fn report_cannot_construct_abstract_type(
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
        self.module_mut(module).diagnostics.push(diagnostic);

        Ok(())
    }

    /// Report one value that cannot be constructed with `new`.
    pub(in crate::check) fn report_not_constructible(
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
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one inferred construction target that is not a concrete newtype.
    pub(in crate::check) fn report_invalid_inferred_construct_target(
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
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one member read through a possibly nullish value.
    pub(in crate::check) fn report_possibly_nullish(
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
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one operator application that matches no overload.
    pub(in crate::check) fn report_no_matching_operator(
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
        self.module_mut(module).diagnostics.push(error.into());

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
    pub(in crate::check) fn report_spread_not_object(
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
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one sequence pattern with a non-sequence source.
    pub(in crate::check) fn report_pattern_source_not_sequence_shaped(
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
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one rest pattern that appears before another field.
    pub(in crate::check) fn report_rest_pattern_not_last(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let error = CheckError::RestPatternNotLast { anchor, module };

        self.module_mut(module).diagnostics.push(error.into());
    }

    /// Report one extra rest pattern in the same field list.
    pub(in crate::check) fn report_multiple_rest_patterns(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let error = CheckError::MultipleRestPatterns { anchor, module };

        self.module_mut(module).diagnostics.push(error.into());
    }

    /// Report one object pattern with a non-object source.
    pub(in crate::check) fn report_pattern_source_not_object_shaped(
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
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one tuple pattern with a non-tuple source.
    pub(in crate::check) fn report_pattern_source_not_tuple_shaped(
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
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one missing pattern field.
    pub(in crate::check) fn report_pattern_field_missing(
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
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one pattern member that is not a field.
    pub(in crate::check) fn report_pattern_member_not_field(
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
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one repeated pattern field.
    pub(in crate::check) fn report_duplicate_pattern_field(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        key: String,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let error = CheckError::DuplicatePatternField {
            anchor,
            module,
            key,
        };

        self.module_mut(module).diagnostics.push(error.into());
    }

    /// Report one repeated pattern binding.
    pub(in crate::check) fn report_duplicate_pattern_binding(
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

        self.module_mut(module).diagnostics.push(error.into());
    }

    /// Report one repeated definition member.
    pub(in crate::check) fn report_duplicate_definition_member(
        &mut self,
        source: dir::GlobalNodeIdAny,
        key: &dir::StaticKey,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let member = self.format_static_key(key);
        let error = CheckError::DuplicateMember {
            anchor,
            module,
            member,
        };

        self.module_mut(module).diagnostics.push(error.into());
    }

    /// Report one computed pattern key that cannot select a field.
    pub(in crate::check) fn report_computed_pattern_key_not_valid(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let error = CheckError::ComputedPatternKeyNotValid { anchor, module };

        self.module_mut(module).diagnostics.push(error.into());
    }

    /// Report one pattern whose tag is not nominal.
    pub(in crate::check) fn report_invalid_pattern_tag(
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
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one variant pattern whose owner does not match the input.
    pub(in crate::check) fn report_pattern_variant_not_in_type(
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
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one variant pattern that names no case on its owner.
    pub(in crate::check) fn report_pattern_variant_missing(
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
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one impossible strict equality comparison.
    pub(in crate::check) fn report_invalid_strict_equality(
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
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report an `instanceof` target that cannot select one class declaration.
    pub(in crate::check) fn report_instanceof_target_not_class(
        &mut self,
        target: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        let module = target.module_id;
        let anchor = self.diagnostic_anchor(module, target.local_id);
        let error = CheckError::InstanceOfTargetNotClass { anchor, module };
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report a runtime predicate target that has no executable representation.
    pub(in crate::check) fn report_runtime_predicate_not_testable(
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
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one failed closed constraint.
    pub(in crate::check) fn report_constraint_failure(
        &mut self,
        origin: Origin,
        relation: Relation,
        value_use: Option<ValueUse>,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
        failure: ConstraintFailure,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let source = self.format_type_at(module, left);
        let target = self.format_type_at(module, right);

        // translate the selected failure reason
        match failure {
            ConstraintFailure::Relation => {
                let error = self
                    .constraint_relation_error(anchor, module, relation, value_use, source, target);
                self.module_mut(module).diagnostics.push(error.into());
            }
            ConstraintFailure::MissingRequiredProperty { key } => {
                let error = CheckError::MissingRequiredProperty {
                    anchor,
                    module,
                    key: self.format_static_key(&key),
                    target,
                };
                self.module_mut(module).diagnostics.push(error.into());
            }
            ConstraintFailure::ExcessProperty { key } => {
                let error = CheckError::ExcessProperty {
                    anchor,
                    module,
                    key: self.format_static_key(&key),
                    target,
                };
                let diagnostic = DiagnosticBuilder::new(error)
                    .note("object literals may only specify known properties");
                self.module_mut(module).diagnostics.push(diagnostic);
            }
            ConstraintFailure::WritableIndexRequiresIndexSet { signature } => {
                let error = CheckError::WritableIndexRequiresIndexSet {
                    anchor,
                    module,
                    source,
                    key: self.format_type_at(module, signature.key_type),
                    value: self.format_type_at(module, signature.value_type),
                };
                self.module_mut(module).diagnostics.push(error.into());
            }
        }

        Ok(())
    }

    /// Report one failed obligation.
    pub(in crate::check) fn report_obligation_failure(
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
                self.module_mut(module).diagnostics.push(diagnostic);
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
                self.module_mut(module).diagnostics.push(diagnostic);
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
                self.module_mut(module).diagnostics.push(diagnostic);
            }
            ObligationFailure::ForInSourceNotObjectShaped { source } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::ForInSourceNotObjectShaped { anchor, module };
                self.module_mut(module).diagnostics.push(error.into());
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
                self.module_mut(module).diagnostics.push(error.into());
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
                self.module_mut(module).diagnostics.push(error.into());
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
                self.module_mut(module).diagnostics.push(error.into());
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
                self.module_mut(module).diagnostics.push(diagnostic);
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
                self.module_mut(module).diagnostics.push(diagnostic);
            }
            ObligationFailure::CannotAssignReadonlyMember { source, member } => {
                let (module, anchor) = self.source_anchor(source);
                let member = self.format_projection_field(member);
                let error = CheckError::CannotAssignReadonlyMember {
                    anchor,
                    module,
                    member,
                };
                self.module_mut(module).diagnostics.push(error.into());
            }
            ObligationFailure::OverwriteStabilityNotSatisfied { source, ty } => {
                let (module, anchor) = self.source_anchor(source);
                let ty = self.format_type(ty);
                let error = CheckError::OverwriteStabilityNotSatisfied { anchor, module, ty };
                self.module_mut(module).diagnostics.push(error.into());
            }
            ObligationFailure::CircularType { source } => {
                let error = self.circular_type_error(Origin::Node(source, None))?;
                self.module_mut(source.module_id)
                    .diagnostics
                    .push(error.into());
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
                    target: self.format_symbol(interface),
                };
                self.module_mut(module).diagnostics.push(error.into());
            }
            ObligationFailure::NonLocalImplementation {
                source,
                interface,
                ty,
            } => {
                self.report_non_local_implementation(source, interface, ty);
            }
            ObligationFailure::ForeignBlanketImplementation { source, interface } => {
                self.report_foreign_blanket_implementation(source, interface);
            }
            ObligationFailure::UnnamedExportedNonlocalExtension { source, target } => {
                self.report_unnamed_exported_nonlocal_extension(source, target);
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
                self.module_mut(module).diagnostics.push(error.into());
            }
            ObligationFailure::CircularHeritage { source, symbol } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::CircularHeritage {
                    anchor,
                    module,
                    source: self.format_symbol(symbol),
                };
                self.module_mut(module).diagnostics.push(error.into());
            }
            ObligationFailure::InvalidOverride { source, member } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::InvalidOverride {
                    anchor,
                    module,
                    member: self.format_static_key(&member),
                };
                self.module_mut(module).diagnostics.push(error.into());
            }
            ObligationFailure::OverrideNotVirtual { source, member } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::OverrideNotVirtual {
                    anchor,
                    module,
                    member: self.format_static_key(&member),
                };
                let diagnostic = error.help("declare the inherited member 'virtual' or 'abstract'");
                self.module_mut(module).diagnostics.push(diagnostic);
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
                self.module_mut(module).diagnostics.push(error.into());
            }
            ObligationFailure::MissingOverride { source, member } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::MissingOverride {
                    anchor,
                    module,
                    member: self.format_static_key(&member),
                };
                let diagnostic = error.help("add the 'override' modifier");
                self.module_mut(module).diagnostics.push(diagnostic);
            }
            ObligationFailure::AbstractMemberInConcreteClass { source, member } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::AbstractMemberInConcreteClass {
                    anchor,
                    module,
                    member: self.format_static_key(&member),
                };
                self.module_mut(module).diagnostics.push(error.into());
            }
            ObligationFailure::UnimplementedAbstractMember { source, member } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::UnimplementedAbstractMember {
                    anchor,
                    module,
                    member: self.format_static_key(&member),
                };
                let diagnostic = error.help("implement the member or declare the class 'abstract'");
                self.module_mut(module).diagnostics.push(diagnostic);
            }
            ObligationFailure::FinalClassExtended { source, base } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::FinalClassExtended {
                    anchor,
                    module,
                    ty: self.format_symbol(base),
                };
                self.module_mut(module).diagnostics.push(error.into());
            }
            ObligationFailure::FieldNotDefinitelyInitialized { source, field } => {
                let (module, anchor) = self.source_anchor(source);
                let error = CheckError::FieldNotDefinitelyInitialized {
                    anchor,
                    module,
                    field: self.format_symbol(field),
                };
                self.module_mut(module).diagnostics.push(error.into());
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
        let diagnostic = match interface {
            dir::AutoInterface::DynamicSafe => {
                let ty = self.format_type(ty);
                let error = CheckError::DynamicSafetyNotSatisfied { anchor, module, ty };

                error.into()
            }
            dir::AutoInterface::OverwriteStable => {
                let ty = self.format_type(ty);
                let error = CheckError::OverwriteStabilityNotSatisfied { anchor, module, ty };

                error.into()
            }
            dir::AutoInterface::Integer
            | dir::AutoInterface::Float
            | dir::AutoInterface::Concrete
            | dir::AutoInterface::Copy
            | dir::AutoInterface::Clone
            | dir::AutoInterface::Debug
            | dir::AutoInterface::Default
            | dir::AutoInterface::Hash
            | dir::AutoInterface::Equal
            | dir::AutoInterface::PartialEqual
            | dir::AutoInterface::Compare
            | dir::AutoInterface::PartialCompare
            | dir::AutoInterface::Serialize
            | dir::AutoInterface::Deserialize
            | dir::AutoInterface::Send
            | dir::AutoInterface::Sync
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

        self.module_mut(module).diagnostics.push(diagnostic);
    }

    /// Return a display name for one projected field.
    fn format_projection_field(&self, field: dir::ProjectionField) -> String {
        match field {
            dir::ProjectionField::Key(key) => self.format_static_key(&key),
            dir::ProjectionField::Member(symbol) => self.format_symbol(symbol),
        }
    }

    /// Format one uncovered pattern value.
    fn format_uncovered_value(&self, value: UncoveredValue) -> String {
        match value {
            UncoveredValue::Type(ty) => self.format_type(ty),
            UncoveredValue::TaggedCase { ty, key } => self.format_variant_case(ty, key),
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
            // explicit casts report their own failure shape
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
            (Relation::Implements, _) => CheckError::InterfaceNotImplemented {
                anchor,
                module,
                source,
                target,
            },
            // value roles specialize assignability diagnostics
            (_, Some(ValueUse::Condition)) => CheckError::NonBooleanCondition {
                anchor,
                module,
                actual: source,
            },
            (_, Some(ValueUse::Argument)) => CheckError::ArgumentNotAssignable {
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
            (_, None | Some(ValueUse::Store)) => CheckError::NotAssignable {
                anchor,
                module,
                source,
                target,
            },
        }
    }

    /// Report one failed static evaluation.
    pub(in crate::check) fn report_static_operation(
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
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one circular type expansion.
    pub(in crate::check) fn report_circular_type(&mut self, origin: Origin) -> CompilerResult<()> {
        let error = self.circular_type_error(origin)?;
        let module = origin.module();

        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Report one non-local implementation warning.
    pub(in crate::check) fn report_non_local_implementation(
        &mut self,
        source: dir::GlobalNodeIdAny,
        interface: dir::GlobalSymbolId,
        ty: dir::GlobalSymbolId,
    ) {
        let (module, anchor) = self.source_anchor(source);
        let warning = CheckWarning::NonLocalImplementation {
            anchor,
            module,
            interface: self.format_symbol(interface),
            ty: self.format_symbol(ty),
        };

        self.module_mut(module).warnings.push(warning.into());
    }

    /// Report one blanket implementation for an interface outside the package.
    pub(in crate::check) fn report_foreign_blanket_implementation(
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

        self.module_mut(module).diagnostics.push(error.into());
    }

    /// Report one exported anonymous extension on a nonlocal target.
    pub(in crate::check) fn report_unnamed_exported_nonlocal_extension(
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

        self.module_mut(module).diagnostics.push(error.into());
    }

    /// Report one implementation that overlaps an existing implementation.
    pub(in crate::check) fn report_conflicting_implementation(
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

        self.module_mut(module).diagnostics.push(diagnostic);

        Ok(())
    }

    /// Report one concrete callable declaration without an implementation body.
    pub(in crate::check) fn report_missing_declaration_body(
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
        self.module_mut(module).diagnostics.push(error.into());
    }

    /// Report one interface inheritance clause that names a non-interface symbol.
    pub(in crate::check) fn report_interface_base_not_interface_symbol(
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
        self.module_mut(module).diagnostics.push(error.into());
    }

    /// Report one interface inheritance clause that names a non-interface type.
    pub(in crate::check) fn report_interface_base_not_interface_type(
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
        self.module_mut(module).diagnostics.push(error.into());
    }

    /// Report one implementation clause that names a non-interface symbol.
    pub(in crate::check) fn report_implementation_target_not_interface_symbol(
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
        self.module_mut(module).diagnostics.push(error.into());
    }

    /// Report one implementation clause that names a non-interface type.
    pub(in crate::check) fn report_implementation_target_not_interface_type(
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
        self.module_mut(module).diagnostics.push(error.into());
    }

    /// Report one class inheritance clause that does not extend its target symbol.
    pub(in crate::check) fn report_does_not_extend_symbol(
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
        self.module_mut(module).diagnostics.push(error.into());
    }

    /// Report one class inheritance clause that does not extend its target type.
    pub(in crate::check) fn report_does_not_extend_type(
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
        self.module_mut(module).diagnostics.push(error.into());
    }
}
