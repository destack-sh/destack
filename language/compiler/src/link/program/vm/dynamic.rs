use destack_mir as mir;
use destack_program::vm::{Instruction, Op};

use crate::LinkResult;

use super::lower::BlockLowerer;

/// One dynamic value operand in the current frame.
pub(super) struct DynamicOperand {
    /// The frame byte offset.
    pub(super) offset: u32,
    /// The implemented dynamic constraint.
    pub(super) constraint: mir::TypeId,
}

impl BlockLowerer<'_> {
    /// Lower one dynamic value binding.
    pub(super) fn lower_dynamic_bind(
        &self,
        destination: mir::Value,
        payload: mir::Value,
        concrete: mir::TypeId,
    ) -> LinkResult<Instruction> {
        let dynamic = self.dynamic_operand(destination)?;
        let constraint = dynamic.constraint;

        // require a local managed reference to the declared concrete type
        let payload_type = self.value_type_for_value(payload)?;
        let payload_type = self.function.tree.repr_type(payload_type);
        let mir::Type::Reference {
            kind: mir::ReferenceKind::Managed,
            space: mir::Space::Local,
            pointee,
            ..
        } = self.function.tree.get(payload_type)
        else {
            return Err(
                self.type_mismatch("local managed dynamic payload", format!("{payload_type:?}"))
            );
        };
        if *pointee != concrete {
            return Err(self.type_mismatch(
                format!("managed reference to {concrete:?}"),
                format!("managed reference to {pointee:?}"),
            ));
        }

        // resolve the durable witness table during program linking
        let table = self
            .function
            .program
            .program()
            .dynamic_table_id(concrete, constraint)
            .ok_or_else(|| self.invalid_instruction("dynamic table"))?;

        Ok(Instruction::new(
            Op::DynamicBind,
            dynamic.offset,
            self.cell_offset(payload)?,
            table.0,
            0,
        ))
    }

    /// Lower one dynamic payload projection.
    pub(super) fn lower_dynamic_payload(
        &self,
        destination: mir::Value,
        dynamic: mir::Value,
    ) -> LinkResult<Instruction> {
        Ok(Instruction::new(
            Op::DynamicPayload,
            self.cell_offset(destination)?,
            self.dynamic_operand(dynamic)?.offset,
            0,
            0,
        ))
    }

    /// Lower one dynamic concrete type projection.
    pub(super) fn lower_dynamic_type(
        &self,
        destination: mir::Value,
        dynamic: mir::Value,
    ) -> LinkResult<Instruction> {
        Ok(Instruction::new(
            Op::DynamicType,
            self.cell_offset(destination)?,
            self.dynamic_operand(dynamic)?.offset,
            0,
            0,
        ))
    }

    /// Lower one dynamic value drop.
    pub(super) fn lower_dynamic_drop(&self, dynamic: mir::Value) -> LinkResult<Instruction> {
        Ok(Instruction::new(
            Op::DropDynamic,
            self.dynamic_operand(dynamic)?.offset,
            0,
            0,
            0,
        ))
    }

    /// Resolve one dynamic value operand.
    fn dynamic_operand(&self, value: mir::Value) -> LinkResult<DynamicOperand> {
        let ty = self.value_type_for_value(value)?;
        let ty = self.function.tree.repr_type(ty);
        let mir::Type::Dynamic { constraint } = self.function.tree.get(ty) else {
            return Err(self.type_mismatch("dynamic value", format!("{ty:?}")));
        };

        Ok(DynamicOperand {
            offset: self.value_offset(value)?,
            constraint: *constraint,
        })
    }

    /// Resolve one dynamic receiver implementing the expected constraint.
    pub(super) fn resolve_dynamic(
        &self,
        value: mir::Value,
        constraint: mir::TypeId,
    ) -> LinkResult<DynamicOperand> {
        let dynamic = self.dynamic_operand(value)?;
        if dynamic.constraint != constraint {
            return Err(self.type_mismatch(
                format!("dynamic value for {constraint:?}"),
                format!("dynamic value for {:?}", dynamic.constraint),
            ));
        }

        Ok(dynamic)
    }
}
