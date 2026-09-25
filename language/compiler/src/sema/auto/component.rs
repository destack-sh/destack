use smallvec::SmallVec;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{CheckState, Origin, Verdict};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Decide one auto interface over a normalized head under the coinductive guard.
    pub(in crate::sema) fn decide_recorded(
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
        // answer a closed type of this module from its recorded verdict, as given or normalized
        let unreduced = ty;
        if unreduced.module_id == self.module_id
            && let Some(holds) = self.module.representations_tail.auto(unreduced, interface)
        {
            return Ok(Verdict::decided(holds));
        }
        let ty = self.normalize(origin, ty)?;
        if ty.module_id == self.module_id
            && let Some(holds) = self.module.representations_tail.auto(ty, interface)
        {
            return Ok(Verdict::decided(holds));
        }

        // close recursive structural types coinductively
        if active.contains(&ty) {
            return Ok(Verdict::Holds);
        }
        active.push(ty);

        // decide by the bounds a generic type declares, else by the components
        let verdict = match self.decide_generic_auto_interface(origin, ty, interface)? {
            Some(decision) => Verdict::decided(decision),
            None => decide(self, ty, active)?,
        };
        active.pop();

        // record a refusal at any depth, a holding verdict only outside every enclosing decision
        let is_recorded = match verdict {
            Verdict::Fails => true,
            Verdict::Holds => active.is_empty(),
            Verdict::Ambiguous => false,
        };
        if is_recorded {
            for recorded in [ty, unreduced] {
                let flags = self.type_flags(recorded)?;
                let is_closed = !flags.has_variable() && !flags.has_error() && !flags.has_this();
                if recorded.module_id == self.module_id && is_closed {
                    self.module.representations_tail.set_auto(
                        recorded,
                        interface,
                        verdict == Verdict::Holds,
                    );
                }
            }
        }

        Ok(verdict)
    }

    /// Return the payload of one transparent payload intrinsic, none for every other application.
    pub(in crate::sema) fn transparent_payload(
        &mut self,
        module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // require a transparent payload intrinsic
        let item = self.language_item(instance.symbol)?;
        if !matches!(
            item,
            Some(
                dir::LanguageItem::UnsafeCell
                    | dir::LanguageItem::ManuallyDrop
                    | dir::LanguageItem::MaybeUninit
                    | dir::LanguageItem::Wrapping
                    | dir::LanguageItem::Pin
            )
        ) {
            return Ok(None);
        }

        // read the payload argument
        let arguments = self.type_ids(module, instance.arguments)?;
        let Some(payload) = arguments.first().copied() else {
            return Err(CompilerError::Internal {
                message: format!("{item:?} applied without its payload argument"),
            });
        };

        Ok(Some(payload))
    }

    /// Refuse one stuck head, an open one staying undecided.
    pub(in crate::sema) fn stuck_verdict(&self, ty: dir::GlobalTypeId) -> CompilerResult<Verdict> {
        Ok(match self.type_flags(ty)?.has_variable() {
            true => Verdict::Ambiguous,
            false => Verdict::Fails,
        })
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

    /// Decide one judgment over the component types, stopping at the first acceptance.
    pub(in crate::sema) fn decide_any(
        &mut self,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        mut decide: impl FnMut(&mut Self, dir::GlobalTypeId) -> CompilerResult<Verdict>,
    ) -> CompilerResult<Verdict> {
        let mut verdict = Verdict::Fails;
        for id in ids {
            verdict = verdict.or(decide(self, id)?);
            if verdict == Verdict::Holds {
                return Ok(Verdict::Holds);
            }
        }

        Ok(verdict)
    }

    /// Decide one judgment over the component types of one nominal instance, accepting any.
    pub(in crate::sema) fn decide_any_applied(
        &mut self,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        mut decide: impl FnMut(&mut Self, dir::GlobalTypeId) -> CompilerResult<Verdict>,
    ) -> CompilerResult<Verdict> {
        let substitution = self.instance_substitution(instance_module, instance)?;

        self.decide_any(ids, |state, id| {
            let applied = state.substitute_type(id, &substitution)?;

            decide(state, applied)
        })
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

    /// Collect the types one object declaration stores inline, with its base class.
    pub(in crate::sema) fn stored_object_types(
        &mut self,
        definition: &dir::Definition,
    ) -> CompilerResult<Option<SmallVec<[dir::GlobalTypeId; 8]>>> {
        match definition {
            dir::Definition::Struct(definition) => {
                Ok(Some(self.stored_field_types(&definition.members)?))
            }
            dir::Definition::Class(definition) => {
                let mut fields = self.stored_field_types(&definition.members)?;
                fields.extend(definition.extends.iter().map(|heritage| heritage.ty));

                Ok(Some(fields))
            }
            dir::Definition::TypeAlias(_)
            | dir::Definition::Interface(_)
            | dir::Definition::Enum(_)
            | dir::Definition::Newtype(_)
            | dir::Definition::Extension(_) => Ok(None),
        }
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
