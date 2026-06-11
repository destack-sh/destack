use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    Answer, CheckState, GenericArgument, GenericInstance, MemberKey, MemberLookup, MemberSpace,
    Origin, SubstitutionSet, TypeMemberDefinition, TypeOperand, TypeTerm,
};
use crate::{CompilerError, CompilerResult};

/// One symbol-backed member candidate.
pub(in crate::check) struct MemberCandidate {
    /// The receiver type that selected this candidate.
    pub(in crate::check) receiver: TypeOperand,
    /// The resolved member symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The readable member type.
    pub(in crate::check) ty: Option<TypeOperand>,
    /// The resolved generic instance selected with this member.
    pub(in crate::check) instance: Option<GenericInstance>,
}

impl CheckState<'_> {
    /// Resolve an applied symbol member.
    pub(in crate::check) fn resolve_symbol_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: TypeOperand,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        space: MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let key = MemberKey::new(space, key);
        let members = self.definitions.members(symbol, key).to_vec();
        let lookup =
            self.resolve_inherent_members(origin, module, receiver, symbol, arguments, members)?;

        // prefer direct members before searching extensions
        match lookup {
            MemberLookup::Found(_) | MemberLookup::Field(_) | MemberLookup::Pending(_) => {
                return Ok(lookup);
            }
            MemberLookup::Missing => {}
        }

        self.resolve_extension_member(origin, module, symbol, arguments, receiver, key)
    }

    /// Resolve inherent members for one applied declaration.
    fn resolve_inherent_members(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: TypeOperand,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        members: Vec<TypeMemberDefinition>,
    ) -> CompilerResult<MemberLookup> {
        let mut substitution = self.generic_substitution(symbol, arguments)?;
        substitution.receiver(receiver);
        let instance = self.generic_instance_from_substitution(symbol, &substitution)?;
        let mut candidates = Vec::new();

        // resolve candidate members in declaration order
        for member in members {
            match self.resolve_member_candidate(
                origin,
                module,
                receiver,
                member,
                &substitution,
                instance.clone(),
            )? {
                Answer::Ready(Some(candidate)) => candidates.push(candidate),
                Answer::Ready(None) => {}
                Answer::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
            }
        }

        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Resolve one type member definition as a member candidate.
    pub(in crate::check) fn resolve_member_candidate(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: TypeOperand,
        member: TypeMemberDefinition,
        substitution: &SubstitutionSet,
        instance: Option<GenericInstance>,
    ) -> CompilerResult<Answer<Option<MemberCandidate>>> {
        let Some(symbol) = member.symbol() else {
            return Ok(Answer::Ready(None));
        };

        // respect static guards before admitting the candidate
        match self.decide_symbol_availability(module, symbol, substitution)? {
            Answer::Ready(true) => {}
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            Answer::Ready(false) => return Ok(Answer::Ready(None)),
        }

        let value = if member.is_abstract_associated_type() {
            None
        } else {
            let Some(ty) = member.value() else {
                return Ok(Answer::Ready(None));
            };

            let value = match self.member_read_type(origin, member.role(), ty)? {
                Answer::Ready(Some(ty)) => ty,
                Answer::Ready(None) => return Ok(Answer::Ready(None)),
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };

            // apply the receiver and selected generic instance
            if substitution.is_empty() {
                Some(value)
            } else {
                Some(self.substitute_type_operand(module, substitution, value)?)
            }
        };

        Ok(Answer::Ready(Some(MemberCandidate {
            receiver,
            symbol,
            ty: value,
            instance,
        })))
    }

    /// Return the value type produced by reading one member.
    fn member_read_type(
        &mut self,
        origin: Origin,
        role: Option<dir::FunctionRole>,
        ty: TypeOperand,
    ) -> CompilerResult<Answer<Option<TypeOperand>>> {
        match role {
            // getters read as their return value
            Some(dir::FunctionRole::Getter) => {
                self.getter_return_type(origin, ty).map(|ty| match ty {
                    Answer::Ready(ty) => Answer::Ready(Some(ty)),
                    Answer::Pending(blockers) => Answer::Pending(blockers),
                })
            }
            // setters are write-only in member reads
            Some(dir::FunctionRole::Setter) => Ok(Answer::Ready(None)),
            // ordinary methods read as callable values
            _ => Ok(Answer::Ready(Some(ty))),
        }
    }

    /// Return the return type produced by reading one getter.
    fn getter_return_type(
        &mut self,
        origin: Origin,
        ty: TypeOperand,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let Some(term) = self.type_operand_term_id(ty)? else {
            return Ok(Answer::pending(ty.dependencies(self)));
        };

        match self.inference.term(term) {
            TypeTerm::Function(function) => {
                let function = self.inference.term(*function);
                let Some(return_type) = function.return_type else {
                    let ty = self.dump_in_module(origin.module(), &ty);

                    return Err(CompilerError::Internal {
                        message: format!("getter member type {ty} has no return type"),
                    });
                };

                Ok(Answer::Ready(return_type))
            }
            _ => {
                let ty = self.dump_in_module(origin.module(), &ty);

                Err(CompilerError::Internal {
                    message: format!("getter member type {ty} is not a function"),
                })
            }
        }
    }
}
