use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{Answer, answer};

use super::query::LayoutQuery;

impl LayoutQuery<'_, '_> {
    /// Compute the DIR layout id for one nested value slot.
    ///
    /// Managed representations occupy pointer slots: stored values apply their default form here.
    pub(super) fn slot_layout(
        &mut self,
        owner: ModuleId,
        ty: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<Option<dir::LocalLayoutId>>> {
        // reduce the slot type to its stored form
        let origin = self.origin;
        let slot = answer!(self.check.reduce_type_head(origin, ty)?);
        let slot = self.layout_type(slot)?;
        let managed = answer!(self.decide_managed_representation(slot)?);
        if managed {
            let pointer_bytes = self.target_pointer_bytes()?;
            let layout = dir::Layout::pointer_slot(slot, pointer_bytes, true);
            let working = self.layout_segment(owner);
            let id = working.insert_layout(layout);

            return Ok(Answer::Ready(Some(id)));
        }

        let Some(id) = answer!(self.layout_id(slot, source)?) else {
            return Ok(Answer::Ready(None));
        };
        if slot.module_id == owner {
            return Ok(Answer::Ready(Some(id)));
        }

        let layout = self.layout(slot.module_id, id).clone();
        let id = self.layout_segment(owner).insert_layout(layout);

        Ok(Answer::Ready(Some(id)))
    }

    /// Decide whether one reduced type stores through a managed pointer by default.
    pub(super) fn decide_managed_representation(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let instance = match self.check.ty(ty)? {
            dir::Type::Instance(instance) => instance,
            _ => return Ok(Answer::Ready(false)),
        };

        let backing = match self.check.definition(instance.symbol) {
            Some(dir::Definition::Class(_)) => return Ok(Answer::Ready(true)),
            // object-shaped newtype backings are managed objects
            Some(dir::Definition::Newtype(definition)) => {
                let substitution = self.check.instance_substitution(ty.module_id, &instance)?;

                self.substituted_type(definition.value, &substitution)?
            }
            _ => return Ok(Answer::Ready(false)),
        };

        // reduce the backing before judging its shape
        let origin = self.origin;
        let backing = answer!(self.check.reduce_type_head(origin, backing)?);

        Ok(Answer::Ready(matches!(
            self.check.ty(backing)?,
            dir::Type::Shape(_)
        )))
    }
}
