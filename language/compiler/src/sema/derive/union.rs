use destack_dir as dir;

use crate::sema::derive::{Component, Derivation};
use crate::sema::{CheckState, Origin};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Build `if (this is A) { ... } else if (this is B) { ... }`, one branch per union member.
    pub(super) fn build_union_body(
        &mut self,
        frame: &mut Derivation,
        origin: Origin,
        members: &[Component],
        interface: dir::AutoInterface,
        result: dir::GlobalTypeId,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // a hash body runs for its writes, every other body for its value
        let void = self.intern_type(dir::Type::Void)?;
        let is_statement = interface == dir::AutoInterface::Hash;
        let branch_type = if is_statement { void } else { result };

        // build the chain from the last member outward
        let mut chain = None;
        for (index, member) in members.iter().enumerate().rev() {
            let branch = match interface {
                dir::AutoInterface::Equal | dir::AutoInterface::PartialEqual => {
                    self.build_member_equal(frame, origin, member)?
                }
                dir::AutoInterface::Hash => self.build_member_hash(frame, origin, member, index)?,
                dir::AutoInterface::Clone => self.build_member_clone(frame, origin, member)?,
                dir::AutoInterface::Debug | dir::AutoInterface::Display => {
                    self.build_member_text(frame, origin, member, result)?
                }
                _ => {
                    return Err(CompilerError::Internal {
                        message: "a union deriving an interface without a member rule".to_owned(),
                    });
                }
            };
            chain = Some(match chain {
                None => branch,
                Some(rest) => {
                    let this = self.build_this(frame)?;
                    let test = self.build_is(frame, origin, this, member.ty)?;

                    self.build_if(frame, test, branch, Some(rest), branch_type)?
                }
            });
        }

        // require the union to declare at least one member
        let Some(chain) = chain else {
            return Err(CompilerError::Internal {
                message: "a union derivation without members".to_owned(),
            });
        };

        // wrap the chain in a block when the derivation stands as a statement
        match is_statement {
            true => self.build_block(frame, vec![chain], None, void),
            false => Ok(chain),
        }
    }

    /// Build `if (other is A) { this.equal(&other) } else { false }` for one member.
    fn build_member_equal(
        &mut self,
        frame: &mut Derivation,
        origin: Origin,
        member: &Component,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // read the peer parameter the comparison takes
        let boolean = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;
        let peer_type = frame.parameters[0].2;

        // equate the members through their protocol, unit members by their tag alone
        let equal = match member.call {
            Some(_) => {
                let this = self.build_narrowed_this(frame, origin, member.ty)?;
                let other = self.build_narrowed_parameter(frame, origin, 0, member.ty)?;
                let borrowed_type = self.borrowed_like(Some(peer_type), member.ty)?;
                let borrowed = self.build_borrow(frame, other, borrowed_type)?;

                self.build_component_call(frame, this, member, vec![borrowed])?
            }
            None => self.build_literal(frame, dir::Literal::Boolean(true), boolean)?,
        };

        // test the peer for the same member, a differing peer comparing false
        let matched = self.build_block(frame, Vec::new(), Some(equal), boolean)?;
        let unmatched = self.build_literal(frame, dir::Literal::Boolean(false), boolean)?;
        let unmatched = self.build_block(frame, Vec::new(), Some(unmatched), boolean)?;
        let peer = self.build_parameter(frame, 0)?;
        let test = self.build_is(frame, origin, peer, member.ty)?;
        let branch = self.build_if(frame, test, matched, Some(unmatched), boolean)?;

        self.build_block(frame, Vec::new(), Some(branch), boolean)
    }

    /// Build `index.hash(state); this.hash(state);` for one member.
    fn build_member_hash(
        &mut self,
        frame: &mut Derivation,
        origin: Origin,
        member: &Component,
        index: usize,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // read the state parameter the hash writes into
        let void = self.intern_type(dir::Type::Void)?;

        // write the member's position, then the value it holds
        let mut leading = Vec::with_capacity(2);
        leading.push(self.build_tag_hash(frame, origin, index)?);
        if member.call.is_some() {
            let this = self.build_narrowed_this(frame, origin, member.ty)?;
            let state = self.build_parameter(frame, 0)?;
            let hashed = self.build_component_call(frame, this, member, vec![state])?;
            leading.push(hashed);
        }

        self.build_block(frame, leading, None, void)
    }

    /// Build `this.clone()` for one member, entering the union again.
    fn build_member_clone(
        &mut self,
        frame: &mut Derivation,
        origin: Origin,
        member: &Component,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // clone the narrowed value, a unit member standing for itself
        let value = match member.call {
            Some(_) => {
                let this = self.build_narrowed_this(frame, origin, member.ty)?;

                self.build_component_call(frame, this, member, Vec::new())?
            }
            None => self.build_unit_value(frame, member.ty)?,
        };

        // enter the union at the member the value holds
        let case = dir::CoercionCase {
            source: member.ty,
            target: member.ty,
            adjustments: Vec::new(),
        };
        let coercion = dir::Coercion::union(
            member.ty,
            frame.receiver,
            vec![case],
            dir::CastOrigin::Implicit,
        );
        self.commit_coercion(value.into_global_any(frame.module), coercion)?;

        self.build_block(frame, Vec::new(), Some(value), frame.receiver)
    }

    /// Build `this.display()` for one member, unit members rendering their own text.
    fn build_member_text(
        &mut self,
        frame: &mut Derivation,
        origin: Origin,
        member: &Component,
        result: dir::GlobalTypeId,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // render the narrowed value, a unit member rendering its own name
        let value = match member.call {
            Some(_) => {
                let this = self.build_narrowed_this(frame, origin, member.ty)?;

                self.build_component_call(frame, this, member, Vec::new())?
            }
            None => {
                let string = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::String))?;
                let text = self.format_type(member.ty);
                let text = self.strings().intern(&text);
                let literal = self.build_literal(frame, dir::Literal::String(text), string)?;

                self.build_owned_text(frame, origin, literal, string, result)?
            }
        };

        self.build_block(frame, Vec::new(), Some(value), result)
    }

    /// Build `this` narrowed to one union member.
    fn build_narrowed_this(
        &mut self,
        frame: &mut Derivation,
        origin: Origin,
        member: dir::GlobalTypeId,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // require the frame's receiver
        let Some(this) = frame.this else {
            return Err(CompilerError::Internal {
                message: "a derived static body reading this".to_owned(),
            });
        };

        // read this at the member's own type and record the narrowing
        let narrowed = self.replace_form_value(origin, this, member)?;
        let node = self.build_expression(frame, dir::Expression::This, narrowed)?;
        self.commit_member_narrowing(frame, node, member);

        Ok(node)
    }

    /// Build a read of one parameter narrowed to one union member.
    fn build_narrowed_parameter(
        &mut self,
        frame: &mut Derivation,
        origin: Origin,
        index: usize,
        member: dir::GlobalTypeId,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // read the parameter at the member's own type
        let (symbol, name, ty) = frame.parameters[index];
        let narrowed = self.replace_form_value(origin, ty, member)?;
        let node = self.build_name(frame, symbol.into_global(frame.module), name, narrowed)?;

        // record the narrowing the read sees
        self.commit_member_narrowing(frame, node, member);

        Ok(node)
    }

    /// Record the narrowing one read sees onto a single union member.
    fn commit_member_narrowing(
        &mut self,
        frame: &mut Derivation,
        node: dir::LocalNodeId<dir::Expression>,
        member: dir::GlobalTypeId,
    ) {
        self.module_mut(frame.module).decisions_tail.set_narrowing(
            node.into_global_any(frame.module),
            dir::Narrowing {
                union: frame.receiver,
                arms: vec![member],
            },
        );
    }

    /// Build `value is Member` over one read of the receiver or its peer.
    fn build_is(
        &mut self,
        frame: &mut Derivation,
        origin: Origin,
        value: dir::LocalNodeId<dir::Expression>,
        member: dir::GlobalTypeId,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // select the predicate the test runs
        let boolean = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;
        let value_type = self.require_node_type(value.into_global_any(frame.module))?;
        let target = self.build_type_expression(frame, member, member)?;
        let predicate = self.select_guard_predicate(
            origin,
            value_type,
            member,
            target.into_global_any(frame.module),
        )?;

        // record the guard decision on the synthesized test
        let node = self.build_expression(
            frame,
            dir::Expression::Is {
                value,
                target_type: target,
            },
            boolean,
        )?;
        self.commit_decision(
            node.into_global_any(frame.module),
            dir::Decision::Guard(dir::GuardDecision::Is(dir::IsGuardDecision {
                value_type,
                target_type: member,
                predicate,
            })),
        )?;

        Ok(node)
    }
}
