use destack_mir as mir;

use crate::program::{
    FrameAccess, FunctionAddr, Instruction, Load, LoadFrame, LoadFrameBytes, LocalAddr, LocalGet,
    LocalSet, Opcode, PointeeAccess, StaticAddr, Store, StoreFrame, StoreFrameBytes,
};
use crate::{Error, Result};

use super::access::pointee_access;
use super::lower::BlockLowerer;
use super::opcode::{select_load_opcode, select_store_opcode};
use super::pool::Pool;
use super::value::{
    heap_pointee_type_for_value, heap_pointee_type_for_value_layout, pointer_class_for_value,
    raw_pointee_type_for_value, raw_pointee_type_for_value_layout, reference_meta_for_value,
};

/// Return one frame access from a pointer access.
fn frame_access(access: PointeeAccess) -> FrameAccess {
    FrameAccess {
        value_type: access.value_type,
        byte_offset: access.byte_offset,
        byte_stride: 0,
        length: 0,
        byte_len: access.byte_len,
        word_layout: access.word_layout,
    }
}

impl<'a> BlockLowerer<'a> {
    /// Lower one local get.
    pub(super) fn lower_local_get(
        &self,
        destination: mir::ValueReference,
        local: mir::LocalReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "local get destination".to_string(),
            })?;
        let local = local.local().ok_or_else(|| Error::MissingRepresentation {
            context: "local get source".to_string(),
        })?;
        let local = self.local_index(local)?;

        Ok(Instruction::new(
            Opcode::LoadLocal,
            LocalGet {
                dest: destination,
                local,
            },
        ))
    }

    /// Lower one local address.
    pub(super) fn lower_local_addr(
        &self,
        destination: mir::ValueReference,
        local: mir::LocalReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "local address destination".to_string(),
            })?;
        let local = local.local().ok_or_else(|| Error::MissingRepresentation {
            context: "local address local".to_string(),
        })?;
        let local = self.local_index(local)?;

        Ok(Instruction::new(
            Opcode::AddressLocal,
            LocalAddr {
                dest: destination,
                local,
                reference: reference_meta_for_value(self.value_layout_map(), destination),
            },
        ))
    }

    /// Lower one local set.
    pub(super) fn lower_local_set(
        &self,
        local: mir::LocalReference,
        value: mir::ValueReference,
    ) -> Result<Instruction> {
        let local = local.local().ok_or_else(|| Error::MissingRepresentation {
            context: "local set destination".to_string(),
        })?;
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "local set value".to_string(),
        })?;
        let local = self.local_index(local)?;

        Ok(Instruction::new(
            Opcode::StoreLocal,
            LocalSet { local, value },
        ))
    }

    /// Lower one static address.
    pub(super) fn lower_static_addr(
        &self,
        destination: mir::ValueReference,
        global: mir::GlobalReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "static address destination".to_string(),
            })?;
        let global = global
            .global()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "static address global".to_string(),
            })?;

        Ok(Instruction::new(
            Opcode::AddressStatic,
            StaticAddr {
                dest: destination,
                global: global.id,
                reference: reference_meta_for_value(self.value_layout_map(), destination),
            },
        ))
    }

    /// Lower one function address.
    pub(super) fn lower_function_addr(
        &self,
        destination: mir::ValueReference,
        function: mir::FunctionReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "function address destination".to_string(),
            })?;
        let function = function
            .function()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "function address callee".to_string(),
            })?;

        Ok(Instruction::new(
            Opcode::AddressFunction,
            FunctionAddr {
                dest: destination,
                function: function.id,
            },
        ))
    }

    /// Lower one load.
    pub(super) fn lower_load(
        &self,
        pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        pointer: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "load destination".to_string(),
            })?;
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "load pointer".to_string(),
            })?;
        let access = self.pointee_access_for_value(pointer)?;
        let opcode = select_load_opcode(access)?;
        let instruction = match opcode {
            Opcode::LoadFrame => Instruction::new(
                opcode,
                LoadFrame {
                    dest: destination,
                    base: pointer,
                    access: pool.frame_access(frame_access(access)),
                },
            ),
            Opcode::LoadFrameBytes => Instruction::new(
                opcode,
                LoadFrameBytes {
                    destination,
                    address: pointer,
                    access: pool.pointee_access(access),
                },
            ),
            _ => Instruction::new(
                opcode,
                Load {
                    dest: destination,
                    pointer,
                    access: pool.pointee_access(access),
                },
            ),
        };

        Ok(instruction)
    }

    /// Lower one store.
    pub(super) fn lower_store(
        &self,
        pool: &mut Pool<'_>,
        pointer: mir::ValueReference,
        value: mir::ValueReference,
    ) -> Result<Instruction> {
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "store pointer".to_string(),
            })?;
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "store value".to_string(),
        })?;
        let access = self.pointee_access_for_value(pointer)?;
        let opcode = select_store_opcode(access)?;
        let instruction = match opcode {
            Opcode::StoreFrame => Instruction::new(
                opcode,
                StoreFrame {
                    base: pointer,
                    value,
                    reference: reference_meta_for_value(self.value_layout_map(), pointer),
                    access: pool.frame_access(frame_access(access)),
                },
            ),
            Opcode::StoreFrameBytes => Instruction::new(
                opcode,
                StoreFrameBytes {
                    address: pointer,
                    source: value,
                    access: pool.pointee_access(access),
                },
            ),
            _ => Instruction::new(
                opcode,
                Store {
                    pointer,
                    value,
                    access: pool.pointee_access(access),
                },
            ),
        };

        Ok(instruction)
    }

    /// Return the lowered access for a pointer value.
    fn pointee_access_for_value(&self, pointer: mir::Value) -> Result<PointeeAccess> {
        let pointee_type = heap_pointee_type_for_value_layout(self.value_layout_map(), pointer)
            .or_else(|| raw_pointee_type_for_value_layout(self.value_layout_map(), pointer))
            .or_else(|| heap_pointee_type_for_value(self.tree, self.value_type(), pointer))
            .or_else(|| raw_pointee_type_for_value(self.tree, self.value_type(), pointer));
        let pointer_class = pointer_class_for_value(self.value_layout_map(), pointer);

        pointee_type
            .and_then(|pointee_type| {
                pointee_access(self.tree, self.layouts(), pointee_type, pointer_class)
            })
            .ok_or(Error::InvalidInstruction)
    }
}
