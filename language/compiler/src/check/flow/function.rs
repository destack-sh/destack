use destack_dir as dir;
use indexmap::IndexSet;

use crate::check::{
    FunctionFrame, GenericArgument, Origin, ReceiverCapture, TypeLiteralTerm, TypeOperand,
    TypeRelation, TypeTerm, VariableId, WalkState,
};

impl WalkState<'_, '_> {
    /// Enter one function body while walking.
    pub(in crate::check) fn enter_function_frame(
        &mut self,
        symbol: dir::GlobalSymbolId,
        return_type: TypeOperand,
        yield_type: Option<VariableId>,
        resume_type: Option<VariableId>,
        asynchrony: dir::Asynchrony,
        receiver: Option<ReceiverCapture>,
    ) {
        // capture enclosing flow stack boundaries
        let flow = self.flow();
        let checkpoint = flow.checkpoint();
        let target_start = flow.targets.len();
        let try_start = flow.tries.len();
        let function = FunctionFrame {
            symbol,
            checkpoint,
            target_start,
            try_start,
            receiver,
            return_type,
            yield_type,
            resume_type,
            asynchrony,
            captured_symbols: IndexSet::new(),
            captured_receiver: None,
        };

        // expose function frame to nested flow checks
        self.flow_mut().push_function(function);
    }

    /// Leave the current function body.
    pub(in crate::check) fn leave_function_frame(&mut self) {
        // collect captures and restore outer flow
        let mut capture = self.flow_mut().pop_function();

        // attach capture directive from source metadata
        capture.directive = self.check.capture_directive_for_symbol(capture.symbol);

        // commit capture result
        self.check.captures_mut(self.module).push(capture);
    }

    /// Constrain one explicit or implicit return value to the current function.
    pub(in crate::check) fn constrain_return_value(
        &mut self,
        source: dir::LocalNodeIdAny,
        value: impl Into<TypeOperand>,
    ) {
        // reject returns outside function bodies
        let Some(function) = self.flow().current_function() else {
            self.check
                .report_invalid_control_flow(self.module, source, "return has no target");

            return;
        };

        // constrain value against the active return channel
        let origin = Origin::Node(source.into_global(self.module));
        let condition = self.flow().active_static_guard();

        self.check.relate_type(
            origin,
            TypeRelation::Assignable,
            value,
            function.return_type,
            condition,
        );
    }

    /// Constrain one yield expression to the current generator function.
    pub(in crate::check) fn constrain_yield_value(
        &mut self,
        source: dir::LocalNodeIdAny,
        cardinality: dir::YieldCardinality,
        value: Option<TypeOperand>,
        delegate_return_type: Option<VariableId>,
    ) {
        // require a surrounding function body
        let Some(function) = self.flow().current_function() else {
            self.check
                .report_invalid_yield(self.module, source, "yield requires a generator");

            return;
        };

        // require a generator yield channel
        let Some(yield_type) = function.yield_type else {
            self.check
                .report_invalid_yield(self.module, source, "yield requires a generator");

            return;
        };

        // snapshot function channels before reporting constraints
        let origin = Origin::Node(source.into_global(self.module));
        let resume_type = function.resume_type;
        let asynchrony = function.asynchrony;

        // yield value
        if cardinality == dir::YieldCardinality::Scalar {
            let value = match value {
                Some(value) => value,
                None => self.void_type_operand(),
            };
            let condition = self.flow().active_static_guard();

            self.check.relate_type(
                origin,
                TypeRelation::Assignable,
                value,
                yield_type,
                condition,
            );
        }
        // yield* values
        else if let (Some(value), Some(delegate_return_type), Some(resume_type)) =
            (value, delegate_return_type, resume_type)
        {
            // build expected iterable protocol
            let item = match asynchrony {
                dir::Asynchrony::Sync => dir::LanguageItem::Iterable,
                dir::Asynchrony::Async => dir::LanguageItem::AsyncIterable,
            };
            let symbol = self.check.language_symbol(self.module, item);

            // apply yield delegate channels
            let yield_type = GenericArgument::Type(yield_type.into());
            let delegate_return_type = GenericArgument::Type(delegate_return_type.into());
            let resume_type = GenericArgument::Type(resume_type.into());
            let expected = TypeTerm::Reference {
                origin: Origin::Node(source.into_global(self.module)),
                symbol,
                arguments: vec![yield_type, delegate_return_type, resume_type].into(),
            };
            let expected = self.check.push_term(expected);
            let condition = self.flow().active_static_guard();

            // require delegated value to implement the protocol
            self.check
                .relate_type(origin, TypeRelation::Assignable, value, expected, condition);
        }
        // reject malformed delegation
        else {
            self.check
                .report_invalid_yield(self.module, source, "yield* requires a value");
        }
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

    /// Return the current generator yield channel.
    pub(in crate::check) fn current_yield_type(&self) -> Option<VariableId> {
        self.flow()
            .current_function()
            .and_then(|function| function.yield_type)
    }

    /// Return the current generator resume channel.
    pub(in crate::check) fn current_resume_type(&self) -> Option<VariableId> {
        self.flow()
            .current_function()
            .and_then(|function| function.resume_type)
    }

    /// Return the enclosing function return type.
    pub(in crate::check) fn current_return_type(&self) -> Option<TypeOperand> {
        self.flow()
            .current_function()
            .map(|function| function.return_type)
    }

    /// Constrain a void return to the current function.
    pub(in crate::check) fn constrain_void_return(&mut self, source: dir::LocalNodeIdAny) {
        // reuse normal return constraint logic
        let value = self.void_type_operand();

        self.constrain_return_value(source, value);
    }

    /// Return the void type operand.
    pub(in crate::check) fn void_type_operand(&mut self) -> TypeOperand {
        // intern canonical void type
        let term = TypeTerm::Literal(TypeLiteralTerm::Void);
        let term = self.check.push_term(term);

        term.into()
    }
}
