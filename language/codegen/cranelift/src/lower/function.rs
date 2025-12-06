//! Function lowering from MIR to Cranelift IR.
//!
//! This module handles the translation of a single MIR function into Cranelift IR.
//! The lowering process works in several phases:
//!
//! 1. **Locals**: Create stack slots for MIR local variables
//! 2. **Blocks**: Create Cranelift blocks and set up block parameters
//! 3. **Instructions**: Lower each instruction, building up the value map
//! 4. **Terminators**: Lower block terminators (jumps, branches, returns)
//!
//! ## Value Mapping
//!
//! MIR uses SSA values (`mir::Value`) that are defined once and used multiple times.
//! During lowering, we maintain a `value_map` that tracks the correspondence between
//! MIR values and Cranelift values (`cir::Value`). When an instruction produces a
//! result, we record it in the map. When an instruction uses a value, we look it up.
//!
//! ## Block Parameters
//!
//! MIR represents phi nodes explicitly as block parameters. When control flow merges,
//! values are passed as arguments to the target block. Cranelift uses the same model,
//! so the translation is direct: MIR block parameters become Cranelift block parameters.

use std::collections::HashMap;
use std::sync::Arc;

use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::isa::TargetIsa;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::FuncId;
use destack_mir as mir;
use destack_source::StringPool;

use super::r#type::lower_type;
use crate::{CraneliftError, trap};

/// Context for lowering a single MIR function to Cranelift IR.
#[allow(dead_code)]
pub(crate) struct FunctionLowerer<'a> {
    /// The MIR node tree.
    tree: &'a mir::NodeTree,
    /// String pool for resolving names.
    strings: &'a StringPool,
    /// The MIR function being lowered.
    function: &'a mir::Function,
    /// The target ISA.
    isa: &'a Arc<dyn TargetIsa>,
    /// Function id mapping for calls.
    cl_function_ids: &'a HashMap<mir::LocalNodeId<mir::Function>, FuncId>,
}

impl<'a> FunctionLowerer<'a> {
    /// Create a new function lowerer.
    pub(crate) fn new(
        tree: &'a mir::NodeTree,
        strings: &'a StringPool,
        function: &'a mir::Function,
        isa: &'a Arc<dyn TargetIsa>,
        cl_function_ids: &'a HashMap<mir::LocalNodeId<mir::Function>, FuncId>,
    ) -> Self {
        Self {
            tree,
            strings,
            function,
            isa,
            cl_function_ids,
        }
    }

    /// Lower the function to Cranelift IR.
    ///
    /// Populates the provided Cranelift function with blocks, instructions,
    /// and control flow based on the MIR function.
    pub(crate) fn lower(self, target: &mut cir::Function) -> Result<(), CraneliftError> {
        let mut builder_context = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(target, &mut builder_context);
        let mut value_map: HashMap<mir::Value, cir::Value> = HashMap::new();
        let mut block_map: HashMap<mir::LocalNodeId<mir::Block>, cir::Block> = HashMap::new();
        let mut local_map: HashMap<mir::LocalNodeId<mir::Local>, cir::StackSlot> = HashMap::new();

        // phase 1: create stack slots for locals
        self.create_locals(&mut builder, &mut local_map)?;

        // phase 2: create all blocks and set up parameters
        self.create_blocks(&mut builder, &mut value_map, &mut block_map)?;

        // phase 3+4: lower each block's instructions and terminator
        for &block_id in &self.function.blocks {
            self.lower_block(
                &mut builder,
                block_id,
                &mut value_map,
                &block_map,
                &local_map,
            )?;
        }

        builder.finalize();
        Ok(())
    }

    /// Create Cranelift stack slots for MIR locals.
    fn create_locals(
        &self,
        builder: &mut FunctionBuilder<'_>,
        local_map: &mut HashMap<mir::LocalNodeId<mir::Local>, cir::StackSlot>,
    ) -> Result<(), CraneliftError> {
        for &local_id in &self.function.locals {
            let local = self.tree.get(local_id);
            let ty = lower_type(self.tree, local.ty)?;
            let size = ty.bytes();

            let slot = builder.create_sized_stack_slot(cir::StackSlotData::new(
                cir::StackSlotKind::ExplicitSlot,
                size,
                0,
            ));

            local_map.insert(local_id, slot);
        }

        Ok(())
    }

    /// Create Cranelift blocks for all MIR blocks.
    ///
    /// Also sets up block parameters: function parameters go on the entry block,
    /// and MIR block parameters (for phi nodes) go on their respective blocks.
    fn create_blocks(
        &self,
        builder: &mut FunctionBuilder<'_>,
        value_map: &mut HashMap<mir::Value, cir::Value>,
        block_map: &mut HashMap<mir::LocalNodeId<mir::Block>, cir::Block>,
    ) -> Result<(), CraneliftError> {
        // first, create all blocks
        for &block_id in &self.function.blocks {
            let block = builder.create_block();
            block_map.insert(block_id, block);
        }

        // set entry block and add function parameters as entry block parameters
        let entry_block = block_map[&self.function.entry];

        // function parameters become entry block parameters in Cranelift
        for param in &self.function.parameters {
            let ty = lower_type(self.tree, param.ty)?;
            let value = builder.append_block_param(entry_block, ty);
            value_map.insert(param.value, value);
        }

        // now add MIR block parameters for non-entry blocks
        for &block_id in &self.function.blocks {
            let mir_block = self.tree.get(block_id);
            let target_block = block_map[&block_id];

            // add block parameters (these are for phi nodes / join points)
            for param in &mir_block.parameters {
                let ty = lower_type(self.tree, param.ty)?;
                let value = builder.append_block_param(target_block, ty);
                value_map.insert(param.value, value);
            }
        }

        // switch to entry block
        builder.switch_to_block(entry_block);

        // seal entry block (no predecessors)
        builder.seal_block(entry_block);

        Ok(())
    }

    /// Lower a single block's instructions and terminator.
    fn lower_block(
        &self,
        builder: &mut FunctionBuilder<'_>,
        block_id: mir::LocalNodeId<mir::Block>,
        value_map: &mut HashMap<mir::Value, cir::Value>,
        block_map: &HashMap<mir::LocalNodeId<mir::Block>, cir::Block>,
        local_map: &HashMap<mir::LocalNodeId<mir::Local>, cir::StackSlot>,
    ) -> Result<(), CraneliftError> {
        let mir_block = self.tree.get(block_id);
        let target_block = block_map[&block_id];

        // switch to this block (may already be current for entry)
        if builder.current_block() != Some(target_block) {
            builder.switch_to_block(target_block);
        }

        // lower instructions
        for &instruction_id in &mir_block.instructions {
            let instruction = self.tree.get(instruction_id);
            self.lower_instruction(instruction, builder, value_map, local_map)?;
        }

        // lower terminator
        self.lower_terminator(&mir_block.terminator, builder, value_map, block_map)?;

        // seal the block (all predecessors are now known)
        builder.seal_block(target_block);

        Ok(())
    }

    /// Lower a MIR instruction to Cranelift IR.
    ///
    /// Each instruction that produces a value records its result in `value_map`.
    /// Instructions that consume values look them up in `value_map`.
    fn lower_instruction(
        &self,
        instruction: &mir::Instruction,
        builder: &mut FunctionBuilder<'_>,
        value_map: &mut HashMap<mir::Value, cir::Value>,
        local_map: &HashMap<mir::LocalNodeId<mir::Local>, cir::StackSlot>,
    ) -> Result<(), CraneliftError> {
        match instruction {
            mir::Instruction::Constant { destination, value } => {
                let result = self.lower_constant(value, builder)?;
                value_map.insert(*destination, result);
            }

            mir::Instruction::Binary {
                destination,
                operator,
                left,
                right,
            } => {
                let left_value = value_map[left];
                let right_value = value_map[right];
                let result = self.lower_binary_op(*operator, left_value, right_value, builder)?;
                value_map.insert(*destination, result);
            }

            mir::Instruction::Unary {
                destination,
                operator,
                argument,
            } => {
                let arg_value = value_map[argument];
                let result = self.lower_unary_op(*operator, arg_value, builder)?;
                value_map.insert(*destination, result);
            }

            mir::Instruction::Cast {
                destination,
                kind,
                argument,
                to_type,
            } => {
                let arg_value = value_map[argument];
                let target_type = lower_type(self.tree, *to_type)?;
                let result = self.lower_cast(*kind, arg_value, target_type, builder)?;
                value_map.insert(*destination, result);
            }

            mir::Instruction::LocalGet { destination, local } => {
                let slot = local_map[local];
                let local_data = self.tree.get(*local);
                let ty = lower_type(self.tree, local_data.ty)?;
                let result = builder.ins().stack_load(ty, slot, 0);
                value_map.insert(*destination, result);
            }

            mir::Instruction::LocalSet { local, value } => {
                let slot = local_map[local];
                let store_value = value_map[value];
                builder.ins().stack_store(store_value, slot, 0);
            }

            mir::Instruction::Load {
                destination,
                pointer,
            } => {
                // TODO #Incomplete: need to know the loaded type from context
                let ptr_value = value_map[pointer];
                let result =
                    builder
                        .ins()
                        .load(cir::types::I64, cir::MemFlags::new(), ptr_value, 0);
                value_map.insert(*destination, result);
            }

            mir::Instruction::Store { pointer, value } => {
                let ptr_value = value_map[pointer];
                let store_value = value_map[value];
                builder
                    .ins()
                    .store(cir::MemFlags::new(), store_value, ptr_value, 0);
            }

            mir::Instruction::ExtractField { .. } => {
                return Err(CraneliftError::unsupported_instruction(
                    "ExtractField not yet implemented",
                ));
            }

            mir::Instruction::InsertField { .. } => {
                return Err(CraneliftError::unsupported_instruction(
                    "InsertField not yet implemented",
                ));
            }

            mir::Instruction::ExtractElement { .. } => {
                return Err(CraneliftError::unsupported_instruction(
                    "ExtractElement not yet implemented",
                ));
            }

            mir::Instruction::InsertElement { .. } => {
                return Err(CraneliftError::unsupported_instruction(
                    "InsertElement not yet implemented",
                ));
            }

            mir::Instruction::Call { .. } => {
                return Err(CraneliftError::unsupported_instruction(
                    "Call not yet implemented",
                ));
            }

            mir::Instruction::CallIndirect { .. } => {
                return Err(CraneliftError::unsupported_instruction(
                    "CallIndirect not yet implemented",
                ));
            }
        }

        Ok(())
    }

    /// Lower a MIR terminator to Cranelift IR.
    ///
    /// Terminators end basic blocks and transfer control flow.
    fn lower_terminator(
        &self,
        terminator: &mir::Terminator,
        builder: &mut FunctionBuilder<'_>,
        value_map: &HashMap<mir::Value, cir::Value>,
        block_map: &HashMap<mir::LocalNodeId<mir::Block>, cir::Block>,
    ) -> Result<(), CraneliftError> {
        match terminator {
            mir::Terminator::Return { value } => {
                if let Some(value) = value {
                    let return_value = value_map[value];
                    builder.ins().return_(&[return_value]);
                } else {
                    builder.ins().return_(&[]);
                }
            }

            mir::Terminator::Jump { target, arguments } => {
                let target_block = block_map[target];
                let args: Vec<cir::BlockArg> = arguments
                    .iter()
                    .map(|v| cir::BlockArg::from(value_map[v]))
                    .collect();
                builder.ins().jump(target_block, &args);
            }

            mir::Terminator::Branch {
                condition,
                then_target,
                then_arguments,
                else_target,
                else_arguments,
            } => {
                let cond_value = value_map[condition];
                let then_block = block_map[then_target];
                let else_block = block_map[else_target];
                let then_args: Vec<cir::BlockArg> = then_arguments
                    .iter()
                    .map(|v| cir::BlockArg::from(value_map[v]))
                    .collect();
                let else_args: Vec<cir::BlockArg> = else_arguments
                    .iter()
                    .map(|v| cir::BlockArg::from(value_map[v]))
                    .collect();

                builder
                    .ins()
                    .brif(cond_value, then_block, &then_args, else_block, &else_args);
            }

            mir::Terminator::Switch {
                value,
                default,
                default_arguments,
                cases,
            } => {
                // TODO #Performance: use br_table for better codegen
                // for now, we lower this as a chain of brif instructions
                let switch_value = value_map[value];
                let default_block = block_map[default];
                let default_args: Vec<cir::BlockArg> = default_arguments
                    .iter()
                    .map(|v| cir::BlockArg::from(value_map[v]))
                    .collect();

                if cases.is_empty() {
                    builder.ins().jump(default_block, &default_args);
                } else {
                    for case in cases {
                        let case_const = builder.ins().iconst(cir::types::I64, case.value);
                        let is_match = builder.ins().icmp(
                            cir::condcodes::IntCC::Equal,
                            switch_value,
                            case_const,
                        );

                        let case_block = block_map[&case.target];
                        let case_args: Vec<cir::BlockArg> = case
                            .arguments
                            .iter()
                            .map(|v| cir::BlockArg::from(value_map[v]))
                            .collect();

                        let next_block = builder.create_block();
                        let empty_args: Vec<cir::BlockArg> = vec![];
                        builder.ins().brif(
                            is_match,
                            case_block,
                            &case_args,
                            next_block,
                            &empty_args,
                        );
                        builder.switch_to_block(next_block);
                        builder.seal_block(next_block);
                    }

                    // final fallthrough to default
                    builder.ins().jump(default_block, &default_args);
                }
            }

            mir::Terminator::Unreachable => {
                builder.ins().trap(trap::UNREACHABLE);
            }
        }

        Ok(())
    }

    /// Lower a constant to Cranelift IR.
    fn lower_constant(
        &self,
        constant: &mir::Constant,
        builder: &mut FunctionBuilder<'_>,
    ) -> Result<cir::Value, CraneliftError> {
        match constant {
            mir::Constant::Boolean { value } => {
                let int_value = if *value { 1i64 } else { 0i64 };
                Ok(builder.ins().iconst(cir::types::I8, int_value))
            }

            mir::Constant::Int {
                value,
                width,
                is_signed: _,
            } => {
                let ty = match width {
                    8 => cir::types::I8,
                    16 => cir::types::I16,
                    32 => cir::types::I32,
                    64 => cir::types::I64,
                    _ => {
                        return Err(CraneliftError::unsupported_type(format!(
                            "integer width {width}"
                        )));
                    }
                };
                Ok(builder.ins().iconst(ty, *value))
            }

            mir::Constant::UInt { value, width } => {
                let ty = match width {
                    8 => cir::types::I8,
                    16 => cir::types::I16,
                    32 => cir::types::I32,
                    64 => cir::types::I64,
                    _ => {
                        return Err(CraneliftError::unsupported_type(format!(
                            "unsigned integer width {width}"
                        )));
                    }
                };
                Ok(builder.ins().iconst(ty, *value as i64))
            }

            mir::Constant::Float { bits, width } => match width {
                32 => Ok(builder
                    .ins()
                    .f32const(cir::immediates::Ieee32::with_bits(*bits as u32))),
                64 => Ok(builder
                    .ins()
                    .f64const(cir::immediates::Ieee64::with_bits(*bits))),
                _ => Err(CraneliftError::unsupported_type(format!(
                    "float width {width}"
                ))),
            },
        }
    }

    /// Lower a binary operation to Cranelift IR.
    fn lower_binary_op(
        &self,
        operator: mir::BinaryOperator,
        left: cir::Value,
        right: cir::Value,
        builder: &mut FunctionBuilder<'_>,
    ) -> Result<cir::Value, CraneliftError> {
        use cir::condcodes::{FloatCC, IntCC};

        let ins = builder.ins();

        let result = match operator {
            // integer arithmetic
            mir::BinaryOperator::Add => ins.iadd(left, right),
            mir::BinaryOperator::Subtract => ins.isub(left, right),
            mir::BinaryOperator::Multiply => ins.imul(left, right),
            mir::BinaryOperator::SignedDivide => ins.sdiv(left, right),
            mir::BinaryOperator::UnsignedDivide => ins.udiv(left, right),
            mir::BinaryOperator::SignedRemainder => ins.srem(left, right),
            mir::BinaryOperator::UnsignedRemainder => ins.urem(left, right),

            // floating point arithmetic
            mir::BinaryOperator::FloatAdd => ins.fadd(left, right),
            mir::BinaryOperator::FloatSubtract => ins.fsub(left, right),
            mir::BinaryOperator::FloatMultiply => ins.fmul(left, right),
            mir::BinaryOperator::FloatDivide => ins.fdiv(left, right),

            // bitwise operations
            mir::BinaryOperator::And => ins.band(left, right),
            mir::BinaryOperator::Or => ins.bor(left, right),
            mir::BinaryOperator::Xor => ins.bxor(left, right),
            mir::BinaryOperator::ShiftLeft => ins.ishl(left, right),
            mir::BinaryOperator::ArithmeticShiftRight => ins.sshr(left, right),
            mir::BinaryOperator::LogicalShiftRight => ins.ushr(left, right),

            // integer comparisons
            mir::BinaryOperator::Equal => ins.icmp(IntCC::Equal, left, right),
            mir::BinaryOperator::NotEqual => ins.icmp(IntCC::NotEqual, left, right),
            mir::BinaryOperator::SignedLessThan => ins.icmp(IntCC::SignedLessThan, left, right),
            mir::BinaryOperator::SignedLessEqual => {
                ins.icmp(IntCC::SignedLessThanOrEqual, left, right)
            }
            mir::BinaryOperator::SignedGreaterThan => {
                ins.icmp(IntCC::SignedGreaterThan, left, right)
            }
            mir::BinaryOperator::SignedGreaterEqual => {
                ins.icmp(IntCC::SignedGreaterThanOrEqual, left, right)
            }
            mir::BinaryOperator::UnsignedLessThan => ins.icmp(IntCC::UnsignedLessThan, left, right),
            mir::BinaryOperator::UnsignedLessEqual => {
                ins.icmp(IntCC::UnsignedLessThanOrEqual, left, right)
            }
            mir::BinaryOperator::UnsignedGreaterThan => {
                ins.icmp(IntCC::UnsignedGreaterThan, left, right)
            }
            mir::BinaryOperator::UnsignedGreaterEqual => {
                ins.icmp(IntCC::UnsignedGreaterThanOrEqual, left, right)
            }

            // floating point comparisons
            mir::BinaryOperator::FloatEqual => ins.fcmp(FloatCC::Equal, left, right),
            mir::BinaryOperator::FloatNotEqual => ins.fcmp(FloatCC::NotEqual, left, right),
            mir::BinaryOperator::FloatLessThan => ins.fcmp(FloatCC::LessThan, left, right),
            mir::BinaryOperator::FloatLessEqual => ins.fcmp(FloatCC::LessThanOrEqual, left, right),
            mir::BinaryOperator::FloatGreaterThan => ins.fcmp(FloatCC::GreaterThan, left, right),
            mir::BinaryOperator::FloatGreaterEqual => {
                ins.fcmp(FloatCC::GreaterThanOrEqual, left, right)
            }
        };

        Ok(result)
    }

    /// Lower a unary operation to Cranelift IR.
    fn lower_unary_op(
        &self,
        operator: mir::UnaryOperator,
        argument: cir::Value,
        builder: &mut FunctionBuilder<'_>,
    ) -> Result<cir::Value, CraneliftError> {
        let ins = builder.ins();

        let result = match operator {
            mir::UnaryOperator::Negate => ins.ineg(argument),
            mir::UnaryOperator::FloatNegate => ins.fneg(argument),
            mir::UnaryOperator::Not => ins.bnot(argument),
        };

        Ok(result)
    }

    /// Lower a cast operation to Cranelift IR.
    fn lower_cast(
        &self,
        kind: mir::CastKind,
        argument: cir::Value,
        to_type: cir::Type,
        builder: &mut FunctionBuilder<'_>,
    ) -> Result<cir::Value, CraneliftError> {
        let ins = builder.ins();

        let result = match kind {
            mir::CastKind::Bitcast => ins.bitcast(to_type, cir::MemFlags::new(), argument),
            mir::CastKind::Truncate => ins.ireduce(to_type, argument),
            mir::CastKind::ZeroExtend => ins.uextend(to_type, argument),
            mir::CastKind::SignExtend => ins.sextend(to_type, argument),
            mir::CastKind::FloatToSignedInt => ins.fcvt_to_sint(to_type, argument),
            mir::CastKind::FloatToUnsignedInt => ins.fcvt_to_uint(to_type, argument),
            mir::CastKind::SignedIntToFloat => ins.fcvt_from_sint(to_type, argument),
            mir::CastKind::UnsignedIntToFloat => ins.fcvt_from_uint(to_type, argument),
            mir::CastKind::FloatTruncate => ins.fdemote(to_type, argument),
            mir::CastKind::FloatExtend => ins.fpromote(to_type, argument),
            // pointer is already an integer in Cranelift
            mir::CastKind::PointerToInt | mir::CastKind::IntToPointer => argument,
        };

        Ok(result)
    }
}
