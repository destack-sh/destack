use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, CandidateResolution, CheckEvent, CheckState, GenericArgument, GenericParameterId,
    MemberCandidate, MemberDecision, MemberFailure, MemberLookup, MemberResolution,
    MemberTargetResolution, Origin, StaticOperand, SubstitutionSet, TypeLiteralTerm, TypeOperand,
    TypeTerm,
};
use crate::{CompilerError, CompilerResult};

/// Receiver used by member resolution.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum MemberReceiver {
    /// Runtime value receiver.
    ///
    /// ```ds
    /// value.member
    /// ```
    Value(TypeOperand),
    /// Generic type parameter declaration receiver.
    ///
    /// ```ds
    /// T.default()
    /// ```
    GenericParameter(GenericParameterId),
    /// Type declaration receiver.
    ///
    /// ```ds
    /// Promise.resolve(value)
    /// ```
    Declaration {
        /// The work origin that introduced this receiver.
        origin: Origin,
        /// The declaration symbol.
        symbol: dir::GlobalSymbolId,
        /// The applied static arguments.
        arguments: SmallVec<[GenericArgument; 2]>,
    },
}

impl MemberReceiver {
    /// Substitute generic arguments through this receiver.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        let receiver = match self {
            Self::Value(operand) => {
                Self::Value(state.substitute_type_operand(module, substitution, *operand)?)
            }
            Self::GenericParameter(parameter) => {
                if let Some(operand) = state.substitution_type_operand(substitution, *parameter) {
                    state.member_receiver_from_type_operand(operand)?
                } else {
                    Self::GenericParameter(*parameter)
                }
            }
            Self::Declaration {
                origin,
                symbol,
                arguments,
            } => Self::Declaration {
                origin: *origin,
                symbol: *symbol,
                arguments: state
                    .substitute_arguments(module, substitution, arguments)?
                    .into_iter()
                    .collect(),
            },
        };

        Ok(receiver)
    }
}

/// Type member projection term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MemberTerm {
    /// The work origin that introduced this member projection.
    pub(in crate::check) origin: Origin,
    /// The member receiver.
    pub(in crate::check) receiver: MemberReceiver,
    /// The selected member key.
    pub(in crate::check) key: dir::StaticKey,
    /// The applied static arguments.
    pub(in crate::check) arguments: SmallVec<[GenericArgument; 2]>,
}

impl MemberTerm {
    /// Substitute generic arguments through this member projection.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        let member = Self {
            origin: self.origin,
            receiver: self.receiver.substitute(module, substitution, state)?,
            key: self.key,
            arguments: state.substitute_arguments(module, substitution, &self.arguments)?,
        };

        Ok(member)
    }
}

impl CheckState<'_> {
    /// Return one member receiver from a type operand.
    pub(in crate::check) fn member_receiver_from_type_operand(
        &mut self,
        operand: TypeOperand,
    ) -> CompilerResult<MemberReceiver> {
        let Some(term) = self.type_operand_term_id(operand)? else {
            return Ok(MemberReceiver::Value(operand));
        };

        let receiver = match self.inference.term(term) {
            TypeTerm::Parameter(parameter) => MemberReceiver::GenericParameter(*parameter),
            TypeTerm::Reference {
                origin,
                symbol,
                arguments,
            } => MemberReceiver::Declaration {
                origin: *origin,
                symbol: *symbol,
                arguments: arguments.iter().copied().collect(),
            },
            _ => MemberReceiver::Value(operand),
        };

        Ok(receiver)
    }

    /// Return the effective type operand for one member receiver.
    pub(in crate::check) fn member_receiver_type_operand(
        &mut self,
        receiver: &MemberReceiver,
    ) -> TypeOperand {
        match receiver {
            MemberReceiver::Value(operand) => *operand,
            MemberReceiver::GenericParameter(parameter) => self
                .inference
                .push_term(TypeTerm::Parameter(*parameter))
                .into(),
            MemberReceiver::Declaration {
                origin,
                symbol,
                arguments,
            } => self
                .inference
                .push_term(TypeTerm::Reference {
                    origin: *origin,
                    symbol: *symbol,
                    arguments: arguments.iter().copied().collect(),
                })
                .into(),
        }
    }

    /// Reduce one member projection to its type.
    pub(in crate::check) fn reduce_member_term(
        &mut self,
        module: ModuleId,
        source: Origin,
        receiver: MemberReceiver,
        key: dir::StaticKey,
        arguments: &[GenericArgument],
        projection: Option<TypeOperand>,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let receiver_type = self.member_receiver_type_operand(&receiver);
        let lookup = self.resolve_member(source, module, &receiver, &key)?;
        if let MemberLookup::Pending(blockers) = lookup {
            return Ok(Answer::Pending(blockers));
        }

        // select solved member for commit and diagnostics
        if let Origin::Node(source) = source
            && self.inference.member(source).is_none()
        {
            self.select_member_resolution(source, receiver_type, key, &lookup)?;
        }

        let Some(term) =
            self.member_type_from_lookup(module, source, key, arguments, lookup, projection)?
        else {
            self.record_event(CheckEvent::MemberReduce {
                origin: source,
                key,
                value: None,
            });

            return Err(CompilerError::Internal {
                message: "resolved member lookup has no readable type".into(),
            });
        };

        self.record_event(CheckEvent::MemberReduce {
            origin: source,
            key,
            value: Some(term),
        });

        Ok(Answer::Ready(term))
    }

    /// Resolve a member type from one reduced receiver operand.
    pub(in crate::check) fn resolve_member_type_operand(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: TypeOperand,
        key: &dir::StaticKey,
        member_arguments: &[GenericArgument],
    ) -> CompilerResult<Option<TypeOperand>> {
        let receiver = MemberReceiver::Value(receiver);
        let lookup = self.resolve_member(origin, module, &receiver, key)?;

        self.member_type_from_lookup(module, origin, *key, member_arguments, lookup, None)
    }

    /// Return a member type from a resolved member result.
    pub(in crate::check) fn member_type_from_lookup(
        &mut self,
        module: ModuleId,
        origin: Origin,
        key: dir::StaticKey,
        member_arguments: &[GenericArgument],
        lookup: MemberLookup,
        projection: Option<TypeOperand>,
    ) -> CompilerResult<Option<TypeOperand>> {
        match lookup {
            MemberLookup::Pending(_) => Ok(None),
            MemberLookup::Field(member) => {
                if member_arguments.is_empty() {
                    Ok(Some(member))
                } else {
                    Ok(Some(self.type_term_operand(TypeTerm::Literal(
                        TypeLiteralTerm::Error,
                    ))))
                }
            }
            MemberLookup::Found(candidates) => self.member_candidate_types(
                module,
                origin,
                key,
                candidates,
                member_arguments,
                projection,
            ),
            MemberLookup::Missing => Ok(Some(
                self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Error)),
            )),
        }
    }

    /// Select the committed member resolution for one result.
    fn select_member_resolution(
        &mut self,
        source: dir::GlobalNodeIdAny,
        receiver: TypeOperand,
        key: dir::StaticKey,
        lookup: &MemberLookup,
    ) -> CompilerResult<()> {
        match lookup {
            // wait for a later solve step
            MemberLookup::Pending(_) => Ok(()),
            // select structural fields
            MemberLookup::Field(_) => {
                let member = MemberResolution {
                    source,
                    receiver,
                    target: MemberTargetResolution::Field(key),
                };

                self.inference
                    .select_member(source, MemberDecision::Resolved(member))
            }
            // select symbol-backed member candidates
            MemberLookup::Found(candidates) => {
                let target = self.member_target_resolution(candidates);
                let member = MemberResolution {
                    source,
                    receiver,
                    target,
                };

                self.inference
                    .select_member(source, MemberDecision::Resolved(member))
            }
            // select closed missing members
            MemberLookup::Missing => self.inference.select_member(
                source,
                MemberDecision::Rejected(MemberFailure::Missing { key }),
            ),
        }
    }

    /// Return the committed target from resolved member candidates.
    fn member_target_resolution(&self, candidates: &[MemberCandidate]) -> MemberTargetResolution {
        if candidates.len() == 1 {
            let candidate = &candidates[0];

            return MemberTargetResolution::Symbol {
                symbol: candidate.symbol,
                instance: candidate.instance.clone(),
            };
        }

        let candidates = candidates
            .iter()
            .map(|candidate| CandidateResolution {
                symbol: candidate.symbol,
                instance: candidate.instance.clone(),
            })
            .collect();

        MemberTargetResolution::Union(candidates)
    }

    /// Return a member static from one reduced owner operand.
    pub(in crate::check) fn member_static_operand(
        &mut self,
        module: ModuleId,
        owner: TypeOperand,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<StaticOperand>> {
        let Some(term) = self.type_operand_term_id(owner)? else {
            return Ok(None);
        };

        match self.inference.term(term) {
            TypeTerm::Form { payload, .. } => self.member_static_operand(module, *payload, key),
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } if arguments.is_empty() => self.symbol_member_static(module, *symbol, key),
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } => {
                let symbol = *symbol;
                let arguments = arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[GenericArgument; 2]>>();

                self.instantiate_symbol_member_static(module, symbol, &arguments, key)
            }
            _ => Ok(None),
        }
    }

    /// Reduce one generic constraint operand.
    pub(in crate::check) fn reduce_generic_constraint(
        &mut self,
        origin: Origin,
        constraint: TypeOperand,
    ) -> CompilerResult<Option<TypeOperand>> {
        let Answer::Ready(constraint) = self.reduce_type_operand(origin, constraint)? else {
            return Ok(None);
        };

        Ok(Some(constraint))
    }

    /// Return a member static from one nominal declaration.
    fn symbol_member_static(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<StaticOperand>> {
        let Some(definition) = self.definitions.definition(symbol) else {
            return Ok(None);
        };
        let members = definition.static_members(&key);
        let [member] = members.as_slice() else {
            return Ok(None);
        };
        let substitution = SubstitutionSet::empty();
        if self.decide_symbol_availability(module, member.symbol, &substitution)?
            != Answer::Ready(true)
        {
            return Ok(None);
        }

        Ok(Some(member.value))
    }

    /// Return the type terms carried by resolved member candidates.
    fn member_candidate_types(
        &mut self,
        module: ModuleId,
        origin: Origin,
        key: dir::StaticKey,
        candidates: Vec<MemberCandidate>,
        arguments: &[GenericArgument],
        projection: Option<TypeOperand>,
    ) -> CompilerResult<Option<TypeOperand>> {
        let mut members = Vec::with_capacity(candidates.len());

        // apply member generic arguments to every candidate
        for candidate in candidates {
            let Some(member) =
                self.member_candidate_type(module, origin, key, candidate, arguments, projection)?
            else {
                return Ok(None);
            };

            members.push(member);
        }

        if members.len() == 1 {
            return Ok(members.pop());
        }

        Ok(Some(
            self.type_term_operand(TypeTerm::Union { elements: members }),
        ))
    }

    /// Return the type term carried by one resolved member candidate.
    fn member_candidate_type(
        &mut self,
        module: ModuleId,
        origin: Origin,
        key: dir::StaticKey,
        candidate: MemberCandidate,
        arguments: &[GenericArgument],
        projection: Option<TypeOperand>,
    ) -> CompilerResult<Option<TypeOperand>> {
        let Some(ty) = candidate.ty else {
            if let Some(projection) = projection {
                return Ok(Some(projection));
            }

            let term = MemberTerm {
                origin,
                receiver: MemberReceiver::Value(candidate.receiver),
                key,
                arguments: arguments.iter().cloned().collect(),
            };
            let term = self.inference.push_term(term);

            return Ok(Some(self.type_term_operand(TypeTerm::Member(term))));
        };

        let substitution = self.generic_substitution(candidate.symbol, arguments)?;
        let ty = if substitution.is_empty() {
            ty
        } else {
            self.substitute_type_operand(module, &substitution, ty)?
        };
        let Some(instance) = candidate.instance else {
            return Ok(Some(ty));
        };
        let substitution = self.generic_instance_substitution(&instance)?;
        if substitution.is_empty() {
            return Ok(Some(ty));
        }

        let ty = self.substitute_type_operand(module, &substitution, ty)?;

        Ok(Some(ty))
    }

    /// Instantiate a member static from one applied nominal declaration.
    fn instantiate_symbol_member_static(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        key: dir::StaticKey,
    ) -> CompilerResult<Option<StaticOperand>> {
        let Some(definition) = self.definitions.definition(symbol) else {
            return Ok(None);
        };
        let members = definition.static_members(&key);
        let [member] = members.as_slice() else {
            return Ok(None);
        };
        let substitution = self.generic_substitution(symbol, arguments)?;
        if self.decide_symbol_availability(module, member.symbol, &substitution)?
            != Answer::Ready(true)
        {
            return Ok(None);
        }
        if substitution.is_empty() {
            return Ok(Some(member.value));
        }

        self.substitute_static_operand(module, &substitution, member.value)
            .map(Some)
    }

    /// Return the type constraint for one generic type parameter.
    pub(in crate::check) fn type_generic_constraint(
        &self,
        parameter_id: GenericParameterId,
    ) -> CompilerResult<Option<TypeOperand>> {
        let generic = self.inference.generic_parameter_binding(parameter_id)?;
        if !generic.is_type() {
            return Ok(None);
        }

        let constraint = generic.type_constraint();

        Ok(constraint)
    }
}
