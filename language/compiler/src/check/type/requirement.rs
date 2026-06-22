use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, MemberLookup, Origin, Relation};

/// One member required by a constraint surface.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) struct RequiredMember {
    /// The member space.
    pub(in crate::check) space: dir::MemberSpace,
    /// The member key.
    pub(in crate::check) key: dir::StaticKey,
    /// The member type, when the member carries a value.
    pub(in crate::check) ty: Option<dir::GlobalTypeId>,
    /// Whether the member is a method.
    pub(in crate::check) is_method: bool,
}

impl CheckState<'_> {
    /// Decide whether one source exposes every member of one interface application.
    pub(in crate::check) fn decide_member_satisfies(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target_instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        let module = origin.module();
        let members = match self.interface_members(origin, target_instance, source)? {
            Answer::Ready(members) => members,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        // require each member from the source
        let mut decision = Answer::Ready(true);
        for member in members {
            let lookup = self.lookup_member(origin, module, source, member.space, member.key)?;

            let found = match lookup {
                MemberLookup::Field(ty) => Some(ty),
                MemberLookup::Found(candidates) => candidates.first().map(|candidate| candidate.ty),
                MemberLookup::Missing => None,
                MemberLookup::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };
            let Some(found) = found else {
                return Ok(Answer::Ready(false));
            };

            // associated types without values only need presence
            let Some(member_type) = member.ty else {
                continue;
            };

            let assignment = if member.is_method {
                self.decide_method_assignable(origin, found, member_type)?
            } else {
                self.decide_relation(origin, Relation::Assignable, found, member_type)?
            };
            decision = decision.and(assignment);
            if decision == Answer::Ready(false) {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Return the members required by one interface application.
    pub(in crate::check) fn interface_members(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<SmallVec<[RequiredMember; 8]>>> {
        let closure = match self.heritage_closure(origin, instance)? {
            Answer::Ready(closure) => closure,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let mut members = SmallVec::<[RequiredMember; 8]>::new();

        // collect inherited members before direct members
        for application in closure.applications {
            self.collect_interface_members(origin, &application.instance, receiver, &mut members)?;
        }
        self.collect_interface_members(origin, instance, receiver, &mut members)?;

        Ok(Answer::Ready(members))
    }

    /// Collect direct members required by one applied interface.
    fn collect_interface_members(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
        receiver: dir::GlobalTypeId,
        required: &mut SmallVec<[RequiredMember; 8]>,
    ) -> CompilerResult<()> {
        let Some(dir::Definition::Interface(definition)) = self.definition(instance.symbol) else {
            return Ok(());
        };
        let members = definition.members.clone();
        let substitution = self
            .parameter_substitution(instance)?
            .with_receiver(receiver);
        let module = origin.module();
        let source = self.origin_source_node(origin)?;

        // collect direct interface members in the applied view
        for member in members {
            let Some(key) = member.key() else {
                continue;
            };
            let ty = match member.ty() {
                Some(ty) if !substitution.is_empty() => {
                    Some(self.fold_type(module, source, ty, substitution.rewrite())?)
                }
                ty => ty,
            };
            required.push(RequiredMember {
                space: member.space(),
                key,
                ty,
                is_method: matches!(member, dir::DefinitionMember::Method(_)),
            });
        }

        Ok(())
    }
}
