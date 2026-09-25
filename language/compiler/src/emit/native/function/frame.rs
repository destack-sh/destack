use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use tspp_native as native;
use tspp_program::object::{FramePlace, FramePoint};

use crate::EmitError;

use super::{FunctionEmitter, Value};

/// One Cranelift stack map before register and stack allocation.
pub(crate) struct StackMap {
    /// Key of the stack slot anchoring this compiled frame.
    pub(crate) marker: cir::StackSlotKey,
    /// Object-local logical frame-state index.
    pub(crate) state: u32,
    /// Keyed canonical value byte lengths.
    pub(crate) values: Vec<(cir::StackSlotKey, u32)>,
}

/// One reconstructable native frame at an emitted operation.
pub(super) struct FrameMap {
    /// Object-local frame map identity.
    pub(super) id: u32,
    /// Runtime address anchoring stack-relative locations.
    pub(super) anchor: cir::Value,
    /// Cranelift locations retained through register allocation.
    pub(super) entries: Vec<cir::UserStackMapEntry>,
}

impl<'a> FunctionEmitter<'a> {
    /// Link one object-local frame map into the Program frame-map range.
    pub(super) fn frame_map_id(
        &mut self,
        frame_map: u32,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        self.index_u32(native::Index::Frame { frame: frame_map }, builder)
    }

    /// Materialize one Cranelift stack map for an exact logical frame state.
    pub(super) fn stack_map(
        &mut self,
        point: FramePoint,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<FrameMap, EmitError> {
        let frame = self
            .object
            .frame(point)
            .ok_or_else(|| self.invalid("native stack map has no logical frame state"))?
            .clone();
        let state = self
            .object
            .frame_index(point)
            .ok_or_else(|| self.invalid("native stack map has no object identity"))?;
        let id = self.frame_base + self.stack_maps.len() as u32;
        let marker = self.next_key();
        let marker_slot = builder.create_sized_stack_slot(cir::StackSlotData::new_with_key(
            cir::StackSlotKind::ExplicitSlot,
            1,
            0,
            marker,
        ));
        let anchor = builder
            .ins()
            .stack_addr(self.types.pointer(), marker_slot, 0);
        let mut entries = vec![cir::UserStackMapEntry {
            ty: cir::types::I8,
            slot: marker_slot,
            offset: 0,
        }];

        // materialize every canonical value and preserve its logical identity
        let mut values = Vec::with_capacity(frame.slots.len());
        for frame_slot in &frame.slots {
            let value_type = self.types.value(frame_slot.ty)?;
            let (slot, key) = match frame_slot.place {
                FramePlace::Environment => {
                    let environment = self
                        .environment
                        .ok_or_else(|| self.invalid("native frame environment is unavailable"))?;
                    self.retain(Value::Direct(environment), value_type, builder)?
                }
                FramePlace::Local(local) => {
                    let local = self.locals[&local];

                    (local.slot, local.key)
                }
                FramePlace::Value(value) => {
                    let value = self.value(value)?;
                    self.retain(value, value_type, builder)?
                }
            };
            entries.push(cir::UserStackMapEntry {
                ty: cir::types::I8,
                slot,
                offset: 0,
            });
            values.push((key, value_type.byte_len()));
        }

        self.stack_maps.push(StackMap {
            marker,
            state,
            values,
        });

        Ok(FrameMap {
            id,
            anchor,
            entries,
        })
    }

    /// Attach one stack map to its mapped instruction.
    pub(super) fn attach_stack_map(
        entries: Vec<cir::UserStackMapEntry>,
        instruction: cir::Inst,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) {
        builder
            .func
            .dfg
            .append_user_stack_map_entries(instruction, entries);
    }
}
