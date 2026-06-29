use destack_dir as dir;
use indexmap::IndexSet;

use crate::CompilerResult;
use crate::check::{
    Expectation, ExpectedType, FlowBranch, FlowSite, FunctionFrame, Origin, ReceiverBinding,
    Relation, Task, ValueUse, WalkState,
};

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
    pub(in crate::check) fn leave_function_frame(&mut self) -> CompilerResult<FlowBranch> {
        // collect captures and restore outer flow
        let (mut capture, flow) = self.flow_mut().pop_function();

        // attach capture directive from source metadata
        capture.directive = self.check.capture_directive_for_symbol(capture.symbol)?;

        // store capture result
        self.check.module_mut(self.module).captures.push(capture);

        Ok(flow)
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
                .report_return_outside_function(self.module, source);

            return;
        };

        // constrain value against the active return target
        let origin = Origin::Node(source.into_global(self.module));
        let return_target = function.return_target;

        self.relate_value(
            origin,
            ValueUse::Output,
            Relation::Assignable,
            value,
            return_target,
        );
    }

    /// Constrain one return expression to the current function.
    pub(in crate::check) fn constrain_return_expression(
        &mut self,
        source: dir::LocalNodeIdAny,
        value: dir::LocalNodeId<dir::Expression>,
    ) {
        // reject returns outside function bodies
        let Some(function) = self.flow().current_function() else {
            self.check
                .report_return_outside_function(self.module, source);

            return;
        };

        // queue the return flow until the expression has a type
        self.check.queue_task(Task::Check {
            site: FlowSite {
                node: value.into_global_any(self.module),
                flow: self.flow().point(),
            },
            expected: ExpectedType::Type(function.return_target),
            relation: Relation::Assignable,
            origin: Origin::Node(source.into_global(self.module)),
            use_: ValueUse::Output,
        });
    }

    /// Return the expected type for one yielded value expression.
    pub(in crate::check) fn yield_value_expectation(
        &mut self,
        source: dir::LocalNodeIdAny,
        cardinality: dir::YieldCardinality,
        value: dir::LocalNodeId<dir::Expression>,
        delegate_return_target: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<Expectation>> {
        // require a surrounding function body
        let Some(function) = self.flow().current_function() else {
            self.check
                .report_yield_outside_generator(self.module, source);

            return Ok(None);
        };

        // require a generator yield target
        let Some(yield_target) = function.yield_target else {
            self.check
                .report_yield_outside_generator(self.module, source);

            return Ok(None);
        };

        let resume_target = function.resume_target;
        let asynchrony = function.asynchrony;
        let origin = Origin::Node(value.into_global_any(self.module));

        // scalar yield values flow directly to the yield target
        if cardinality == dir::YieldCardinality::Scalar {
            return Ok(Some(Expectation::assignable(
                yield_target,
                origin,
                ValueUse::Output,
            )));
        }

        // delegated yield values must implement the generator protocol
        if let (Some(delegate_return_target), Some(resume_target)) =
            (delegate_return_target, resume_target)
        {
            let item = match asynchrony {
                dir::Asynchrony::Sync => dir::LanguageItem::Iterable,
                dir::Asynchrony::Async => dir::LanguageItem::AsyncIterable,
            };
            let expected = self.language_type_reference(
                source,
                item,
                vec![yield_target, delegate_return_target, resume_target],
            )?;

            Ok(Some(Expectation::assignable(
                expected,
                origin,
                ValueUse::Output,
            )))
        }
        // reject malformed delegation
        else {
            self.check
                .report_yield_delegate_missing_value(self.module, source);

            Ok(None)
        }
    }

    /// Constrain an omitted yield value to the current generator function.
    pub(in crate::check) fn constrain_void_yield(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<()> {
        // require a surrounding generator body
        let Some(function) = self.flow().current_function() else {
            self.check
                .report_yield_outside_generator(self.module, source);

            return Ok(());
        };
        let Some(yield_target) = function.yield_target else {
            self.check
                .report_yield_outside_generator(self.module, source);

            return Ok(());
        };

        // flow omitted yield as void
        let origin = Origin::Node(source.into_global(self.module));
        let value = self.push_type(dir::Type::Void, source)?;
        self.relate_value(
            origin,
            ValueUse::Output,
            Relation::Assignable,
            value,
            yield_target,
        );

        Ok(())
    }

    /// Validate one await expression against the current async context.
    pub(in crate::check) fn validate_await_context(&mut self, source: dir::LocalNodeIdAny) {
        // require a surrounding function body
        let Some(function) = self.flow().current_function() else {
            self.check
                .report_await_outside_async_context(self.module, source);

            return;
        };

        // require async function context
        if function.asynchrony != dir::Asynchrony::Async {
            self.check
                .report_await_outside_async_context(self.module, source);
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
        // flow omitted return as void
        let value = self.push_type(dir::Type::Void, source)?;
        self.constrain_return_value(source, value);

        Ok(())
    }
}
