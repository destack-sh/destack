use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::disposal::Disposal;
use crate::lower::function::lower::TryFrame;
use crate::lower::function::operand::Operand;
use crate::lower::function::place::Place;
use crate::{CompilerError, CompilerResult};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one must, trapping when the operand holds its residual.
    pub(in crate::lower) fn lower_must(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        let decision = self.residual_decision(expression)?;
        if !matches!(decision.target, dir::ResidualTarget::Trap) {
            return Err(CompilerError::Internal {
                message: "a must recorded outside a trap residual".to_string(),
            });
        }

        // split a try implementor through its recorded branch, trapping on the residual
        let value = self.lower_value(left)?;
        if decision.branch.is_some() {
            return self.lower_try_branch(value, &decision, |lower, _| {
                lower.builder.panic(None);

                Ok(())
            });
        }

        // narrow a nullish operand to the case it asserts
        let narrowed = self.node_type_id(expression)?;
        let members = self
            .lower
            .union_members_maybe(narrowed)?
            .unwrap_or_else(|| vec![narrowed]);

        self.narrow(value, self.node_type_id(left)?, &members, narrowed)
    }

    /// Lower one propagating try projection, continuing with the output its operand holds.
    pub(in crate::lower) fn lower_maybe(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        let decision = self.residual_decision(expression)?;
        if matches!(decision.target, dir::ResidualTarget::Trap) {
            return Err(CompilerError::Internal {
                message: "a try projection recorded as a trap".to_string(),
            });
        }

        // split a nullish operand on its own absent case
        let value = self.lower_value(left)?;
        let declared = self.node_type_id(left)?;
        if decision.branch.is_none() {
            let Some((present, absent)) = self.split_absent(value)? else {
                return Err(CompilerError::Internal {
                    message: "a nullish try projection without an absent case".to_string(),
                });
            };
            self.builder.switch_to_block(absent);
            self.transfer_residual(value, declared, &decision)?;
            self.builder.switch_to_block(present);
            let narrowed = self.node_type_id(expression)?;
            let members = self
                .lower
                .union_members_maybe(narrowed)?
                .unwrap_or_else(|| vec![narrowed]);

            return self.narrow(value, self.node_type_id(left)?, &members, narrowed);
        }

        // split a try implementor through its recorded branch, transferring the residual
        self.lower_try_branch(value, &decision, |lower, residual| {
            lower.transfer_residual(residual, decision.residual, &decision)
        })
    }

    /// Lower one try expression, answering whether control falls through.
    pub(in crate::lower) fn lower_try(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Expression>,
        catch: Option<dir::LocalNodeId<dir::Catch>>,
        finally: Option<dir::LocalNodeId<dir::Expression>>,
        destination: Option<&Place>,
    ) -> CompilerResult<bool> {
        // run the finally on every way out of the try
        let depth = self.open_disposals();
        if let Some(finally) = finally {
            self.disposals.push(Disposal::Finally(finally));
        }

        // run an uncaught body as it stands
        let Some(catch) = catch else {
            let falls_through = self.lower_try_arm(body, destination)?;
            self.close_disposals(depth, !falls_through)?;

            return Ok(falls_through);
        };
        let dir::Catch {
            pattern,
            body: handler,
            ..
        } = *self.source().tree().get(catch);

        // home the caught residual at the type the catch pattern declares
        let residual = match pattern {
            Some(pattern) => {
                let node = pattern.into_global_any(self.source);
                let Some(ty) = self.source().types.get_node_type_id(node) else {
                    return Err(CompilerError::Internal {
                        message: "a catch pattern without its type".to_string(),
                    });
                };
                let representation = self.lower_type(ty)?;
                let local = self.builder.local(representation, mir::Mutability::Mutable);

                Some((local, ty))
            }
            None => None,
        };

        // run the body under the frame, its residuals landing in the catch block
        let catch_block = self.builder.block();
        let exit = self.builder.block();
        self.tries.push(TryFrame {
            node: expression,
            residual,
            catch: catch_block,
            disposals: self.disposals.len(),
        });
        let mut falls_through = self.lower_try_arm(body, destination)?;
        self.tries.pop();
        if falls_through {
            self.builder.jump(exit);
        }

        // run the catch over the caught residual when any residual reaches it
        if self.builder.is_entered(catch_block) {
            self.builder.switch_to_block(catch_block);
            if let (Some(pattern), Some((local, _))) = (pattern, residual) {
                self.lower_pattern_bindings(pattern, &Place::local(local))?;
            }
            if self.lower_try_arm(handler, destination)? {
                self.builder.jump(exit);
                falls_through = true;
            }
        }

        // continue past the try when a side falls through
        if falls_through {
            self.builder.switch_to_block(exit);
        }
        self.close_disposals(depth, !falls_through)?;

        Ok(falls_through)
    }

    /// Lower one try side, answering whether it falls through.
    pub(in crate::lower) fn lower_try_arm(
        &mut self,
        body: dir::LocalNodeId<dir::Expression>,
        destination: Option<&Place>,
    ) -> CompilerResult<bool> {
        match destination {
            Some(destination) => self.lower_into(body, destination),
            None => match *self.source().tree().get(body) {
                dir::Expression::Block(block) => Ok(!self.lower_block(block)?),
                _ => Ok(!self.lower_statement(body)?),
            },
        }
    }

    /// Return the residual decision one try projection recorded.
    fn residual_decision(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::ResidualDecision> {
        let node = expression.into_global_any(self.source);
        match self.source().decisions.decision(node).cloned() {
            Some(dir::Decision::Residual(decision)) => Ok(*decision),
            _ => Err(CompilerError::Internal {
                message: "a try projection without its recorded residual".to_string(),
            }),
        }
    }

    /// Split one try implementor through its recorded branch into its residual and its output.
    fn lower_try_branch(
        &mut self,
        value: mir::Value,
        decision: &dir::ResidualDecision,
        on_residual: impl FnOnce(&mut Self, mir::Value) -> CompilerResult<()>,
    ) -> CompilerResult<mir::Value> {
        let Some(branch) = decision.branch.clone() else {
            return Err(CompilerError::Internal {
                message: "a try branch without its recorded call".to_string(),
            });
        };
        let Some(flow) = self.lower_target_call(Operand::Value(value), &branch)? else {
            return Err(CompilerError::Internal {
                message: "a try branch producing no control flow".to_string(),
            });
        };
        let flow = self.read_through_newtypes(flow)?;
        let control = branch.return_type;
        let (breaking, continuing) = self.language_members(
            control,
            dir::LanguageItem::Break,
            dir::LanguageItem::Continue,
        )?;
        let members = self.lower.union_members(control)?;
        let break_case = self.case(&members, breaking)?;
        let continue_case = self.case(&members, continuing)?;

        // hand the residual the breaking case holds to the caller
        let break_block = self.builder.block();
        let continue_block = self.builder.block();
        self.builder.variant_switch(
            flow,
            None,
            vec![(break_case, break_block), (continue_case, continue_block)],
        );
        self.builder.switch_to_block(break_block);
        let payload = self.builder.variant_payload(flow, break_case);
        let index = self.language_member_field(breaking, "residual")?;
        let residual = self.builder.field_get(payload, index);
        on_residual(self, residual)?;

        // continue with the output the continuing case holds
        self.builder.switch_to_block(continue_block);
        let payload = self.builder.variant_payload(flow, continue_case);
        let index = self.language_member_field(continuing, "value")?;

        Ok(self.builder.field_get(payload, index))
    }

    /// Transfer one residual to the enclosing try's catch or the callable's return.
    fn transfer_residual(
        &mut self,
        residual: mir::Value,
        source: dir::GlobalTypeId,
        decision: &dir::ResidualDecision,
    ) -> CompilerResult<()> {
        // land the residual in the enclosing try's catch
        if let dir::ResidualTarget::Try(target) = decision.target {
            let frame = self
                .tries
                .iter()
                .rposition(|frame| frame.node.into_global(self.source) == target);
            let Some(frame) = frame else {
                return Err(CompilerError::Internal {
                    message: "a try residual outside its try expression".to_string(),
                });
            };
            let TryFrame {
                residual: slot,
                catch,
                disposals,
                ..
            } = self.tries[frame];
            if let Some((local, ty)) = slot {
                let caught = self.inject(Operand::Value(residual), source, ty, &[])?;
                self.builder.local_set(local, caught);
            }
            self.dispose_down_to(disposals)?;
            self.builder.jump(catch);

            return Ok(());
        }

        // rebuild the residual for the enclosing function's output
        let transferred = match &decision.from_residual {
            Some(call) => {
                let dir::CallableTarget::Symbol { function, .. } = &call.target else {
                    return Err(CompilerError::Internal {
                        message: "a residual rebuild outside a direct symbol target".to_string(),
                    });
                };
                let callee = self.resolve_callee(&function.key)?;
                let Some(&slot) = self.signature_parameters(callee.signature)?.first() else {
                    return Err(CompilerError::Internal {
                        message: "a residual rebuild without a declared slot".to_string(),
                    });
                };
                let residual = self.adopt(residual, slot)?;
                self.builder.call(
                    callee.target,
                    callee.signature,
                    vec![residual],
                    callee.result,
                )
            }
            None => Some(residual),
        };

        // leave the callable with the transferred residual
        self.dispose_down_to(0)?;

        self.return_value(transferred)
    }

    /// Return the two members of one union that two language items name.
    pub(in crate::lower) fn language_members(
        &mut self,
        union: dir::GlobalTypeId,
        first: dir::LanguageItem,
        second: dir::LanguageItem,
    ) -> CompilerResult<(dir::GlobalTypeId, dir::GlobalTypeId)> {
        let mut found = (None, None);
        for member in self.lower.union_members(union)? {
            let dir::Type::Application(application) = self.lower.ty(member)? else {
                continue;
            };
            let item = self.lower.language_item(application.symbol);
            if item == Some(first) {
                found.0 = Some(member);
            } else if item == Some(second) {
                found.1 = Some(member);
            }
        }
        match found {
            (Some(first), Some(second)) => Ok((first, second)),
            _ => Err(CompilerError::Internal {
                message: "a union without both of its language cases".to_string(),
            }),
        }
    }
}
