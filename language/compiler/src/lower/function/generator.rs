use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::lower::Binding;
use crate::{CompilerError, CompilerResult};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one yield: hand the value to the producer, then act on the request it returns.
    pub(in crate::lower) fn lower_yield(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<mir::Value>> {
        // read the yield's cardinality off its expression
        let dir::Expression::Yield { cardinality, .. } = *self.source().tree().get(expression)
        else {
            return Err(CompilerError::Internal {
                message: "a yield lowering outside a yield expression".to_string(),
            });
        };

        // reject delegation until the iteration protocol lowers
        if cardinality == dir::YieldCardinality::Generator {
            return Err(self.unsupported("a delegating yield"));
        }

        // read the recorded producer call
        let node = expression.into_global_any(self.source);
        let Some(dir::OperationResolution::One(call)) = self
            .lower
            .state(self.source)?
            .decisions
            .call_decision(node)
            .cloned()
        else {
            return Err(CompilerError::Internal {
                message: "a yield without its recorded producer call".to_string(),
            });
        };
        let dir::CallableTarget::Symbol { function, .. } = &call.target else {
            return Err(CompilerError::Internal {
                message: "a yield call outside a direct symbol target".to_string(),
            });
        };
        let Some(producer) = self.producer else {
            return Err(CompilerError::Internal {
                message: "a yield outside a generator body".to_string(),
            });
        };

        // split the yield method's receiver parameter off its value parameters
        let yield_function = self.resolve_callee(&function.key)?;
        let parameters = self.signature_parameters(yield_function.signature)?;
        let Some((&target, parameters)) = parameters.split_first() else {
            return Err(CompilerError::Internal {
                message: "a yield method without a receiver slot".to_string(),
            });
        };

        // borrow the producer at the receiver representation the yield method declares
        let arguments = self.lower_call_arguments(&call.arguments, parameters, &[])?;
        let place = match producer {
            Binding::Local(local) => mir::Place::local(local),
            Binding::Captured { frame, field, .. } => mir::Place::value(frame)
                .with_projection(mir::Projection::Deref)
                .with_projection(mir::Projection::Field { index: field }),
        };
        let receiver = self.builder.address(place, target);

        // call the producer with the yielded value
        let mut values = vec![receiver];
        values.extend(arguments);
        let Some(request) = self.call(&yield_function, values) else {
            return Err(CompilerError::Internal {
                message: "a yield producing no request".to_string(),
            });
        };

        // find the next and return cases of the request union
        let (next_member, return_member) = self.language_members(
            call.return_type,
            dir::LanguageItem::GeneratorNext,
            dir::LanguageItem::GeneratorReturn,
        )?;
        let members = self.lower.union_members(call.return_type)?;
        let next = self.case(&members, next_member)?;
        let finish = self.case(&members, return_member)?;

        // hold the resumed value in a local when the yield produces one
        let resumed = self.node_type_id(expression)?;
        let resumed_type = self.lower_type(resumed)?;
        let is_void = matches!(self.builder.tree().get(resumed_type), mir::Type::Void);
        let slot = (!is_void).then(|| self.builder.local(resumed_type, mir::Mutability::Immutable));

        // dispatch on the request the producer returned
        let next_block = self.builder.block();
        let finish_block = self.builder.block();
        let exit = self.builder.block();
        self.builder.variant_switch(
            request,
            None,
            vec![(next, next_block), (finish, finish_block)],
        );

        // return the completion value the consumer sent
        self.builder.switch_to_block(finish_block);
        let completion = self.request_value(request, finish, return_member)?;
        let completion_type = self.value_representation(completion)?;
        let completion = match self.builder.tree().get(completion_type) {
            mir::Type::Void => None,
            _ => Some(completion),
        };
        self.dispose_down_to(0)?;
        self.return_value(completion)?;

        // resume with the sent value
        self.builder.switch_to_block(next_block);
        let sent = self.request_value(request, next, next_member)?;
        if let Some(slot) = slot {
            self.builder.local_set(slot, sent);
        }
        self.builder.jump(exit);

        // continue lowering after the resumption
        self.builder.switch_to_block(exit);

        Ok(slot.map(|slot| self.builder.local_get(slot)))
    }

    /// Read the value field of one request case.
    fn request_value(
        &mut self,
        request: mir::Value,
        case: u32,
        member: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Value> {
        let payload = self.builder.variant_payload(request, case);
        let index = self.union_case_value_field(member)?;

        Ok(self.builder.field_get(payload, index))
    }
}
