use destack_dir as dir;
use indexmap::IndexSet;

use crate::CompilerResult;
use crate::check::{ConstraintCause, FunctionFrame, Origin, ReceiverBinding, Relation, WalkState};

impl WalkState<'_, '_> {
    /// Enter one function body while walking.
    pub(in crate::check) fn enter_function_frame(
        &mut self,
        symbol: dir::GlobalSymbolId,
        return_target: dir::GlobalTypeId,
        yield_target: Option<dir::GlobalTypeId>,
        resume_target: Option<dir::GlobalTypeId>,
        asynchrony: dir::Asynchrony,
        receiver: Option<ReceiverBinding>,
    ) {
        // capture enclosing flow stack boundaries
        let flow = self.flow();
        let checkpoint = flow.fork();
        let target_start = flow.targets.len();
        let try_start = flow.tries.len();
        let function = FunctionFrame {
            symbol,
            checkpoint,
            target_start,
            try_start,
            receiver,
            return_target,
            yield_target,
            resume_target,
            asynchrony,
            captured_symbols: IndexSet::new(),
            captured_receiver: None,
        };

        // expose function frame to nested flow checks
        self.flow_mut().push_function(function);
    }

    /// Leave the current function body.
    pub(in crate::check) fn leave_function_frame(&mut self) -> CompilerResult<()> {
        // collect captures and restore outer flow
        let mut capture = self.flow_mut().pop_function();

        // attach capture directive from source metadata
        capture.directive = self.check.capture_directive_for_symbol(capture.symbol)?;

        // commit capture result
        self.check.module_mut(self.module).captures.push(capture);

        Ok(())
    }

    /// Constrain one explicit or implicit return value to the current function.
    pub(in crate::check) fn constrain_return_value(
        &mut self,
        source: dir::LocalNodeIdAny,
        value: dir::GlobalTypeId,
    ) {
        // reject returns outside function bodies
        let Some(function) = self.flow().current_function() else {
            self.check
                .report_invalid_control_flow(self.module, source, "return has no target");

            return;
        };

        // constrain value against the active return target
        let origin = Origin::Node(source.into_global(self.module));
        let return_target = function.return_target;

        self.push_relation(
            origin,
            ConstraintCause::Return,
            Relation::Assignable,
            value,
            return_target,
        );
    }

    /// Constrain one yield expression to the current generator function.
    pub(in crate::check) fn constrain_yield_value(
        &mut self,
        source: dir::LocalNodeIdAny,
        cardinality: dir::YieldCardinality,
        value: Option<dir::GlobalTypeId>,
        delegate_return_target: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        // require a surrounding function body
        let Some(function) = self.flow().current_function() else {
            self.check
                .report_invalid_yield(self.module, source, "yield requires a generator");

            return Ok(());
        };

        // require a generator yield target
        let Some(yield_target) = function.yield_target else {
            self.check
                .report_invalid_yield(self.module, source, "yield requires a generator");

            return Ok(());
        };

        // snapshot function channels before reporting constraints
        let origin = Origin::Node(source.into_global(self.module));
        let resume_target = function.resume_target;
        let asynchrony = function.asynchrony;

        // yield value
        if cardinality == dir::YieldCardinality::Scalar {
            let value = match value {
                Some(value) => value,
                None => self.push_type(dir::Type::Void, source)?,
            };

            self.push_relation(
                origin,
                ConstraintCause::Yield,
                Relation::Assignable,
                value,
                yield_target,
            );
        }
        // yield* values
        else if let (Some(value), Some(delegate_return_target), Some(resume_target)) =
            (value, delegate_return_target, resume_target)
        {
            // build expected iterable protocol
            let item = match asynchrony {
                dir::Asynchrony::Sync => dir::LanguageItem::Iterable,
                dir::Asynchrony::Async => dir::LanguageItem::AsyncIterable,
            };
            let expected = self.language_type_reference(
                source,
                item,
                vec![yield_target, delegate_return_target, resume_target],
            )?;

            // require delegated value to implement the protocol
            self.relate_types(origin, Relation::Assignable, value, expected);
        }
        // reject malformed delegation
        else {
            self.check
                .report_invalid_yield(self.module, source, "yield* requires a value");
        }

        Ok(())
    }

    /// Validate one await expression against the current async context.
    pub(in crate::check) fn validate_await_context(&mut self, source: dir::LocalNodeIdAny) {
        // require a surrounding function body
        let Some(function) = self.flow().current_function() else {
            self.check
                .report_invalid_await(self.module, source, "await requires an async context");

            return;
        };

        // require async function context
        if function.asynchrony != dir::Asynchrony::Async {
            self.check
                .report_invalid_await(self.module, source, "await requires an async context");
        }
    }

    /// Return the current generator resume target.
    pub(in crate::check) fn current_resume_target(&self) -> Option<dir::GlobalTypeId> {
        self.flow()
            .current_function()
            .and_then(|function| function.resume_target)
    }

    /// Return the enclosing function return target.
    pub(in crate::check) fn current_return_target(&self) -> Option<dir::GlobalTypeId> {
        self.flow()
            .current_function()
            .map(|function| function.return_target)
    }

    /// Constrain a void return to the current function.
    pub(in crate::check) fn constrain_void_return(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<()> {
        // reuse normal return constraint logic
        let value = self.push_type(dir::Type::Void, source)?;
        self.constrain_return_value(source, value);

        Ok(())
    }
}
