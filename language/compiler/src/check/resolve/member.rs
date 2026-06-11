use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckEvent, CheckState, Dependency, GenericArgument, GenericParameterId,
    MemberCandidate, MemberProtocol, MemberReceiver, MemberSpace, MemberTerm, Origin, ShapeMember,
    ShapeTerm, TermId, TraceMemberCandidate, TraceMemberLookup, TypeOperand, TypeOperationTerm,
    TypeRelation, TypeTerm,
};

/// Result of resolving one member on a receiver type.
pub(in crate::check) enum MemberLookup {
    /// Member resolution is waiting for dependency.
    Pending(SmallVec<[Dependency; 2]>),
    /// No member exists.
    Missing,
    /// One structural field exists.
    Field(TypeOperand),
    /// One or more symbol-backed members exist.
    Found(Vec<MemberCandidate>),
}

impl MemberLookup {
    /// Return a member resolution from collected candidates.
    pub(in crate::check) fn from_candidates(candidates: Vec<MemberCandidate>) -> Self {
        if candidates.is_empty() {
            Self::Missing
        } else {
            Self::Found(candidates)
        }
    }

    /// Return this resolution's trace outcome.
    pub(in crate::check) fn trace(&self) -> TraceMemberLookup {
        match self {
            Self::Pending(_) => TraceMemberLookup::Pending,
            Self::Missing => TraceMemberLookup::Missing,
            Self::Field(_) => TraceMemberLookup::Field,
            Self::Found(candidates) => TraceMemberLookup::Found {
                candidates: candidates
                    .iter()
                    .map(|candidate| TraceMemberCandidate {
                        symbol: candidate.symbol,
                        value: candidate.ty,
                    })
                    .collect(),
            },
        }
    }
}

impl CheckState<'_> {
    /// Resolve members from one type term.
    pub(in crate::check) fn resolve_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: &MemberReceiver,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let effective_receiver = self.member_receiver_type_operand(receiver);
        let lookup = match receiver {
            MemberReceiver::Value(receiver) => {
                let Answer::Ready(receiver) = self.reduce_type_operand(origin, *receiver)? else {
                    let blockers = effective_receiver.dependencies(self);
                    self.record_event(CheckEvent::MemberLookup {
                        origin,
                        receiver: effective_receiver,
                        key: *key,
                        result: TraceMemberLookup::Pending,
                    });

                    return Ok(MemberLookup::Pending(blockers));
                };

                self.resolve_type_member(origin, module, receiver, key)?
            }
            MemberReceiver::GenericParameter(parameter) => self
                .resolve_generic_constraint_static_member(
                    module,
                    origin,
                    effective_receiver,
                    *parameter,
                    key,
                )?,
            MemberReceiver::Declaration {
                origin: _,
                symbol,
                arguments,
            } => self.resolve_symbol_member(
                origin,
                module,
                effective_receiver,
                *symbol,
                arguments,
                MemberSpace::Static,
                *key,
            )?,
        };

        self.record_event(CheckEvent::MemberLookup {
            origin,
            receiver: effective_receiver,
            key: *key,
            result: lookup.trace(),
        });

        Ok(lookup)
    }

    /// Resolve members that satisfy one protocol.
    pub(in crate::check) fn resolve_protocol_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: TypeOperand,
        key: &dir::StaticKey,
        protocol: &MemberProtocol,
    ) -> CompilerResult<MemberLookup> {
        let Answer::Ready(receiver) = self.reduce_type_operand(origin, receiver)? else {
            return Ok(MemberLookup::Pending(receiver.dependencies(self)));
        };

        match self.decide_member_protocol(receiver, protocol)? {
            Answer::Ready(true) => {
                let receiver = MemberReceiver::Value(receiver);

                self.resolve_member(origin, module, &receiver, key)
            }
            Answer::Pending(blockers) => Ok(MemberLookup::Pending(blockers)),
            Answer::Ready(false) => Ok(MemberLookup::Missing),
        }
    }

    /// Resolve members from one type operand.
    fn resolve_type_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: TypeOperand,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let Some(term) = self.type_operand_term_id(receiver)? else {
            return Ok(MemberLookup::Pending(receiver.dependencies(self)));
        };

        if let TypeTerm::Form { payload, .. } = self.inference.term(term) {
            let payload = *payload;
            let Answer::Ready(payload) = self.reduce_type_operand(origin, payload)? else {
                return Ok(MemberLookup::Pending(payload.dependencies(self)));
            };

            return self.resolve_type_member(origin, module, payload, key);
        }

        if let TypeTerm::Reference {
            origin: _,
            symbol,
            arguments,
        } = self.inference.term(term)
        {
            let symbol = *symbol;
            let arguments = arguments
                .iter()
                .copied()
                .collect::<SmallVec<[GenericArgument; 2]>>();

            // resolve members through a type generic parameter
            if arguments.is_empty()
                && let Some(parameter) = self.inference.generic_parameter_by_symbol(symbol)
                && self
                    .inference
                    .generic_parameter_binding(parameter)?
                    .is_type()
            {
                return self.resolve_generic_constraint_member(module, origin, parameter, key);
            }

            // resolve members through an ordinary symbol reference
            return self.resolve_symbol_member(
                origin,
                module,
                receiver,
                symbol,
                &arguments,
                MemberSpace::Instance,
                *key,
            );
        }

        if let TypeTerm::Parameter(parameter) = self.inference.term(term) {
            return self.resolve_generic_constraint_member(module, origin, *parameter, key);
        }

        if let TypeTerm::Shape(shape) = self.inference.term(term) {
            return self.resolve_shape_member(*shape, key);
        }

        if let TypeTerm::Tuple { .. } = self.inference.term(term) {
            return self.resolve_tuple_member(term, key);
        }

        if let TypeTerm::Union { .. } = self.inference.term(term) {
            return self.resolve_union_member(origin, module, term, key);
        }

        if let TypeTerm::Operation(operation) = self.inference.term(term) {
            return self.resolve_operation_member(origin, module, *operation, key);
        }

        Ok(MemberLookup::Missing)
    }

    /// Resolve members through a generic parameter constraint.
    fn resolve_generic_constraint_member(
        &mut self,
        module: ModuleId,
        origin: Origin,
        parameter: GenericParameterId,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let Some(constraint) = self.type_generic_constraint(parameter)? else {
            return Ok(MemberLookup::Missing);
        };
        let Some(constraint) = self.reduce_generic_constraint(origin, constraint)? else {
            return Ok(MemberLookup::Pending(constraint.dependencies(self)));
        };

        self.resolve_type_member(origin, module, constraint, key)
    }

    /// Resolve static members through a generic parameter constraint.
    fn resolve_generic_constraint_static_member(
        &mut self,
        module: ModuleId,
        origin: Origin,
        receiver: TypeOperand,
        parameter: GenericParameterId,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let Some(constraint) = self.type_generic_constraint(parameter)? else {
            return Ok(MemberLookup::Missing);
        };
        let Some(constraint) = self.reduce_generic_constraint(origin, constraint)? else {
            return Ok(MemberLookup::Pending(constraint.dependencies(self)));
        };

        self.resolve_static_type_member(origin, module, receiver, constraint, key)
    }

    /// Resolve static members from one constrained receiver type.
    pub(in crate::check) fn resolve_static_type_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: TypeOperand,
        constraint: TypeOperand,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let Some(term) = self.type_operand_term_id(constraint)? else {
            return Ok(MemberLookup::Pending(constraint.dependencies(self)));
        };

        if let TypeTerm::Form { payload, .. } = self.inference.term(term) {
            let payload = *payload;
            let Answer::Ready(payload) = self.reduce_type_operand(origin, payload)? else {
                return Ok(MemberLookup::Pending(payload.dependencies(self)));
            };

            return self.resolve_static_type_member(origin, module, receiver, payload, key);
        }

        if let TypeTerm::Reference {
            origin: _,
            symbol,
            arguments,
        } = self.inference.term(term)
        {
            let symbol = *symbol;
            let arguments = arguments
                .iter()
                .copied()
                .collect::<SmallVec<[GenericArgument; 2]>>();

            return self.resolve_symbol_member(
                origin,
                module,
                receiver,
                symbol,
                &arguments,
                MemberSpace::Static,
                *key,
            );
        }

        if let TypeTerm::Union { .. } = self.inference.term(term) {
            return self.resolve_union_static_member(origin, module, receiver, term, key);
        }

        if let TypeTerm::Operation(operation) = self.inference.term(term) {
            return self.resolve_operation_static_member(origin, module, receiver, *operation, key);
        }

        Ok(MemberLookup::Missing)
    }

    /// Resolve a structural member from one shape field.
    fn resolve_shape_member(
        &self,
        shape: TermId<ShapeTerm>,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        for member in &self.inference.term(shape).members {
            let ShapeMember::Field {
                key: member_key,
                ty,
                ..
            } = member
            else {
                continue;
            };
            if !member_key.matches(key) {
                continue;
            };
            return Ok(MemberLookup::Field(*ty));
        }

        Ok(MemberLookup::Missing)
    }

    /// Resolve a structural member from one tuple element.
    fn resolve_tuple_member(
        &self,
        tuple: TermId<TypeTerm>,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let dir::StaticKey::Index(index) = key else {
            return Ok(MemberLookup::Missing);
        };
        let element = match self.inference.term(tuple) {
            TypeTerm::Tuple { elements, .. } => elements.get(*index).copied(),
            _ => None,
        };
        let Some(element) = element else {
            return Ok(MemberLookup::Missing);
        };

        Ok(MemberLookup::Field(element.ty))
    }

    /// Decide whether one type satisfies one member protocol.
    fn decide_member_protocol(
        &mut self,
        receiver: TypeOperand,
        protocol: &MemberProtocol,
    ) -> CompilerResult<Answer<bool>> {
        let symbol = self.language_symbol(protocol.item);
        let target = TypeTerm::Reference {
            origin: Origin::Symbol(symbol),
            symbol,
            arguments: protocol.arguments.iter().copied().collect(),
        };

        let target = self.inference.push_term(target);

        self.decide_type_relation(TypeRelation::Satisfies, receiver, target)
    }

    /// Return the member resolution shared by every union element.
    pub(in crate::check) fn resolve_union_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        union: TermId<TypeTerm>,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let len = self.union_member_len(union);
        let mut candidates = Vec::with_capacity(len);
        let mut fields = Vec::with_capacity(len);

        // collect one member resolution per union element
        for index in 0..len {
            let Some(element) = self.union_member_element(union, index) else {
                return Ok(MemberLookup::Missing);
            };
            let receiver = MemberReceiver::Value(element);

            match self.resolve_member(origin, module, &receiver, key)? {
                MemberLookup::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
                MemberLookup::Missing => return Ok(MemberLookup::Missing),
                MemberLookup::Found(mut members) => {
                    candidates.append(&mut members);
                }
                MemberLookup::Field(member) => fields.push(member),
            };
        }

        // return symbol-backed members when every branch had symbols
        if fields.is_empty() {
            return Ok(MemberLookup::from_candidates(candidates));
        }

        // return a structural union when every branch had a field
        if candidates.is_empty() {
            return Ok(self.resolve_union_field(fields));
        }

        // join symbol-backed and structural branches as field types
        let mut fields = fields;
        for candidate in candidates {
            match candidate.ty {
                Some(ty) => fields.push(ty),
                None => {
                    let member = MemberTerm {
                        origin,
                        receiver: MemberReceiver::Value(candidate.receiver),
                        key: *key,
                        arguments: SmallVec::new(),
                    };
                    let member = self.inference.push_term(member);
                    let member = self.inference.push_term(TypeTerm::Member(member));

                    fields.push(member.into());
                }
            }
        }

        Ok(self.resolve_union_field(fields))
    }

    /// Return a union member from structural field types.
    fn resolve_union_field(&mut self, fields: Vec<TypeOperand>) -> MemberLookup {
        if let [field] = fields.as_slice() {
            let field = *field;

            return MemberLookup::Field(field);
        }

        let field = self
            .inference
            .push_term(TypeTerm::Union { elements: fields });

        MemberLookup::Field(field.into())
    }

    /// Return member resolution from one type operation.
    pub(in crate::check) fn resolve_operation_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        operation: TermId<TypeOperationTerm>,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let operation_term = self.inference.term(operation);
        let conditional = match operation_term {
            TypeOperationTerm::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => Some((*left, *right, *then_type, *else_type)),
            _ => None,
        };

        if let Some((left, right, then_type, else_type)) = conditional {
            self.resolve_conditional_member(origin, module, left, right, then_type, else_type, key)
        } else if operation_term.must_reduce_for_member_resolution() {
            self.resolve_reduced_operation_member(origin, module, operation, key)
        } else {
            Ok(MemberLookup::Missing)
        }
    }

    /// Return member resolution after reducing one type operation.
    fn resolve_reduced_operation_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        operation: TermId<TypeOperationTerm>,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let operation = self.inference.push_term(TypeTerm::Operation(operation));
        let operand = TypeOperand::Term(operation);
        let Answer::Ready(receiver) = self.reduce_type_operand(origin, operand)? else {
            return Ok(MemberLookup::Pending(operand.dependencies(self)));
        };
        let receiver = MemberReceiver::Value(receiver);

        self.resolve_member(origin, module, &receiver, key)
    }

    /// Return member resolution from one conditional type operation.
    fn resolve_conditional_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        left: TypeOperand,
        right: TypeOperand,
        then_type: TypeOperand,
        else_type: TypeOperand,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let decision = self.decide_type_relation(TypeRelation::Extends, left, right)?;

        match decision {
            // use the known true branch
            Answer::Ready(true) => {
                let receiver = MemberReceiver::Value(then_type);

                self.resolve_member(origin, module, &receiver, key)
            }
            // use the known false branch
            Answer::Ready(false) => {
                let receiver = MemberReceiver::Value(else_type);

                self.resolve_member(origin, module, &receiver, key)
            }
            // join both possible branches
            Answer::Pending(_) => {
                let then_receiver = MemberReceiver::Value(then_type);
                let else_receiver = MemberReceiver::Value(else_type);
                let then_lookup = self.resolve_member(origin, module, &then_receiver, key)?;
                let else_lookup = self.resolve_member(origin, module, &else_receiver, key)?;

                Ok(self.join_member_lookup(origin, key, then_lookup, else_lookup))
            }
        }
    }

    /// Return static member resolution shared by every union element.
    pub(in crate::check) fn resolve_union_static_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: TypeOperand,
        union: TermId<TypeTerm>,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let len = self.union_member_len(union);
        let mut candidates = Vec::with_capacity(len);

        // collect static members from each possible constraint
        for index in 0..len {
            let Some(element) = self.union_member_element(union, index) else {
                return Ok(MemberLookup::Missing);
            };

            match self.resolve_static_type_member(origin, module, receiver, element, key)? {
                MemberLookup::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
                MemberLookup::Missing | MemberLookup::Field(_) => {
                    return Ok(MemberLookup::Missing);
                }
                MemberLookup::Found(mut members) => {
                    candidates.append(&mut members);
                }
            };
        }

        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Return one union element count.
    fn union_member_len(&self, union: TermId<TypeTerm>) -> usize {
        match self.inference.term(union) {
            TypeTerm::Union { elements } => elements.len(),
            _ => 0,
        }
    }

    /// Return one copied union element.
    fn union_member_element(&self, union: TermId<TypeTerm>, index: usize) -> Option<TypeOperand> {
        match self.inference.term(union) {
            TypeTerm::Union { elements } => elements.get(index).copied(),
            _ => None,
        }
    }

    /// Return static member resolution from one type operation.
    pub(in crate::check) fn resolve_operation_static_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: TypeOperand,
        operation: TermId<TypeOperationTerm>,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let operation_term = self.inference.term(operation);
        let conditional = match operation_term {
            TypeOperationTerm::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => Some((*left, *right, *then_type, *else_type)),
            _ => None,
        };

        if let Some((left, right, then_type, else_type)) = conditional {
            self.resolve_conditional_static_member(
                origin, module, receiver, left, right, then_type, else_type, key,
            )
        } else if operation_term.must_reduce_for_member_resolution() {
            self.resolve_reduced_operation_static_member(origin, module, receiver, operation, key)
        } else {
            Ok(MemberLookup::Missing)
        }
    }

    /// Return static member resolution after reducing one type operation.
    fn resolve_reduced_operation_static_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: TypeOperand,
        operation: TermId<TypeOperationTerm>,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let operation = self.inference.push_term(TypeTerm::Operation(operation));
        let operand = TypeOperand::Term(operation);
        let Answer::Ready(constraint) = self.reduce_type_operand(origin, operand)? else {
            return Ok(MemberLookup::Pending(operand.dependencies(self)));
        };

        self.resolve_static_type_member(origin, module, receiver, constraint, key)
    }

    /// Return static member resolution from one conditional type operation.
    #[allow(clippy::too_many_arguments)]
    fn resolve_conditional_static_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: TypeOperand,
        left: TypeOperand,
        right: TypeOperand,
        then_type: TypeOperand,
        else_type: TypeOperand,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let decision = self.decide_type_relation(TypeRelation::Extends, left, right)?;

        match decision {
            // use the known true branch
            Answer::Ready(true) => {
                self.resolve_static_type_member(origin, module, receiver, then_type, key)
            }
            // use the known false branch
            Answer::Ready(false) => {
                self.resolve_static_type_member(origin, module, receiver, else_type, key)
            }
            // join both possible branches
            Answer::Pending(_) => {
                let then_lookup =
                    self.resolve_static_type_member(origin, module, receiver, then_type, key)?;
                let else_lookup =
                    self.resolve_static_type_member(origin, module, receiver, else_type, key)?;

                Ok(self.join_member_lookup(origin, key, then_lookup, else_lookup))
            }
        }
    }

    /// Return a member resolution from two conditional branches.
    fn join_member_lookup(
        &mut self,
        origin: Origin,
        key: &dir::StaticKey,
        left: MemberLookup,
        right: MemberLookup,
    ) -> MemberLookup {
        match (left, right) {
            // preserve pending branches
            (MemberLookup::Pending(blockers), _) | (_, MemberLookup::Pending(blockers)) => {
                MemberLookup::Pending(blockers)
            }
            // require both branches to define the member
            (MemberLookup::Missing, _) | (_, MemberLookup::Missing) => MemberLookup::Missing,
            // join symbol-backed branches
            (MemberLookup::Found(mut left), MemberLookup::Found(mut right)) => {
                left.append(&mut right);

                MemberLookup::from_candidates(left)
            }
            // join structural branches
            (MemberLookup::Field(left), MemberLookup::Field(right)) => {
                self.join_field_lookup(left, right)
            }
            // join symbol and field branches as an effective field
            (MemberLookup::Found(candidates), MemberLookup::Field(member))
            | (MemberLookup::Field(member), MemberLookup::Found(candidates)) => {
                self.join_candidate_field_lookup(origin, key, candidates, member)
            }
        }
    }

    /// Return a member resolution from two structural field types.
    fn join_field_lookup(&mut self, left: TypeOperand, right: TypeOperand) -> MemberLookup {
        if left == right {
            return MemberLookup::Field(left);
        }

        let field = self.inference.push_term(TypeTerm::Union {
            elements: vec![left, right],
        });

        MemberLookup::Field(field.into())
    }

    /// Return a member resolution from symbol candidates and one structural field.
    fn join_candidate_field_lookup(
        &mut self,
        origin: Origin,
        key: &dir::StaticKey,
        candidates: Vec<MemberCandidate>,
        member: TypeOperand,
    ) -> MemberLookup {
        let mut members = Vec::with_capacity(candidates.len() + 1);
        for candidate in candidates {
            match candidate.ty {
                Some(ty) => members.push(ty),
                None => {
                    let member = MemberTerm {
                        origin,
                        receiver: MemberReceiver::Value(candidate.receiver),
                        key: *key,
                        arguments: SmallVec::new(),
                    };
                    let member = self.inference.push_term(member);
                    let member = self.inference.push_term(TypeTerm::Member(member));

                    members.push(member.into());
                }
            }
        }

        members.push(member);

        self.resolve_union_field(members)
    }
}
