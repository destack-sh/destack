use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CheckState, Decision, GenericArgument, GenericInstance, GenericParameterId, MemberProtocol,
    Origin, ShapeMember, SubstitutionSet, TupleElement, TypeMemberDefinition, TypeOperand,
    TypeRelation, TypeTerm,
};
use crate::{CompilerError, CompilerResult};

/// A symbol-backed member candidate found by lookup.
pub(in crate::check) struct MemberCandidate {
    /// The resolved member symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The resolved member type.
    pub(in crate::check) ty: TypeOperand,
    /// The resolved generic instance, when lookup instantiated an owner.
    pub(in crate::check) instance: Option<GenericInstance>,
}

/// Result of looking up one member on a receiver type.
pub(in crate::check) enum MemberLookup {
    /// Member lookup is waiting for solver input.
    Pending,
    /// No member exists.
    Missing,
    /// One structural field exists.
    Field(TypeOperand),
    /// One or more symbol-backed members exist.
    Found(Vec<MemberCandidate>),
}

impl MemberLookup {
    /// Return a member lookup from collected candidates.
    pub(in crate::check) fn from_candidates(candidates: Vec<MemberCandidate>) -> Self {
        if candidates.is_empty() {
            Self::Missing
        } else {
            Self::Found(candidates)
        }
    }
}

impl CheckState<'_> {
    /// Resolve members from one type term.
    pub(in crate::check) fn resolve_type_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: TypeOperand,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let Some(receiver) = self.reduce_type_operand(origin, receiver)? else {
            return Ok(MemberLookup::Pending);
        };

        self.resolve_reduced_type_member(origin, module, receiver, key)
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
        let Some(receiver) = self.reduce_type_operand(origin, receiver)? else {
            return Ok(MemberLookup::Pending);
        };

        match self.decide_member_protocol(module, receiver, protocol)? {
            Decision::Yes => self.resolve_reduced_type_member(origin, module, receiver, key),
            Decision::Undecidable => Ok(MemberLookup::Pending),
            Decision::No => Ok(MemberLookup::Missing),
        }
    }

    /// Resolve members from one reduced type term.
    fn resolve_reduced_type_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: TypeOperand,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let Some(term) = self.type_operand_term(receiver)? else {
            return Ok(MemberLookup::Pending);
        };

        match term {
            TypeTerm::Form { payload, .. } => {
                let Some(payload) = self.reduce_type_operand(origin, payload)? else {
                    return Ok(MemberLookup::Pending);
                };

                self.resolve_reduced_type_member(origin, module, payload, key)
            }
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } if arguments.is_empty() => {
                match self.inference.symbol_generic_parameter(symbol) {
                    // resolve members through a type generic parameter
                    Some(parameter)
                        if self
                            .inference
                            .require_generic_parameter(parameter)
                            .is_type() =>
                    {
                        self.lookup_parameter_member(module, origin, parameter, key)
                    }
                    // resolve members through an ordinary symbol reference
                    _ => self.lookup_symbol_member(origin, module, symbol, &arguments, *key),
                }
            }
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } => self.lookup_symbol_member(origin, module, symbol, &arguments, *key),
            TypeTerm::Parameter(parameter) => {
                self.lookup_parameter_member(module, origin, parameter, key)
            }
            TypeTerm::Shape(shape) => {
                self.lookup_shape_member(&self.inference.term(shape).members, key)
            }
            TypeTerm::Tuple { elements, .. } => self.lookup_tuple_member(&elements, key),
            TypeTerm::Union { elements } => {
                self.union_member_lookup(origin, module, &elements, key)
            }
            TypeTerm::Operation(operation) => {
                self.operation_member_lookup(origin, module, operation, key)
            }
            _ => Ok(MemberLookup::Missing),
        }
    }

    /// Resolve members through a generic parameter constraint.
    fn lookup_parameter_member(
        &mut self,
        module: ModuleId,
        origin: Origin,
        parameter: GenericParameterId,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let Some(constraint) = self.type_generic_constraint(parameter)? else {
            return Ok(MemberLookup::Missing);
        };
        let Some(constraint) = self.reduced_generic_type_constraint(origin, constraint)? else {
            return Ok(MemberLookup::Pending);
        };

        self.resolve_reduced_type_member(origin, module, constraint, key)
    }

    /// Look up a structural member from one shape field.
    fn lookup_shape_member(
        &self,
        members: &[ShapeMember],
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        for member in members {
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

    /// Look up a structural member from one tuple element.
    fn lookup_tuple_member(
        &self,
        elements: &[TupleElement],
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let dir::StaticKey::Index(index) = key else {
            return Ok(MemberLookup::Missing);
        };
        let Some(element) = elements.get(*index) else {
            return Ok(MemberLookup::Missing);
        };
        Ok(MemberLookup::Field(element.ty))
    }

    /// Look up an applied symbol member.
    fn lookup_symbol_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let Some(definition) = self.definition(module, symbol)? else {
            return Ok(MemberLookup::Missing);
        };
        let members = definition.type_members(&key);
        let lookup = self.inherent_member_lookup(module, symbol, arguments, members)?;

        // prefer inherent members before searching extensions
        match lookup {
            MemberLookup::Found(_) | MemberLookup::Field(_) | MemberLookup::Pending => {
                return Ok(lookup);
            }
            MemberLookup::Missing => {}
        }

        self.extension_member_lookup(origin, module, symbol, arguments, key)
    }

    /// Return inherent member lookup for one applied declaration.
    fn inherent_member_lookup(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        members: Vec<TypeMemberDefinition>,
    ) -> CompilerResult<MemberLookup> {
        let substitution = self.generic_substitution(symbol, arguments)?;
        let instance = if substitution.is_empty() {
            None
        } else {
            let Some(template) = self.inference.symbol_generic_template(symbol) else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "generic arguments supplied for non-generic symbol {symbol:?}"
                    ),
                });
            };

            Some(GenericInstance::new(template, arguments.to_vec().into()))
        };
        let mut candidates = Vec::new();

        // resolve candidate members in declaration order
        for member in members {
            match self.collect_member_candidate(
                module,
                member,
                &substitution,
                instance.clone(),
                &mut candidates,
            )? {
                Decision::Yes | Decision::No => {}
                Decision::Undecidable => return Ok(MemberLookup::Pending),
            }
        }

        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Collect one type member definition as a lookup candidate.
    pub(in crate::check) fn collect_member_candidate(
        &mut self,
        module: ModuleId,
        member: TypeMemberDefinition,
        substitution: &SubstitutionSet,
        instance: Option<GenericInstance>,
        candidates: &mut Vec<MemberCandidate>,
    ) -> CompilerResult<Decision> {
        let Some(symbol) = member.symbol else {
            return Ok(Decision::No);
        };

        // skip statically unavailable declarations
        if self.reduce_symbol_availability(module, symbol, substitution)? != Decision::Yes {
            return Ok(Decision::No);
        }

        // apply receiver or extension generic substitutions
        let ty = if substitution.is_empty() {
            member.ty
        } else {
            let Some(term) = self.type_operand_term(member.ty)? else {
                return Ok(Decision::Undecidable);
            };
            let Some(term) = term.substitute(module, substitution, self)? else {
                return Ok(Decision::Undecidable);
            };

            self.inference.push_term(term).into()
        };

        candidates.push(MemberCandidate {
            symbol,
            ty,
            instance,
        });

        Ok(Decision::Yes)
    }

    /// Decide whether one type satisfies one member protocol.
    fn decide_member_protocol(
        &mut self,
        module: ModuleId,
        receiver: TypeOperand,
        protocol: &MemberProtocol,
    ) -> CompilerResult<Decision> {
        let symbol = self.language_symbol(module, protocol.item);
        let target = TypeTerm::Reference {
            origin: Origin::Symbol(symbol),
            symbol,
            arguments: protocol.arguments.to_vec(),
        };

        let target = self.inference.push_term(target);

        self.decide_type_relation(TypeRelation::Satisfies, receiver, target)
    }
}
