use destack_artifact::DiagnosticBuilder;
use destack_core::closest_string;
use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;
use smallvec::SmallVec;

use crate::check::{CheckState, Origin, Relation, ValueUse};
use crate::{
    CheckError, CheckWarning, CompilerError, CompilerResult, DiagnosticAnchor,
    diagnostic_suggestion_distance,
};

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

    /// Report one static condition that did not reduce to a boolean literal.
    pub(in crate::check) fn report_invalid_static_condition(
        &mut self,
        origin: Origin,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let diagnostic = CheckError::InvalidStaticCondition { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic.into());

        Ok(())
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

    /// Report a final output type that still references inference variables.
    pub(in crate::check) fn report_unresolved_output_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        reported: &mut IndexSet<(ModuleId, DiagnosticAnchor)>,
    ) -> CompilerResult<()> {
        if self.type_variables(ty)?.is_empty() {
            return Ok(());
        }

        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        if self.modules.contains_key(&module) && reported.insert((module, anchor.clone())) {
            let error = CheckError::MissingTypeAnnotation { anchor, module };
            self.module_mut(module).diagnostics.push(error.into());
        }

        Ok(())
    }

    /// Report every heritage error except the first returned error.
    pub(in crate::check) fn report_heritage_errors(
        &mut self,
        source: dir::GlobalNodeIdAny,
        errors: Vec<DiagnosticBuilder<CheckError>>,
    ) -> Option<DiagnosticBuilder<CheckError>> {
        let mut errors = errors.into_iter();
        let first = errors.next();
        for error in errors {
            self.module_mut(source.module_id).diagnostics.push(error);
        }

        first
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
        note: Option<String>,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::NoMatchingCall {
            anchor,
            module,
            arguments: self.format_types(arguments),
        };
        let diagnostic = match note {
            Some(note) => error.note(note),
            None => error.into(),
        };
        self.module_mut(module).diagnostics.push(diagnostic);

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
        operands: String,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::NoMatchingOperator {
            anchor,
            module,
            operator,
            operands,
        };
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
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

    /// Report one failed closed relation.
    pub(in crate::check) fn report_relation_failure(
        &mut self,
        origin: Origin,
        relation: Relation,
        value_use: Option<ValueUse>,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let source = self.format_type(left);
        let target = self.format_type(right);

        // label the written annotation that demanded the target type
        let written = match self.written_type_anchor(right)? {
            Some(written) if written != anchor => {
                Some((written, format!("expected '{target}' from this annotation")))
            }
            _ => None,
        };

        // object literals name their excess property directly
        if matches!(
            relation,
            Relation::Assignable | Relation::Writable | Relation::Satisfies
        ) && matches!(
            value_use,
            Some(ValueUse::Store | ValueUse::Argument | ValueUse::Output)
        ) && let Some(key) = self.object_literal_excess_property(origin, left, right)?
        {
            let error = CheckError::ExcessProperty {
                anchor,
                module,
                key,
                target,
            };
            let mut diagnostic = DiagnosticBuilder::new(error)
                .note("object literals may only specify known properties");
            if let Some((written, label)) = written {
                diagnostic = diagnostic.label(written, label);
            }
            self.module_mut(module).diagnostics.push(diagnostic);

            return Ok(());
        }

        let error = match (relation, value_use) {
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
        };
        let mut diagnostic = DiagnosticBuilder::new(error);
        if let Some((written, label)) = written {
            diagnostic = diagnostic.label(written, label);
        }
        self.module_mut(module).diagnostics.push(diagnostic);

        Ok(())
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

    /// Return the diagnostic anchor for one source node.
    pub(in crate::check) fn diagnostic_anchor(
        &self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) -> DiagnosticAnchor {
        let span = match self.modules.get(&module) {
            Some(state) => state.diagnostic_span(source),
            None => self.external_module(module).diagnostic_span(source),
        };
        let span = match span {
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

    /// Return one symbol's declaration node.
    pub(in crate::check) fn symbol_source(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalNodeIdAny> {
        let source = match self.modules.get(&symbol.module_id) {
            Some(module) => module
                .symbol_declaration_node(symbol.local_id)?
                .into_global(symbol.module_id),
            None => {
                let external = self.external_module(symbol.module_id);
                let binding = external.bindings.get_symbol(symbol.local_id);
                let Some(declaration) = binding.declaration else {
                    return Err(CompilerError::Internal {
                        message: format!("external symbol {symbol:?} has no declaration node"),
                    });
                };

                declaration
            }
        };

        Ok(source)
    }

    /// Return a circular type diagnostic for one origin.
    pub(in crate::check) fn circular_type_error(
        &self,
        origin: Origin,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;

        Ok(CheckError::CircularType { anchor, module })
    }

    /// Return the visible member key closest to one missing key.
    fn closest_member_key(
        &mut self,
        receiver: dir::GlobalTypeId,
        key: &str,
    ) -> CompilerResult<Option<String>> {
        let keys = self.visible_member_keys(receiver)?;

        Ok(closest_string(
            key,
            keys,
            diagnostic_suggestion_distance(key),
        ))
    }

    /// Collect the member keys visible on one receiver.
    fn visible_member_keys(&mut self, receiver: dir::GlobalTypeId) -> CompilerResult<Vec<String>> {
        let mut current = self.settled_root(receiver)?;
        while let dir::Type::Form(form) = self.ty(current)? {
            current = self.settled_root(form.value)?;
        }

        let mut keys = Vec::new();
        match self.ty(current)? {
            dir::Type::Shape(shape) => {
                for field in &shape.fields {
                    keys.push(self.format_static_key(&field.key));
                }
            }
            dir::Type::Reference(reference) => {
                if let Some(definition) = self.definition(reference.symbol) {
                    for member in definition.members() {
                        if member.space() == dir::MemberSpace::Static
                            && let Some(key) = member.key()
                        {
                            keys.push(self.format_static_key(&key));
                        }
                    }
                }
            }
            dir::Type::Instance(instance) => {
                if let Some(definition) = self.definition(instance.symbol) {
                    for member in definition.members() {
                        if let Some(key) = member.key() {
                            keys.push(self.format_static_key(&key));
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(keys)
    }

    /// Return the source anchor of one type written as an annotation.
    fn written_type_anchor(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<DiagnosticAnchor>> {
        let id = self.settled_root(id)?;

        let Some(module) = self.modules.get(&id.module_id) else {
            return Ok(None);
        };
        if module.types.get_type_maybe(id.local_id).is_some() {
            return Ok(None);
        }

        let source = module.types.get_type_source(id.local_id);

        Ok(Some(self.diagnostic_anchor(id.module_id, source)))
    }

    /// Return the first excess property one object literal supplies to one target.
    pub(in crate::check) fn object_literal_excess_property(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Option<String>> {
        let Some(expression) = origin.expression() else {
            return Ok(None);
        };
        if !matches!(
            self.module(expression.module_id)
                .view()
                .get(expression.local_id),
            dir::Expression::ObjectExpression { .. }
        ) {
            return Ok(None);
        }

        let left = self.settled_root(left)?;
        let left = match self.ty(left)? {
            dir::Type::Form(form) if form.form == dir::Form::Managed => {
                self.settled_root(form.value)?
            }
            _ => left,
        };
        let dir::Type::Shape(shape) = self.ty(left)? else {
            return Ok(None);
        };
        let keys = shape
            .fields
            .iter()
            .map(|field| field.key)
            .collect::<SmallVec<[_; 8]>>();

        let Some(right) = self.reduce_type_root(origin, right)?.ready() else {
            return Ok(None);
        };
        let Some(accepted) = self.accepted_property_keys(origin, right)? else {
            return Ok(None);
        };

        for key in keys {
            if !accepted.contains(&key) {
                return Ok(Some(self.format_static_key(&key)));
            }
        }

        Ok(None)
    }

    /// Collect the property keys one target accepts, none when it accepts any.
    fn accepted_property_keys(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[dir::StaticKey; 8]>>> {
        match self.ty(target)? {
            dir::Type::Shape(shape) => {
                if !shape.index_signatures.is_empty() {
                    return Ok(None);
                }

                Ok(Some(shape.fields.iter().map(|field| field.key).collect()))
            }
            dir::Type::Instance(instance) => Ok(Some(self.nominal_member_keys(instance.symbol))),
            dir::Type::Union(union) => {
                let elements = union.elements.iter().copied().collect::<SmallVec<[_; 4]>>();
                let mut keys = SmallVec::new();
                for element in elements {
                    let Some(element) = self.reduce_type_root(origin, element)?.ready() else {
                        return Ok(None);
                    };
                    match self.accepted_property_keys(origin, element)? {
                        None => return Ok(None),
                        Some(element_keys) => keys.extend(element_keys),
                    }
                }

                Ok(Some(keys))
            }
            dir::Type::Form(form) => {
                let value = self.settled_root(form.value)?;

                self.accepted_property_keys(origin, value)
            }
            _ => Ok(None),
        }
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
        let scope = bindings.scope_at(&self.module(module).view(), source);
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
                if !kind.is_visible_in(dir::SymbolSpace::Declaration) {
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
