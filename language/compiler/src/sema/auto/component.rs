use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{CheckState, Origin, Verdict};

impl CheckState<'_> {
    /// Decide one auto interface over a normalized head under the coinductive guard.
    pub(in crate::sema) fn decide_guarded(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
        decide: impl FnOnce(
            &mut Self,
            dir::GlobalTypeId,
            &mut SmallVec<[dir::GlobalTypeId; 8]>,
        ) -> CompilerResult<Verdict>,
    ) -> CompilerResult<Verdict> {
        // unfold aliases, then use bounds declared by generic types
        let ty = self.normalize(origin, ty)?;
        if let Some(decision) = self.decide_generic_auto_interface(origin, ty, interface)? {
            return Ok(Verdict::decided(decision));
        }

        // close recursive structural types coinductively
        if active.contains(&ty) {
            return Ok(Verdict::Holds);
        }
        active.push(ty);

        // restore the active guard after this decision
        let result = decide(self, ty, active);
        active.pop();

        result
    }

    /// Decide one judgment over every component type, stopping at the first refusal.
    pub(in crate::sema) fn decide_all(
        &mut self,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        mut decide: impl FnMut(&mut Self, dir::GlobalTypeId) -> CompilerResult<Verdict>,
    ) -> CompilerResult<Verdict> {
        let mut verdict = Verdict::Holds;
        for id in ids {
            verdict = verdict.and(decide(self, id)?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Decide one judgment over every component type of one nominal instance.
    pub(in crate::sema) fn decide_all_applied(
        &mut self,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        mut decide: impl FnMut(&mut Self, dir::GlobalTypeId) -> CompilerResult<Verdict>,
    ) -> CompilerResult<Verdict> {
        let substitution = self.instance_substitution(instance_module, instance)?;

        self.decide_all(ids, |state, id| {
            let applied = state.substitute_type(id, &substitution)?;

            decide(state, applied)
        })
    }

    /// Collect the stored field types one declaration holds, in declaration order.
    pub(in crate::sema) fn stored_field_types(
        &mut self,
        members: &[dir::DefinitionMember],
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 8]>> {
        let mut fields = SmallVec::new();
        for member in members {
            if let dir::DefinitionMember::Field(_) = member
                && let Some(ty) = self.definition_member_type(member)?
            {
                fields.push(ty);
            }
        }

        Ok(fields)
    }
}
