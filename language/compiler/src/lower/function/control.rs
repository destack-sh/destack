use destack_core::StringId;
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::call::ReceiverUse;
use crate::lower::function::lower::{Binding, ChainFrame, ControlFrame};
use crate::lower::function::place::Place;
use crate::{CompilerError, CompilerResult};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one if statement, returning whether every arm terminated.
    pub(in crate::lower) fn lower_if(
        &mut self,
        condition: &dir::Condition,
        then_expression: dir::LocalNodeId<dir::Expression>,
        else_expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<bool> {
        // branch on the condition, with the join serving as the else edge of a bare if
        let condition = self.lower_condition(condition)?;
        let then_block = self.builder.block();
        let join = self.builder.block();
        let else_block = match else_expression {
            Some(_) => self.builder.block(),
            None => join,
        };
        self.builder.branch(condition, then_block, else_block);

        // lower the then arm, joining its fallthrough edge
        self.builder.switch_to_block(then_block);
        let then_terminated = self.lower_arm(then_expression)?;
        if !then_terminated {
            self.builder.jump(join);
        }

        // lower the else arm the same way
        let mut else_terminated = false;
        if let Some(else_expression) = else_expression {
            self.builder.switch_to_block(else_block);
            else_terminated = self.lower_arm(else_expression)?;
            if !else_terminated {
                self.builder.jump(join);
            }
        }

        // leave the join unreachable when both arms terminate
        if then_terminated && else_terminated {
            return Ok(true);
        }

        // continue lowering at the join
        self.builder.switch_to_block(join);

        Ok(false)
    }

    /// Lower one while or do-while loop.
    pub(in crate::lower) fn lower_while(
        &mut self,
        label: Option<StringId>,
        form: dir::WhileForm,
        condition: &dir::Condition,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<bool> {
        // enter through the condition for while, through the body for do-while
        let header = self.builder.block();
        let body_block = self.builder.block();
        let exit = self.builder.block();
        match form {
            dir::WhileForm::While => self.builder.jump(header),
            dir::WhileForm::DoWhile => self.builder.jump(body_block),
        }

        // re-evaluate the condition in the header on every iteration
        self.builder.switch_to_block(header);
        let condition = self.lower_condition(condition)?;
        self.builder.branch(condition, body_block, exit);

        // lower the body, looping back through the condition header
        self.builder.switch_to_block(body_block);
        let terminated = self.lower_loop_body(label, header, exit, body, None, None)?;
        if !terminated {
            self.builder.jump(header);
        }

        // continue lowering after the loop
        self.builder.switch_to_block(exit);

        Ok(false)
    }

    /// Lower one for loop over its initialization, condition and increment.
    pub(in crate::lower) fn lower_for(
        &mut self,
        label: Option<StringId>,
        initialization: Option<dir::LocalNodeId<dir::Expression>>,
        condition: Option<dir::LocalNodeId<dir::Expression>>,
        increment: Option<dir::LocalNodeId<dir::Expression>>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<bool> {
        // run the initialization once before entering the loop
        if let Some(initialization) = initialization {
            self.lower_statement(initialization)?;
        }

        // enter the loop through the condition header
        let header = self.builder.block();
        let body_block = self.builder.block();
        let continue_block = self.builder.block();
        let exit = self.builder.block();
        self.builder.jump(header);

        // re-evaluate the condition in the header on every iteration
        self.builder.switch_to_block(header);
        match condition {
            Some(condition) => {
                let condition = self.lower_value(condition)?;
                self.builder.branch(condition, body_block, exit);
            }
            None => self.builder.jump(body_block),
        }

        // lower the body, routing continue through the increment
        self.builder.switch_to_block(body_block);
        let terminated = self.lower_loop_body(label, continue_block, exit, body, None, None)?;
        if !terminated {
            self.builder.jump(continue_block);
        }

        // run the increment and loop back to the condition
        self.builder.switch_to_block(continue_block);
        if let Some(increment) = increment {
            self.lower_statement(increment)?;
        }
        self.builder.jump(header);

        // continue lowering after the loop
        self.builder.switch_to_block(exit);

        Ok(false)
    }

    /// Lower one unconditional loop.
    pub(in crate::lower) fn lower_loop(
        &mut self,
        label: Option<StringId>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<bool> {
        let body_block = self.builder.block();
        let exit = self.builder.block();
        self.builder.jump(body_block);
        self.builder.switch_to_block(body_block);

        // loop back to the body when it falls through
        let terminated = self.lower_loop_body(label, body_block, exit, body, None, None)?;
        if !terminated {
            self.builder.jump(body_block);
        }
        self.builder.switch_to_block(exit);

        Ok(false)
    }

    /// Lower one break statement.
    pub(in crate::lower) fn lower_break(
        &mut self,
        label: Option<StringId>,
        value: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<bool> {
        // write the jump's value into the destination its target reads
        if let Some(value) = value {
            let Some(destination) = self.break_frame(label)?.destination.clone() else {
                return Err(CompilerError::Internal {
                    message: "a valued break outside a valued loop".to_string(),
                });
            };
            if !self.lower_into(value, &destination)? {
                return Ok(true);
            }
        }

        // dispose the resources of the scopes the break leaves, then jump to the exit
        let (target, disposals) = self.break_target(label)?;
        self.dispose_down_to(disposals)?;
        self.builder.jump(target);

        Ok(true)
    }

    /// Lower one continue statement.
    pub(in crate::lower) fn lower_continue(
        &mut self,
        label: Option<StringId>,
    ) -> CompilerResult<bool> {
        // dispose the resources of the scopes the continue leaves, then jump to the next pass
        let (target, disposals) = self.continue_target(label)?;
        self.dispose_down_to(disposals)?;
        self.builder.jump(target);

        Ok(true)
    }

    /// Lower one for-of loop through its recorded iteration protocol calls.
    pub(in crate::lower) fn lower_for_each(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        label: Option<StringId>,
        binding: dir::ForEachBinding,
        iterator: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<bool> {
        let node = expression.into_global_any(self.source);
        let Some(decision) = self
            .lower
            .state(self.source)?
            .decisions
            .iteration_decision(node)
            .cloned()
        else {
            return Err(CompilerError::Internal {
                message: "a for-of loop without its recorded iteration calls".to_string(),
            });
        };

        // open the iterator over the source, homing it for the borrows next takes
        let opened = match &decision.iterator.target {
            dir::CallableTarget::Symbol { function, .. } => self.lower_function_target_call(
                iterator,
                &decision.iterator,
                function,
                None,
                false,
            )?,
            dir::CallableTarget::Dynamic {
                dispatch,
                function: dir::DynamicFunction::Symbol(symbol),
                ..
            } => {
                let receiver = self.lower_adjusted_receiver(
                    iterator,
                    &dispatch.receiver,
                    false,
                    ReceiverUse::Value,
                )?;

                self.lower_dynamic_symbol_call(receiver, dispatch, *symbol, &decision.iterator)?
            }
            _ => {
                return Err(self.unsupported("a virtual iterator call"));
            }
        };
        let Some(opened) = opened else {
            return Err(CompilerError::Internal {
                message: "an iterator call producing no iterator".to_string(),
            });
        };
        self.lower_iteration(&decision, opened, |lower, value, header, exit| {
            let pattern = match binding {
                dir::ForEachBinding::Pattern {
                    pattern,
                    keyword: Some(_),
                }
                | dir::ForEachBinding::Using { pattern, .. } => pattern,
                dir::ForEachBinding::Pattern { keyword: None, .. } => {
                    return Err(lower.unsupported("a for-of assignment binding"));
                }
            };
            let place = Place::local(lower.home(value));
            lower.lower_pattern_bindings(pattern, &place)?;

            // queue the disposal a using binding runs after every pass
            let resource = match (binding, decision.disposal.clone()) {
                (dir::ForEachBinding::Using { pattern, .. }, Some(disposal)) => {
                    let node = pattern.into_global_any(lower.source);
                    let Some(symbol) = lower.lower.symbol_declared_at(node)? else {
                        return Err(CompilerError::Internal {
                            message: "a missing symbol for one for-of using binding".to_string(),
                        });
                    };
                    let Some(home) = lower.values.get(&symbol.local_id).copied() else {
                        return Err(CompilerError::Internal {
                            message: "a for-of using binding without a home".to_string(),
                        });
                    };

                    Some((home, disposal))
                }
                _ => None,
            };
            lower.lower_loop_body(label, header, exit, body, resource, None)
        })
    }

    /// Consume a selected iterator and emit the body for each element.
    pub(in crate::lower) fn lower_iteration(
        &mut self,
        decision: &dir::IterationDecision,
        opened: mir::Value,
        body: impl FnOnce(
            &mut Self,
            mir::Value,
            mir::LocalNodeId<mir::Block>,
            mir::LocalNodeId<mir::Block>,
        ) -> CompilerResult<bool>,
    ) -> CompilerResult<bool> {
        let home = self.bind_receiver(opened)?;
        let awaited = match &decision.awaits {
            Some(awaited) => {
                let dir::CallableTarget::Symbol { function, .. } = &awaited.call.target else {
                    return Err(CompilerError::Internal {
                        message: "an iteration await outside a direct symbol target".to_string(),
                    });
                };

                Some((
                    self.resolve_callee(&function.key)?,
                    awaited.call.return_type,
                    awaited.target,
                ))
            }
            None => None,
        };
        let result_type = match awaited {
            Some((_, parked, dir::AwaitTarget::Result)) => parked,
            _ => decision.next.return_type,
        };
        let (yield_member, return_member) = self.language_members(
            result_type,
            dir::LanguageItem::IteratorYield,
            dir::LanguageItem::IteratorReturn,
        )?;
        let members = self.lower.union_members(result_type)?;
        let yielded = self.case(&members, yield_member)?;
        let finished = self.case(&members, return_member)?;

        // advance in the header, dispatching each result on its case
        let header = self.builder.block();
        let body_block = self.builder.block();
        let exit = self.builder.block();
        self.builder.jump(header);
        self.builder.switch_to_block(header);
        let advanced = match &decision.next.target {
            // borrow the homed iterator at the declared receiver slot
            dir::CallableTarget::Symbol { function, .. } => {
                let mut next_function = self.resolve_callee(&function.key)?;
                self.instantiate_symbol_callee(&mut next_function, &function.key, &decision.next)?;
                let parameters = next_function.parameters.clone();
                let Some((&receiver_slot, _)) = parameters.split_first() else {
                    return Err(CompilerError::Internal {
                        message: "an iterator advance without its receiver slot".to_string(),
                    });
                };
                let place = self.binding_home(home)?;
                let receiver = self.borrow_place(&place, receiver_slot)?;

                self.call(&next_function, vec![receiver])
            }
            // dispatch through the erased iterator's constraint slot
            dir::CallableTarget::Dynamic {
                dispatch,
                function: dir::DynamicFunction::Symbol(symbol),
                ..
            } => {
                let receiver = self.read_binding(home)?;
                let receiver =
                    self.lower_receiver_adjustments(receiver, &dispatch.receiver.adjustments)?;

                self.lower_dynamic_symbol_call(receiver, dispatch, *symbol, &decision.next)?
            }
            _ => {
                return Err(self.unsupported("a virtual iterator advance"));
            }
        };
        let Some(mut result) = advanced else {
            return Err(CompilerError::Internal {
                message: "an iterator advance producing no result".to_string(),
            });
        };
        if let Some((await_function, _, dir::AwaitTarget::Result)) = &awaited {
            let Some(parked) = self.call(await_function, vec![result]) else {
                return Err(CompilerError::Internal {
                    message: "an iteration await producing no result".to_string(),
                });
            };
            result = parked;
        }

        // read through the result's newtype layers to its variant
        let result = self.read_through_newtypes(result)?;
        self.builder
            .variant_switch(result, None, vec![(yielded, body_block), (finished, exit)]);

        // bind the yielded value and run the body
        self.builder.switch_to_block(body_block);
        let payload = self.builder.variant_payload(result, yielded);
        let index = self.union_case_value_field(yield_member)?;
        let mut value = self.builder.field_get(payload, index);
        if let Some((await_function, _, dir::AwaitTarget::Element)) = &awaited {
            let Some(parked) = self.call(await_function, vec![value]) else {
                return Err(CompilerError::Internal {
                    message: "an element await producing no value".to_string(),
                });
            };
            value = parked;
        }
        let terminated = body(self, value, header, exit)?;
        if !terminated {
            self.builder.jump(header);
        }

        // continue lowering after the loop
        self.builder.switch_to_block(exit);

        Ok(false)
    }

    /// Lower one loop body under its control frame, disposing a per-pass resource after every pass.
    pub(in crate::lower) fn lower_loop_body(
        &mut self,
        label: Option<StringId>,
        continue_target: mir::LocalNodeId<mir::Block>,
        exit: mir::LocalNodeId<mir::Block>,
        body: dir::LocalNodeId<dir::Block>,
        resource: Option<(Binding, dir::DisposalDecision)>,
        destination: Option<Place>,
    ) -> CompilerResult<bool> {
        // open the disposal frame ahead of the resource
        self.enter_control(label, exit, Some(continue_target), destination);
        let depth = self.open_disposals();
        if let Some((home, decision)) = resource {
            self.queue_disposal(home, decision);
        }

        // run the pass, disposing the resource on fallthrough
        let terminated = self.lower_block(body)?;
        self.close_disposals(depth, terminated)?;
        self.leave_control();

        Ok(terminated)
    }

    /// Lower one condition to a boolean value.
    pub(in crate::lower) fn lower_condition(
        &mut self,
        condition: &dir::Condition,
    ) -> CompilerResult<mir::Value> {
        // reject a binding condition
        let Some(condition) = condition.as_expression() else {
            return Err(self.unsupported("a binding condition"));
        };

        self.lower_value(condition)
    }

    /// Enter one statement's break and continue targets, a break value landing at the join.
    pub(in crate::lower) fn enter_control(
        &mut self,
        label: Option<StringId>,
        break_target: mir::LocalNodeId<mir::Block>,
        continue_target: Option<mir::LocalNodeId<mir::Block>>,
        destination: Option<Place>,
    ) {
        self.controls.push(ControlFrame {
            label,
            break_target,
            disposals: self.disposals.len(),
            continue_target,
            destination,
        });
    }

    /// Leave the innermost break and continue target.
    pub(in crate::lower) fn leave_control(&mut self) {
        self.controls.pop();
    }

    /// Return the frame one break leaves: the labeled one, or the innermost for a bare break.
    fn break_frame(&self, label: Option<StringId>) -> CompilerResult<&ControlFrame> {
        let frame = match label {
            None => self.controls.last(),
            Some(label) => self
                .controls
                .iter()
                .rev()
                .find(|frame| frame.label == Some(label)),
        };

        // require an enclosing breakable statement
        frame.ok_or_else(|| CompilerError::Internal {
            message: "a missing enclosing statement for one break".to_string(),
        })
    }

    /// Return the block one break targets and the disposal depth its statement opened at.
    fn break_target(
        &self,
        label: Option<StringId>,
    ) -> CompilerResult<(mir::LocalNodeId<mir::Block>, usize)> {
        let frame = self.break_frame(label)?;

        Ok((frame.break_target, frame.disposals))
    }

    /// Return the block one continue targets and the disposal depth its loop opened at.
    fn continue_target(
        &self,
        label: Option<StringId>,
    ) -> CompilerResult<(mir::LocalNodeId<mir::Block>, usize)> {
        // take the innermost loop frame with the label
        let frame = self.controls.iter().rev().find(|frame| {
            frame.continue_target.is_some()
                && match label {
                    Some(label) => frame.label == Some(label),
                    None => true,
                }
        });

        // require an enclosing loop
        let Some((target, disposals)) =
            frame.and_then(|frame| Some((frame.continue_target?, frame.disposals)))
        else {
            return Err(CompilerError::Internal {
                message: "a missing enclosing loop for one continue".to_string(),
            });
        };

        Ok((target, disposals))
    }

    /// Lower one optional chain, joining its value with the short-circuit undefined.
    pub(in crate::lower) fn lower_chain(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        inner: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // stage the destination and the exit block
        let destination = self.join_place(expression)?;
        let exit = self.builder.block();

        // lower the chained accesses under the frame their guards exit through
        self.chains.push(ChainFrame {
            destination: destination.clone(),
            exit,
        });
        let falls_through = self.lower_into(inner, &destination)?;
        self.chains.pop();
        if falls_through {
            self.builder.jump(exit);
        }
        self.builder.switch_to_block(exit);

        self.read_place(&destination)
    }

    /// Branch one optional receiver, short-circuiting the enclosing chain when it is absent.
    pub(in crate::lower) fn lower_chain_guard(
        &mut self,
        value: mir::Value,
    ) -> CompilerResult<mir::Value> {
        // pass receivers through outside a chain
        let Some(frame) = self.chains.last() else {
            return Ok(value);
        };
        let (destination, exit) = (frame.destination.clone(), frame.exit);

        // branch the receiver on its nullish cases, passing a present receiver through
        let Some((present, absent)) = self.split_absent(value)? else {
            return Ok(value);
        };

        // short-circuit the chain with undefined when the receiver is absent
        self.builder.switch_to_block(absent);
        let representation = self.place_type(&destination)?;
        let Some(undefined) = self.absent_value(representation) else {
            return Err(CompilerError::Internal {
                message: "an optional chain without an undefined case".to_string(),
            });
        };
        self.write_place(&destination, undefined)?;
        self.builder.jump(exit);

        // continue the chain with the whole receiver
        self.builder.switch_to_block(present);

        Ok(value)
    }
}
