use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, answer};

use super::query::LayoutQuery;

/// One summarized slot layout composed into an aggregate.
#[derive(Debug, Clone, Copy)]
pub(super) struct SlotLayout {
    /// The inserted layout id.
    pub(super) id: dir::LocalLayoutId,
    /// The size in bytes.
    pub(super) size: u32,
    /// The alignment in bytes.
    pub(super) alignment: u32,
    /// The largest niche of free values.
    pub(super) niche: Option<dir::Niche>,
}

impl LayoutQuery<'_, '_> {
    /// Compute one nested layout and summarize it for composition.
    ///
    /// Managed representations occupy pointer slots: the unqualified
    /// default form materializes here, at the representation boundary.
    pub(super) fn slot_layout(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<SlotLayout>>> {
        // close the slot type to check its default form
        let origin = self.origin;
        let slot = answer!(self.check.reduce_type_root(origin, ty)?);
        let slot = self.represented_type(slot)?;
        let managed = answer!(self.decide_managed_representation(slot)?);
        if managed {
            let pointer_bytes = self.target_pointer_bytes()?;
            let layout = dir::Layout::pointer_slot(slot, pointer_bytes, true);
            let working = self.layout_segment(slot.module_id);
            let id = working.insert_layout(layout);

            return Ok(Answer::Ready(Some(SlotLayout {
                id,
                size: pointer_bytes,
                alignment: pointer_bytes,
                niche: Some(dir::Niche {
                    offset: 0,
                    width: pointer_bytes,
                    start: 1,
                    end: dir::Niche::scalar_max(pointer_bytes),
                }),
            })));
        }

        let Some(id) = answer!(self.layout_id(slot)?) else {
            return Ok(Answer::Ready(None));
        };

        // summarize from the owning segment
        let origin = self.origin;
        let ty = answer!(self.check.reduce_type_root(origin, slot)?);
        let working = self
            .check
            .layouts
            .get(&ty.module_id)
            .unwrap_or_else(|| unreachable!("computed layout must own a layout segment"));
        let layout = working.get_layout(id);
        let size = layout.size;
        let alignment = layout.alignment;

        Ok(Answer::Ready(Some(SlotLayout {
            id,
            size,
            alignment,
            niche: layout.niche,
        })))
    }

    /// Decide whether one reduced type stores through a managed pointer by default.
    pub(super) fn decide_managed_representation(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let instance = match self.check.ty(ty)? {
            dir::Type::Instance(instance) => instance.clone(),
            _ => return Ok(Answer::Ready(false)),
        };

        let backing = match self.check.definition(instance.symbol) {
            Some(dir::Definition::Class(_)) => return Ok(Answer::Ready(true)),
            // object-shaped newtype backings are managed objects
            Some(dir::Definition::Newtype(definition)) => definition.value,
            _ => return Ok(Answer::Ready(false)),
        };

        // close the backing before judging its shape
        let origin = self.origin;
        let backing = answer!(self.check.reduce_type_root(origin, backing)?);

        Ok(Answer::Ready(matches!(
            self.check.ty(backing)?,
            dir::Type::Shape(_)
        )))
    }
}
