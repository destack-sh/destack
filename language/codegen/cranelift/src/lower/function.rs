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
//! ## Type Inference
//!
//! We also track MIR types for values (`type_map`) to support instructions like
//! `Load` that need to know what type to load. Types are inferred from:
//! - Function/block parameters (explicit `TypedValue`)
//! - Local variables (explicit type on `Local`)
//! - Instructions (inferred from operands and instruction kind)
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
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Switch};
use cranelift_module::FuncId;
use destack_mir as mir;
use destack_source::StringPool;

use super::r#type::lower_type;
use crate::{CodegenCraneliftError, trap};

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
    /// Pointer size in bytes for this target.
    pointer_bytes: u8,
}

#[allow(clippy::too_many_arguments)]
impl<'a> FunctionLowerer<'a> {
    /// Create a new function lowerer.
    pub(crate) fn new(
        tree: &'a mir::NodeTree,
        strings: &'a StringPool,
        function: &'a mir::Function,
        isa: &'a Arc<dyn TargetIsa>,
        cl_function_ids: &'a HashMap<mir::LocalNodeId<mir::Function>, FuncId>,
        pointer_bytes: u8,
    ) -> Self {
        Self {
            tree,
            strings,
            function,
            isa,
            cl_function_ids,
            pointer_bytes,
        }
    }

    /// Lower the function to Cranelift IR.
    ///
    /// Populates the provided Cranelift function with blocks, instructions,
    /// and control flow based on the MIR function.
    pub(crate) fn lower(self, target: &mut cir::Function) -> Result<(), CodegenCraneliftError> {
        let mut builder_context = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(target, &mut builder_context);
        let mut value_map: HashMap<mir::Value, cir::Value> = HashMap::new();
        let mut block_map: HashMap<mir::LocalNodeId<mir::Block>, cir::Block> = HashMap::new();
        let mut local_map: HashMap<mir::LocalNodeId<mir::Local>, cir::StackSlot> = HashMap::new();
        let mut type_map: HashMap<mir::Value, mir::LocalNodeId<mir::Type>> = HashMap::new();

        // phase 0: infer types
        self.infer_type_map(&mut type_map)?;

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
                &type_map,
            )?;
        }

        builder.finalize();
        Ok(())
    }

    /// Build the type_map map by walking the function:
    /// - Function parameters (have explicit types)
    /// - Block parameters (have explicit types)
    /// - Instructions (infer from instruction kind and operands)
    fn infer_type_map(
        &self,
        type_map: &mut HashMap<mir::Value, mir::LocalNodeId<mir::Type>>,
    ) -> Result<(), CodegenCraneliftError> {
        // function parameters have explicit types
        for param in &self.function.parameters {
            type_map.insert(param.value, param.ty);
        }

        // block parameters have explicit types
        for &block_id in &self.function.blocks {
            let block = self.tree.get(block_id);
            for param in &block.parameters {
                type_map.insert(param.value, param.ty);
            }
        }

        // infer types from instructions (must be in definition order for SSA)
        for &block_id in &self.function.blocks {
            let block = self.tree.get(block_id);
            for &inst_id in &block.instructions {
                let instruction = self.tree.get(inst_id);
                if let Some((dest, ty)) = self.infer_instruction_type(instruction, type_map) {
                    type_map.insert(dest, ty);
                }
            }
        }

        Ok(())
    }

    /// Infer the result type of an instruction (None if the instruction does not produce a value).
    fn infer_instruction_type(
        &self,
        instruction: &mir::Instruction,
        type_map: &HashMap<mir::Value, mir::LocalNodeId<mir::Type>>,
    ) -> Option<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        match instruction {
            // constants: type is embedded in the constant, but we don't have a Type node
            // we'll handle this specially when lowering
            mir::Instruction::Constant { .. } => None,

            // binary: result type = operand type (for arithmetic), or bool (for comparisons)
            mir::Instruction::Binary {
                destination,
                operator,
                left,
                ..
            } => {
                if operator.is_comparison() {
                    // comparisons produce bool, but we don't have a bool type node
                    // we'll handle this specially when lowering
                    None
                } else {
                    // arithmetic: result type = operand type
                    type_map.get(left).map(|ty| (*destination, *ty))
                }
            }

            // unary: result type = operand type
            mir::Instruction::Unary {
                destination,
                argument,
                ..
            } => type_map.get(argument).map(|ty| (*destination, *ty)),

            // cast: result type is explicit
            mir::Instruction::Cast {
                destination,
                to_type,
                ..
            } => Some((*destination, *to_type)),

            // local_get: result type = local's type
            mir::Instruction::LocalGet { destination, local } => {
                let local_data = self.tree.get(*local);
                Some((*destination, local_data.ty))
            }

            // local_set: no result
            mir::Instruction::LocalSet { .. } => None,

            // load: result type = pointee of pointer
            mir::Instruction::Load {
                destination,
                pointer,
            } => {
                if let Some(ptr_type_id) = type_map.get(pointer) {
                    let ptr_type = self.tree.get(*ptr_type_id);
                    if let mir::Type::Pointer { pointee } = ptr_type {
                        return Some((*destination, *pointee));
                    }
                }
                None
            }

            // store: no result
            mir::Instruction::Store { .. } => None,

            // extract_field: type is the field's type
            mir::Instruction::ExtractField {
                destination,
                aggregate,
                index,
            } => {
                if let Some(agg_type_id) = type_map.get(aggregate) {
                    let agg_type = self.tree.get(*agg_type_id);
                    match agg_type {
                        mir::Type::Struct { fields } => {
                            if let Some(field) = fields.get(*index as usize) {
                                return Some((*destination, field.ty));
                            }
                        }
                        mir::Type::Tuple { elements } => {
                            if let Some(&ty) = elements.get(*index as usize) {
                                return Some((*destination, ty));
                            }
                        }
                        _ => {}
                    }
                }
                None
            }

            // insert_field: result type = aggregate type
            mir::Instruction::InsertField {
                destination,
                aggregate,
                ..
            } => type_map.get(aggregate).map(|ty| (*destination, *ty)),

            // extract_element: type is array element type
            mir::Instruction::ExtractElement {
                destination, array, ..
            } => {
                if let Some(arr_type_id) = type_map.get(array) {
                    let arr_type = self.tree.get(*arr_type_id);
                    if let mir::Type::Array { element, .. } = arr_type {
                        return Some((*destination, *element));
                    }
                }
                None
            }

            // insert_element: result type = array type
            mir::Instruction::InsertElement {
                destination, array, ..
            } => type_map.get(array).map(|ty| (*destination, *ty)),

            // call: would need function signature lookup
            // TODO #Incomplete: implement function signature lookup
            mir::Instruction::Call { .. } | mir::Instruction::CallIndirect { .. } => None,
        }
    }

    /// Create Cranelift stack slots for MIR locals.
    fn create_locals(
        &self,
        builder: &mut FunctionBuilder<'_>,
        local_map: &mut HashMap<mir::LocalNodeId<mir::Local>, cir::StackSlot>,
    ) -> Result<(), CodegenCraneliftError> {
        for &local_id in &self.function.locals {
            let local = self.tree.get(local_id);
            let ty = lower_type(self.tree, local.ty, self.pointer_bytes)?;
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
    ) -> Result<(), CodegenCraneliftError> {
        // first, create all blocks
        for &block_id in &self.function.blocks {
            let block = builder.create_block();
            block_map.insert(block_id, block);
        }

        // set entry block and add function parameters as entry block parameters
        let entry_block = block_map[&self.function.entry];

        // function parameters become entry block parameters in Cranelift
        for param in &self.function.parameters {
            let ty = lower_type(self.tree, param.ty, self.pointer_bytes)?;
            let value = builder.append_block_param(entry_block, ty);
            value_map.insert(param.value, value);
        }

        // now add MIR block parameters for non-entry blocks
        for &block_id in &self.function.blocks {
            let mir_block = self.tree.get(block_id);
            let target_block = block_map[&block_id];

            // add block parameters (these are for phi nodes / join points)
            for param in &mir_block.parameters {
                let ty = lower_type(self.tree, param.ty, self.pointer_bytes)?;
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
        type_map: &HashMap<mir::Value, mir::LocalNodeId<mir::Type>>,
    ) -> Result<(), CodegenCraneliftError> {
        let mir_block = self.tree.get(block_id);
        let target_block = block_map[&block_id];

        // switch to this block (may already be current for entry)
        if builder.current_block() != Some(target_block) {
            builder.switch_to_block(target_block);
        }

        // lower instructions
        for &instruction_id in &mir_block.instructions {
            self.lower_instruction(instruction_id, builder, value_map, local_map, type_map)?;
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
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        builder: &mut FunctionBuilder<'_>,
        value_map: &mut HashMap<mir::Value, cir::Value>,
        local_map: &HashMap<mir::LocalNodeId<mir::Local>, cir::StackSlot>,
        type_map: &HashMap<mir::Value, mir::LocalNodeId<mir::Type>>,
    ) -> Result<(), CodegenCraneliftError> {
        let instruction = self.tree.get(instruction_id);
        match instruction {
            mir::Instruction::Constant { destination, value } => {
                let result = self.lower_constant(instruction_id.into_any(), value, builder)?;
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
                let target_type = lower_type(self.tree, *to_type, self.pointer_bytes)?;
                let result = self.lower_cast(*kind, arg_value, target_type, builder)?;
                value_map.insert(*destination, result);
            }

            mir::Instruction::LocalGet { destination, local } => {
                let slot = local_map[local];
                let local_data = self.tree.get(*local);
                let ty = lower_type(self.tree, local_data.ty, self.pointer_bytes)?;
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
                let ptr_value = value_map[pointer];
                let loaded_type_id = type_map.get(destination).ok_or_else(|| {
                    CodegenCraneliftError::MissingType {
                        node: instruction_id.into_any(),
                        message: Some(format!(
                            "could not infer type for load destination {destination:?} from pointer {pointer:?}"
                        )),
                    }
                })?;
                let loaded_type = lower_type(self.tree, *loaded_type_id, self.pointer_bytes)?;

                let result = builder
                    .ins()
                    .load(loaded_type, cir::MemFlags::new(), ptr_value, 0);
                value_map.insert(*destination, result);
            }

            mir::Instruction::Store { pointer, value } => {
                let ptr_value = value_map[pointer];
                let store_value = value_map[value];
                builder
                    .ins()
                    .store(cir::MemFlags::new(), store_value, ptr_value, 0);
            }

            // TODO #Incomplete: implement codegen for field / element / call instructions

            mir::Instruction::ExtractField { .. } => {
                return Err(CodegenCraneliftError::unsupported_instruction(
                    "ExtractField not yet implemented",
                    instruction_id.into_any(),
                ));
            }

            mir::Instruction::InsertField { .. } => {
                return Err(CodegenCraneliftError::unsupported_instruction(
                    "InsertField not yet implemented",
                    instruction_id.into_any(),
                ));
            }

            mir::Instruction::ExtractElement { .. } => {
                return Err(CodegenCraneliftError::unsupported_instruction(
                    "ExtractElement not yet implemented",
                    instruction_id.into_any(),
                ));
            }

            mir::Instruction::InsertElement { .. } => {
                return Err(CodegenCraneliftError::unsupported_instruction(
                    "InsertElement not yet implemented",
                    instruction_id.into_any(),
                ));
            }

            mir::Instruction::Call { .. } => {
                return Err(CodegenCraneliftError::unsupported_instruction(
                    "Call not yet implemented",
                    instruction_id.into_any(),
                ));
            }

            mir::Instruction::CallIndirect { .. } => {
                return Err(CodegenCraneliftError::unsupported_instruction(
                    "CallIndirect not yet implemented",
                    instruction_id.into_any(),
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
    ) -> Result<(), CodegenCraneliftError> {
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
                let arguments: Vec<cir::BlockArg> = arguments
                    .iter()
                    .map(|v| cir::BlockArg::from(value_map[v]))
                    .collect();
                builder.ins().jump(target_block, &arguments);
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
                let then_arguments: Vec<cir::BlockArg> = then_arguments
                    .iter()
                    .map(|v| cir::BlockArg::from(value_map[v]))
                    .collect();
                let else_arguments: Vec<cir::BlockArg> = else_arguments
                    .iter()
                    .map(|v| cir::BlockArg::from(value_map[v]))
                    .collect();

                builder
                    .ins()
                    .brif(cond_value, then_block, &then_arguments, else_block, &else_arguments);
            }

            mir::Terminator::Switch {
                value,
                default,
                default_arguments,
                cases,
            } => {
                self.lower_switch(
                    value_map[value],
                    block_map[default],
                    default_arguments,
                    cases,
                    builder,
                    value_map,
                    block_map,
                )?;
            }

            mir::Terminator::Unreachable => {
                builder.ins().trap(trap::UNREACHABLE);
            }
        }

        Ok(())
    }

    /// Lower a switch terminator using Cranelift's Switch helper.
    fn lower_switch(
        &self,
        switch_value: cir::Value,
        default_block: cir::Block,
        default_arguments: &[mir::Value],
        cases: &[mir::SwitchCase],
        builder: &mut FunctionBuilder<'_>,
        value_map: &HashMap<mir::Value, cir::Value>,
        block_map: &HashMap<mir::LocalNodeId<mir::Block>, cir::Block>,
    ) -> Result<(), CodegenCraneliftError> {
        // if no cases, just jump to default
        if cases.is_empty() {
            let default_arguments: Vec<cir::BlockArg> = default_arguments
                .iter()
                .map(|v| cir::BlockArg::from(value_map[v]))
                .collect();
            builder.ins().jump(default_block, &default_arguments);
            return Ok(());
        }

        // check if any cases have block arguments
        // Cranelift's Switch doesn't support block arguments directly,
        // so we need to create intermediate blocks for cases with arguments
        let has_block_arguments =
            !default_arguments.is_empty() || cases.iter().any(|c| !c.arguments.is_empty());

        if has_block_arguments {
            // fall back to chain of brif for cases with arguments
            self.lower_switch_with_arguments(
                switch_value,
                default_block,
                default_arguments,
                cases,
                builder,
                value_map,
                block_map,
            )
        } else {
            // use Cranelift's Switch for simple cases (no block arguments)
            let mut switch = Switch::new();
            for case in cases {
                let case_block = block_map[&case.target];
                switch.set_entry(case.value as u128, case_block);
            }
            switch.emit(builder, switch_value, default_block);
            Ok(())
        }
    }

    /// Lower a switch with block arguments using a chain of brif instructions.
    fn lower_switch_with_arguments(
        &self,
        switch_value: cir::Value,
        default_block: cir::Block,
        default_arguments: &[mir::Value],
        cases: &[mir::SwitchCase],
        builder: &mut FunctionBuilder<'_>,
        value_map: &HashMap<mir::Value, cir::Value>,
        block_map: &HashMap<mir::LocalNodeId<mir::Block>, cir::Block>,
    ) -> Result<(), CodegenCraneliftError> {
        let default_arguments: Vec<cir::BlockArg> = default_arguments
            .iter()
            .map(|v| cir::BlockArg::from(value_map[v]))
            .collect();

        for case in cases {
            let case_const = builder.ins().iconst(self.pointer_type(), case.value);
            let is_match =
                builder
                    .ins()
                    .icmp(cir::condcodes::IntCC::Equal, switch_value, case_const);

            let case_block = block_map[&case.target];
            let case_arguments: Vec<cir::BlockArg> = case
                .arguments
                .iter()
                .map(|v| cir::BlockArg::from(value_map[v]))
                .collect();

            let next_block = builder.create_block();
            let empty_arguments: Vec<cir::BlockArg> = vec![];
            builder
                .ins()
                .brif(is_match, case_block, &case_arguments, next_block, &empty_arguments);
            builder.switch_to_block(next_block);
            builder.seal_block(next_block);
        }

        // final fallthrough to default
        builder.ins().jump(default_block, &default_arguments);
        Ok(())
    }

    /// Get the Cranelift type for pointers on this target.
    fn pointer_type(&self) -> cir::Type {
        match self.pointer_bytes {
            4 => cir::types::I32,
            8 => cir::types::I64,
            _ => panic!("unsupported pointer size: {} bytes", self.pointer_bytes),
        }
    }

    /// Lower a constant to Cranelift IR.
    fn lower_constant(
        &self,
        node_id: mir::LocalNodeIdAny,
        constant: &mir::Constant,
        builder: &mut FunctionBuilder<'_>,
    ) -> Result<cir::Value, CodegenCraneliftError> {
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
                        return Err(CodegenCraneliftError::unsupported_type(
                            format!("integer width {width}",),
                            node_id,
                        ));
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
                        return Err(CodegenCraneliftError::unsupported_type(
                            format!("unsigned integer width {width}",),
                            node_id,
                        ));
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
                _ => Err(CodegenCraneliftError::unsupported_type(
                    format!("float width {width} not supported",),
                    node_id,
                )),
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
    ) -> Result<cir::Value, CodegenCraneliftError> {
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
    ) -> Result<cir::Value, CodegenCraneliftError> {
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
    ) -> Result<cir::Value, CodegenCraneliftError> {
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
