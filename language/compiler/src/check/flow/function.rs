use destack_dir as dir;
use indexmap::IndexSet;

use crate::check::{
    ArgumentTerm, CheckModuleState, ConstraintOrigin, FunctionFrame, ReceiverCapture,
    TypeLiteralTerm, TypeRelation, TypeTerm, VariableId,
};

impl CheckModuleState {
    /// Enter one function body while walking.
    pub(in crate::check) fn enter_function_frame(
        &mut self,
        symbol: dir::GlobalSymbolId,
        return_type: VariableId,
        yield_type: Option<VariableId>,
        resume_type: Option<VariableId>,
        asynchrony: dir::Asynchrony,
        receiver: Option<ReceiverCapture>,
    ) {
        let checkpoint = self.work.flow.checkpoint();
        let function = FunctionFrame {
            symbol,
            checkpoint,
            receiver,
            return_type,
            yield_type,
            resume_type,
            asynchrony,
            captured_symbols: IndexSet::new(),
            captured_receiver: None,
        };

        self.work.flow.push_function(function);
    }

    /// Leave the current function body.
    pub(in crate::check) fn leave_function_frame(&mut self) {
        let Some(mut capture) = self.work.flow.pop_function() else {
            return;
        };

        capture.directive = self.capture_directive_for_symbol(capture.symbol);

        self.work.captures.push(capture);
    }

    /// Constrain one explicit or implicit return value to the current function.
    pub(in crate::check) fn constrain_return_value(
        &mut self,
        source: dir::LocalNodeIdAny,
        value: VariableId,
    ) {
        let Some(function) = self.work.flow.current_function() else {
            self.report_invalid_control_flow(source, "return has no target");

            return;
        };
        let origin = ConstraintOrigin::Node(source.into_global(self.input.module_id));

        self.constrain_type(
            origin,
            TypeRelation::Assignable,
            value,
            function.return_type,
        );
    }

    /// Constrain one yield expression to the current generator function.
    pub(in crate::check) fn constrain_yield_value(
        &mut self,
        source: dir::LocalNodeIdAny,
        cardinality: dir::YieldCardinality,
        value: Option<VariableId>,
        delegate_return_type: Option<VariableId>,
    ) {
        let Some(function) = self.work.flow.current_function() else {
            self.report_invalid_yield(source, "yield requires a generator");

            return;
        };
        let Some(yield_type) = function.yield_type else {
            self.report_invalid_yield(source, "yield requires a generator");

            return;
        };
        let origin = ConstraintOrigin::Node(source.into_global(self.input.module_id));
        let resume_type = function.resume_type;
        let asynchrony = function.asynchrony;

        // yield value
        if cardinality == dir::YieldCardinality::Scalar {
            let value = match value {
                Some(value) => value,
                None => self.define_void_type(source),
            };

            self.constrain_type(origin, TypeRelation::Assignable, value, yield_type);
        }
        // yield* values
        else if let (Some(value), Some(delegate_return_type), Some(resume_type)) =
            (value, delegate_return_type, resume_type)
        {
            let item = match asynchrony {
                dir::Asynchrony::Sync => dir::LanguageItem::Iterable,
                dir::Asynchrony::Async => dir::LanguageItem::AsyncIterable,
            };
            let Some(symbol) = self.language_symbol(item) else {
                self.report_internal_error(
                    source,
                    format!("missing language item: {}", item.key()),
                );

                return;
            };
            let expected = TypeTerm::Reference {
                source: Some(source.into_global(self.input.module_id)),
                symbol,
                arguments: vec![
                    ArgumentTerm::Type(yield_type),
                    ArgumentTerm::Type(delegate_return_type),
                    ArgumentTerm::Type(resume_type),
                ],
            };
            let expected = self.define_anonymous_type(origin, expected);

            self.constrain_type(origin, TypeRelation::Assignable, value, expected);
        }
        // reject malformed delegation
        else {
            self.report_invalid_yield(source, "yield* requires a value");
        }
    }

    /// Validate one await expression against the current async context.
    pub(in crate::check) fn validate_await_context(&mut self, source: dir::LocalNodeIdAny) {
        let Some(function) = self.work.flow.current_function() else {
            self.report_invalid_await(source, "await requires an async context");

            return;
        };
        if function.asynchrony != dir::Asynchrony::Async {
            self.report_invalid_await(source, "await requires an async context");
        }
    }

    /// Return the current generator yield channel.
    pub(in crate::check) fn current_yield_type(&self) -> Option<VariableId> {
        self.work
            .flow
            .current_function()
            .and_then(|function| function.yield_type)
    }

    /// Return the current generator resume channel.
    pub(in crate::check) fn current_resume_type(&self) -> Option<VariableId> {
        self.work
            .flow
            .current_function()
            .and_then(|function| function.resume_type)
    }

    /// Return the enclosing function return type.
    pub(in crate::check) fn current_return_type(&self) -> Option<VariableId> {
        self.work
            .flow
            .current_function()
            .map(|function| function.return_type)
    }

    /// Constrain a void return to the current function.
    pub(in crate::check) fn constrain_void_return(&mut self, source: dir::LocalNodeIdAny) {
        let value = self.define_void_type(source);

        self.constrain_return_value(source, value);
    }

    /// Define one void type variable.
    pub(in crate::check) fn define_void_type(&mut self, source: dir::LocalNodeIdAny) -> VariableId {
        let origin = ConstraintOrigin::Node(source.into_global(self.input.module_id));

        self.define_anonymous_type(origin, TypeTerm::Literal(TypeLiteralTerm::Void))
    }
}
