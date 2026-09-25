use tspp_dir as dir;
use tspp_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::lower::Binding;
use crate::lower::function::operand::Operand;
use crate::{CompilerError, CompilerResult};

/// One action a scope runs on every way out of it.
#[derive(Clone)]
pub(in crate::lower) enum Disposal {
    /// A using resource awaiting its disposal.
    Resource {
        /// The home holding the resource.
        home: Binding,
        /// The recorded disposal protocol calls.
        decision: Box<dir::DisposalDecision>,
    },
    /// The finally of a try expression.
    Finally(dir::LocalNodeId<dir::Expression>),
}

impl FunctionLowerer<'_, '_, '_> {
    /// Bind the declarators of a using, queueing each resource's disposal for the scope exit.
    pub(in crate::lower) fn lower_using(
        &mut self,
        declarators: &[dir::LocalNodeId<dir::Declarator>],
    ) -> CompilerResult<()> {
        for declarator_id in declarators {
            // require an initializer to acquire the resource from
            let declarator = self.source().tree().get(*declarator_id);
            let (pattern, value) = (declarator.pattern, declarator.value);
            let Some(value) = value else {
                return Err(self.unsupported("an uninitialized using binding"));
            };

            // require a plain binding pattern
            let dir::Pattern::Binding { pattern: None, .. } = self.source().tree().get(pattern)
            else {
                return Err(self.unsupported("a destructuring using binding"));
            };

            // find the symbol the pattern declares
            let node = pattern.into_global_any(self.source);
            let Some(symbol) = self.lower.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "a missing symbol for one using binding".to_string(),
                });
            };

            // home the resource for its disposal borrow
            let value = self.lower_value(value)?;
            let home = self.home_resource(symbol, value)?;

            // queue the recorded disposal
            let anchor = declarator_id.into_global_any(self.source);
            let decision = self
                .lower
                .state(self.source)?
                .decisions
                .disposal_decision(anchor)
                .cloned();
            if let Some(decision) = decision {
                self.queue_disposal(home, decision);
            }
        }

        Ok(())
    }

    /// Home one resource binding in its lifted frame or a frame local.
    pub(in crate::lower) fn home_resource(
        &mut self,
        symbol: dir::GlobalSymbolId,
        value: mir::Value,
    ) -> CompilerResult<Binding> {
        // keep a lifted binding in its frame
        if self.bind_lifted(symbol, value)? {
            return self.values.get(&symbol.local_id).copied().ok_or_else(|| {
                CompilerError::Internal {
                    message: "a lifted using binding without a home".to_string(),
                }
            });
        }

        // otherwise give the resource a frame local
        let local = self.bind_local(symbol, value)?;

        Ok(Binding::Local(local))
    }

    /// Queue one resource's disposal for the scope exit.
    pub(in crate::lower) fn queue_disposal(
        &mut self,
        home: Binding,
        decision: dir::DisposalDecision,
    ) {
        self.disposals.push(Disposal::Resource {
            home,
            decision: Box::new(decision),
        });
    }

    /// Return the disposal depth one scope opens at.
    pub(in crate::lower) fn open_disposals(&self) -> usize {
        self.disposals.len()
    }

    /// Close one scope: dispose its resources on fallthrough and forget them on every path.
    pub(in crate::lower) fn close_disposals(
        &mut self,
        depth: usize,
        is_terminated: bool,
    ) -> CompilerResult<()> {
        // run the disposals the scope opened on its way out
        if !is_terminated {
            self.dispose_down_to(depth)?;
        }

        // forget the scope's resources on every path out of it
        self.disposals.truncate(depth);

        Ok(())
    }

    /// Dispose every resource above one depth in reverse order, the queue kept for other paths.
    pub(in crate::lower) fn dispose_down_to(&mut self, depth: usize) -> CompilerResult<()> {
        // dispose in reverse order of acquisition
        let pending: Vec<Disposal> = self.disposals[depth..].iter().rev().cloned().collect();
        for disposal in pending {
            self.lower_disposal(&disposal)?;
        }

        Ok(())
    }

    /// Run one scope exit action: a present resource's disposal or a finally.
    fn lower_disposal(&mut self, disposal: &Disposal) -> CompilerResult<()> {
        let (home, decision) = match disposal {
            Disposal::Resource { home, decision } => (*home, &**decision),
            Disposal::Finally(body) => {
                if !self.lower_try_arm(*body, None)? {
                    let dead = self.builder.block();
                    self.builder.switch_to_block(dead);
                }

                return Ok(());
            }
        };

        // inspect the tag without moving the resource
        let held = self.binding_representation(home);
        let cases = self.nullish_cases(held);
        let join = if cases.is_empty() {
            None
        } else {
            // switch each nullish case to the absent block
            let place = self.binding_home(home)?;
            let present = self.builder.block();
            let absent = self.builder.block();
            let join = self.builder.block();
            let targets = cases.into_iter().map(|case| (case, absent)).collect();
            self.switch_place(&place, Some(present), targets)?;

            // skip the disposal of an absent resource
            self.builder.switch_to_block(absent);
            self.builder.jump(join);
            self.builder.switch_to_block(present);

            Some(join)
        };

        // call the disposal on the homed resource
        let result = self.lower_home_call(home, &decision.dispose)?;

        // park the completion of an asynchronous disposal
        if let Some(park) = &decision.awaits {
            let Some(completion) = result else {
                return Err(CompilerError::Internal {
                    message: "an asynchronous disposal producing no completion".to_string(),
                });
            };
            let dir::CallableTarget::Symbol { function, .. } = &park.target else {
                return Err(CompilerError::Internal {
                    message: "a disposal await outside a direct symbol target".to_string(),
                });
            };
            let park_function = self.resolve_callee(&function.key)?;
            self.call(&park_function, vec![completion]);
        }

        // rejoin the absent path
        if let Some(join) = join {
            self.builder.jump(join);
            self.builder.switch_to_block(join);
        }

        Ok(())
    }

    /// Call one recorded member on a homed binding, borrowing the home where the receiver borrows.
    fn lower_home_call(
        &mut self,
        home: Binding,
        call: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        match &call.target {
            // borrow the home at the selected receiver form, then call the declared instance
            dir::CallableTarget::Symbol {
                function,
                dispatch: dir::FunctionDispatch::Direct,
            } => {
                let Some(adjusted) = function.receiver.as_ref() else {
                    return Err(CompilerError::Internal {
                        message: "a disposal call without a selected receiver".to_string(),
                    });
                };
                let place = self.binding_home(home)?;
                let receiver = self.adjust_receiver(Operand::Place(place), adjusted)?;

                self.lower_direct_call(Some(receiver), &function.key, call)
            }
            // dispatch through the erased resource's constraint slot
            dir::CallableTarget::Dynamic {
                dispatch,
                function: dir::DynamicFunction::Symbol(symbol),
                ..
            } => {
                let receiver = self.read_binding(home)?;
                let receiver =
                    self.lower_receiver_adjustments(receiver, &dispatch.receiver.adjustments)?;

                self.lower_dynamic_symbol_call(receiver, dispatch, *symbol, call)
            }
            _ => Err(self.unsupported("a virtual disposal call")),
        }
    }

    /// Return the representation one binding's home holds.
    pub(in crate::lower) fn binding_representation(&self, home: Binding) -> mir::TypeId {
        match home {
            Binding::Local(local) => self.builder.tree().get(local).ty,
            Binding::Captured { ty, .. } => ty,
        }
    }

    /// Branch a nullish value into its present and absent blocks.
    pub(in crate::lower) fn split_absent(
        &mut self,
        value: mir::Value,
    ) -> CompilerResult<Option<(mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>)>> {
        // leave a representation without nullish cases present
        let received = self.value_representation(value)?;
        let cases = self.nullish_cases(received);
        if cases.is_empty() {
            return Ok(None);
        }

        // switch the variant on each of its nullish cases
        let present = self.builder.block();
        let absent = self.builder.block();
        let targets = cases.into_iter().map(|case| (case, absent)).collect();
        self.builder.variant_switch(value, Some(present), targets);

        Ok(Some((present, absent)))
    }
}
