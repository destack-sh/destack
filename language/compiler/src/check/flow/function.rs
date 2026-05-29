use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;

use crate::check::{
    CheckState, FunctionFrame, GenericArgument, Origin, ReceiverCapture, TypeLiteralTerm,
    TypeOperand, TypeRelation, TypeTerm, VariableId,
};

impl CheckState<'_> {
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
        let module = symbol.module_id;
        let flow = self.flow(module);
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

        self.flow_mut(module).push_function(function);
    }

    /// Leave the current function body.
    pub(in crate::check) fn leave_function_frame(&mut self, module: ModuleId) {
        let mut capture = self.flow_mut(module).pop_function();

        capture.directive = self.capture_directive_for_symbol(capture.symbol);

        self.captures_mut(module).push(capture);
    }

    /// Constrain one explicit or implicit return value to the current function.
    pub(in crate::check) fn constrain_return_value(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        value: impl Into<TypeOperand>,
    ) {
        let Some(function) = self.flow(module).current_function() else {
            self.report_invalid_control_flow(module, source, "return has no target");

            return;
        };
        let origin = Origin::Node(source.into_global(module));
        let condition = self.flow(module).current_static_condition();

        self.add_type_constraint(
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
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        cardinality: dir::YieldCardinality,
        value: Option<VariableId>,
        delegate_return_type: Option<VariableId>,
    ) {
        let Some(function) = self.flow(module).current_function() else {
            self.report_invalid_yield(module, source, "yield requires a generator");

            return;
        };
        let Some(yield_type) = function.yield_type else {
            self.report_invalid_yield(module, source, "yield requires a generator");

            return;
        };
        let origin = Origin::Node(source.into_global(module));
        let resume_type = function.resume_type;
        let asynchrony = function.asynchrony;

        // yield value
        if cardinality == dir::YieldCardinality::Scalar {
            let value = match value {
                Some(value) => value.into(),
                None => self.void_type_operand(),
            };
            let condition = self.flow(module).current_static_condition();

            self.add_type_constraint(
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
            let item = match asynchrony {
                dir::Asynchrony::Sync => dir::LanguageItem::Iterable,
                dir::Asynchrony::Async => dir::LanguageItem::AsyncIterable,
            };
            let symbol = self.language_symbol(module, item);
            let yield_type = GenericArgument::Type(yield_type.into());
            let delegate_return_type = GenericArgument::Type(delegate_return_type.into());
            let resume_type = GenericArgument::Type(resume_type.into());
            let expected = TypeTerm::Reference {
                origin: Origin::Node(source.into_global(module)),
                symbol,
                arguments: vec![yield_type, delegate_return_type, resume_type].into(),
            };
            let expected = self.terms.push(expected);
            let condition = self.flow(module).current_static_condition();

            self.add_type_constraint(origin, TypeRelation::Assignable, value, expected, condition);
        }
        // reject malformed delegation
        else {
            self.report_invalid_yield(module, source, "yield* requires a value");
        }
    }

    /// Validate one await expression against the current async context.
    pub(in crate::check) fn validate_await_context(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let Some(function) = self.flow(module).current_function() else {
            self.report_invalid_await(module, source, "await requires an async context");

            return;
        };
        if function.asynchrony != dir::Asynchrony::Async {
            self.report_invalid_await(module, source, "await requires an async context");
        }
    }

    /// Return the current generator yield channel.
    pub(in crate::check) fn current_yield_type(&self, module: ModuleId) -> Option<VariableId> {
        self.flow(module)
            .current_function()
            .and_then(|function| function.yield_type)
    }

    /// Return the current generator resume channel.
    pub(in crate::check) fn current_resume_type(&self, module: ModuleId) -> Option<VariableId> {
        self.flow(module)
            .current_function()
            .and_then(|function| function.resume_type)
    }

    /// Return the enclosing function return type.
    pub(in crate::check) fn current_return_type(&self, module: ModuleId) -> Option<VariableId> {
        self.flow(module)
            .current_function()
            .map(|function| function.return_type)
    }

    /// Constrain a void return to the current function.
    pub(in crate::check) fn constrain_void_return(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let value = self.void_type_operand();

        self.constrain_return_value(module, source, value);
    }

    /// Return the void type operand.
    pub(in crate::check) fn void_type_operand(&mut self) -> TypeOperand {
        let term = TypeTerm::Literal(TypeLiteralTerm::Void);
        let term = self.terms.push(term);

        term.into()
    }
}
