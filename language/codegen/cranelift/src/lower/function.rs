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
use cranelift_module::{DataId, FuncId, Module};
use cranelift_object::ObjectModule;
use destack_mir as mir;
use destack_source::StringPool;

use super::layout::compute_tuple_element_offset;
use super::r#type::lower_type;
use crate::{CodegenCraneliftError, CodegenCraneliftResult, trap};

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
    /// The target Cranelift module.
    cl_module: &'a mut ObjectModule,
    /// Function id mapping for calls.
    cl_function_ids: &'a HashMap<mir::LocalNodeId<mir::Function>, FuncId>,
    /// Global data id mapping (MIR global -> Cranelift DataId).
    cl_global_data_ids: &'a HashMap<mir::LocalNodeId<mir::Global>, DataId>,
    /// Map from MIR function id to Cranelift FuncRef (populated during lowering).
    function_ref_map: HashMap<mir::LocalNodeId<mir::Function>, cir::FuncRef>,
    /// Map from MIR global id to Cranelift GlobalValue (populated during lowering).
    global_map: HashMap<mir::LocalNodeId<mir::Global>, cir::GlobalValue>,
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
        cl_module: &'a mut ObjectModule,
        cl_function_ids: &'a HashMap<mir::LocalNodeId<mir::Function>, FuncId>,
        cl_global_data_ids: &'a HashMap<mir::LocalNodeId<mir::Global>, DataId>,
        pointer_bytes: u8,
    ) -> Self {
        Self {
            tree,
            strings,
            function,
            isa,
            cl_module,
            cl_function_ids,
            cl_global_data_ids,
            function_ref_map: HashMap::new(),
            global_map: HashMap::new(),
            pointer_bytes,
        }
    }

    /// Lower the function to Cranelift IR.
    ///
    /// Populates the provided Cranelift function with blocks, instructions,
    /// and control flow based on the MIR function.
    pub(crate) fn lower(mut self, target: &mut cir::Function) -> CodegenCraneliftResult<()> {
        let mut value_map: HashMap<mir::Value, cir::Value> = HashMap::new();
        let mut block_map: HashMap<mir::LocalNodeId<mir::Block>, cir::Block> = HashMap::new();
        let mut local_map: HashMap<mir::LocalNodeId<mir::Local>, cir::StackSlot> = HashMap::new();
        let mut type_map: HashMap<mir::Value, mir::LocalNodeId<mir::Type>> = HashMap::new();

        // phase 0: infer types
        self.infer_type_map(&mut type_map)?;

        // phase 0.5: pre-declare all called functions in the current function
        // (must be done before creating the FunctionBuilder)
        self.declare_called_functions(target)?;

        let mut builder_context = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(target, &mut builder_context);

        // phase 1: create stack slots for locals
        self.create_locals(&mut builder, &mut local_map)?;

        // phase 2: create all blocks and set up parameters
        self.create_blocks(&mut builder, &mut value_map, &mut block_map)?;

        // phase 3+4: lower each block's instructions and terminator
        let entry_block_id = self.function.entry;
        for &block_id in &self.function.blocks {
            self.lower_block(
                &mut builder,
                block_id,
                &mut value_map,
                &block_map,
                &local_map,
                &type_map,
            )?;

            // seal non-entry blocks (entry block was already sealed in create_blocks)
            if entry_block_id != Some(block_id) {
                let target_block = block_map[&block_id];
                builder.seal_block(target_block);
            }
        }

        builder.finalize();
        Ok(())
    }

    /// Pre-declare all functions that this function calls.
    /// Populates `self.function_ref_map` with the mapping.
    fn declare_called_functions(
        &mut self,
        target: &mut cir::Function,
    ) -> CodegenCraneliftResult<()> {
        // scan all blocks for Call instructions
        for &block_id in &self.function.blocks {
            let block = self.tree.get(block_id);
            for &inst_id in &block.instructions {
                let inst = self.tree.get(inst_id);
                if let mir::Instruction::Call { function, .. } = inst
                    && !self.function_ref_map.contains_key(function)
                {
                    let callee_function_id =
                        self.cl_function_ids.get(function).ok_or_else(|| {
                            CodegenCraneliftError::Internal {
                                message: format!("unknown function {function:?}"),
                            }
                        })?;
                    let function_ref = self
                        .cl_module
                        .declare_func_in_func(*callee_function_id, target);
                    self.function_ref_map.insert(*function, function_ref);
                }
            }
        }

        Ok(())
    }

    /// Build the type_map map by walking the function:
    /// - Function parameters (have explicit types)
    /// - Block parameters (have explicit types)
    /// - Instructions (infer from instruction kind and operands)
    fn infer_type_map(
        &self,
        type_map: &mut HashMap<mir::Value, mir::LocalNodeId<mir::Type>>,
    ) -> CodegenCraneliftResult<()> {
        let mut pointer_pointee_map: HashMap<mir::Value, mir::LocalNodeId<mir::Type>> =
            HashMap::new();

        // function parameters have explicit types
        for param in &self.function.parameters {
            type_map.insert(param.value, param.ty);
            // if param is a pointer type, record its pointee
            let ty = self.tree.get(param.ty);
            if let mir::Type::RawPointer { pointee } | mir::Type::ManagedReference { pointee, .. } =
                ty
            {
                pointer_pointee_map.insert(param.value, *pointee);
            }
        }

        // block parameters have explicit types
        for &block_id in &self.function.blocks {
            let block = self.tree.get(block_id);
            for param in &block.parameters {
                type_map.insert(param.value, param.ty);
                // If param is a pointer type, record its pointee
                let ty = self.tree.get(param.ty);
                if let mir::Type::RawPointer { pointee }
                | mir::Type::ManagedReference { pointee, .. } = ty
                {
                    pointer_pointee_map.insert(param.value, *pointee);
                }
            }
        }

        // infer types from instructions (must be in definition order for SSA)
        for &block_id in &self.function.blocks {
            let block = self.tree.get(block_id);
            for &inst_id in &block.instructions {
                let instruction = self.tree.get(inst_id);
                self.infer_pointer_pointee(instruction, &mut pointer_pointee_map);
                if let Some((dest, ty)) =
                    self.infer_instruction_type(instruction, type_map, &pointer_pointee_map)
                {
                    type_map.insert(dest, ty);
                }
            }
        }

        Ok(())
    }

    /// Record pointee types for pointer-producing instructions.
    fn infer_pointer_pointee(
        &self,
        instruction: &mir::Instruction,
        pointer_pointee_map: &mut HashMap<mir::Value, mir::LocalNodeId<mir::Type>>,
    ) {
        match instruction {
            // stack_allocate -> rawptr<layout>
            mir::Instruction::StackAlloc {
                destination,
                layout,
            } => {
                pointer_pointee_map.insert(*destination, *layout);
            }
            // raw_allocate -> rawptr<layout>
            mir::Instruction::RawAlloc {
                destination,
                layout,
                ..
            } => {
                pointer_pointee_map.insert(*destination, *layout);
            }
            // managed_allocate -> managed ref to layout
            mir::Instruction::ManagedAlloc {
                destination,
                layout,
            } => {
                pointer_pointee_map.insert(*destination, *layout);
            }
            // managed_allocate_array -> array pointer -> element type
            mir::Instruction::ManagedAllocArray {
                destination,
                element,
                ..
            } => {
                // array pointer -> element type
                pointer_pointee_map.insert(*destination, *element);
            }
            // global_addr -> rawptr<global's type>
            mir::Instruction::GlobalAddr {
                destination,
                global,
            } => {
                let global_data = self.tree.get(*global);
                pointer_pointee_map.insert(*destination, global_data.ty);
            }
            _ => {}
        }
    }

    /// Infer the result type of an instruction (None if the instruction does not produce a value).
    fn infer_instruction_type(
        &self,
        instruction: &mir::Instruction,
        type_map: &HashMap<mir::Value, mir::LocalNodeId<mir::Type>>,
        pointer_pointee_map: &HashMap<mir::Value, mir::LocalNodeId<mir::Type>>,
    ) -> Option<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        match instruction {
            // constants: type is embedded in the constant, but we don't have a Type node
            // (we'll handle this specially when lowering instructions)
            mir::Instruction::Const { .. } => None,

            // binary: result type = operand type (for arithmetic), or bool (for comparisons)
            mir::Instruction::Binary {
                destination,
                operator,
                left,
                ..
            } => {
                if operator.is_comparison() {
                    // comparisons produce bool, but we don't have a bool type node
                    // (we'll handle this specially when lowering instructions)
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

            // global_addr: result type = pointer (but we don't track pointer types here)
            mir::Instruction::GlobalAddr { .. } => None,

            // global_const: result type = global's type
            mir::Instruction::GlobalConst {
                destination,
                global,
            } => {
                let global_data = self.tree.get(*global);
                Some((*destination, global_data.ty))
            }

            // load: result type = pointee of pointer
            mir::Instruction::Load {
                destination,
                pointer,
            } => {
                // try pointer_pointee_map (for stack_allocate, etc.)
                if let Some(&pointee) = pointer_pointee_map.get(pointer) {
                    return Some((*destination, pointee));
                }
                // fallback to extracting from pointer type
                if let Some(ptr_type_id) = type_map.get(pointer) {
                    let ptr_type = self.tree.get(*ptr_type_id);
                    if let mir::Type::RawPointer { pointee } = ptr_type {
                        return Some((*destination, *pointee));
                    } else if let mir::Type::ManagedReference { pointee, .. } = ptr_type {
                        return Some((*destination, *pointee));
                    }
                }
                None
            }

            // store: no result
            mir::Instruction::Store { .. } => None,

            // extract_field: type is the field's type
            mir::Instruction::FieldGet {
                destination,
                aggregate,
                index,
            } => {
                if let Some(aggregate_type_id) = type_map.get(aggregate) {
                    let aggregate_type = self.tree.get(*aggregate_type_id);
                    match aggregate_type {
                        mir::Type::Struct { fields } => {
                            if let Some(field_id) = fields.get(*index as usize) {
                                let field = self.tree.get(*field_id);
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
            mir::Instruction::FieldSet {
                destination,
                aggregate,
                ..
            } => type_map.get(aggregate).map(|ty| (*destination, *ty)),

            // extract_element: type is array element type
            mir::Instruction::ElementGet {
                destination, array, ..
            } => {
                if let Some(array_type_id) = type_map.get(array) {
                    let array_type = self.tree.get(*array_type_id);
                    if let mir::Type::Array { element, .. } = array_type {
                        return Some((*destination, *element));
                    }
                }
                None
            }

            // insert_element: result type = array type
            mir::Instruction::ElementSet {
                destination, array, ..
            } => type_map.get(array).map(|ty| (*destination, *ty)),

            // call: look up the callee's return type
            mir::Instruction::Call {
                destination,
                function,
                ..
            } => {
                if let Some(dest) = destination {
                    let callee = self.tree.get(*function);
                    Some((*dest, callee.return_type))
                } else {
                    None
                }
            }

            // call_indirect: look up return type from function pointer type
            mir::Instruction::CallIndirect {
                destination,
                callee,
                ..
            } => {
                if let Some(dest) = destination
                    && let Some(callee_type_id) = type_map.get(callee)
                {
                    let callee_type = self.tree.get(*callee_type_id);
                    if let mir::Type::FunctionPointer { result, .. } = callee_type {
                        return Some((*dest, *result));
                    }
                }
                None
            }

            // stack_allocate: result type is rawptr<layout>
            mir::Instruction::StackAlloc { .. } => None,

            // managed_allocate: result type is managed reference
            mir::Instruction::ManagedAlloc { .. } => None,
            mir::Instruction::ManagedAllocArray { .. } => None,

            // raw_allocate: result type is rawptr
            mir::Instruction::RawAlloc { .. } => None,

            // these don't produce values
            mir::Instruction::RawFree { .. } => None,
        }
    }

    /// Create Cranelift stack slots for MIR locals.
    fn create_locals(
        &self,
        builder: &mut FunctionBuilder<'_>,
        local_map: &mut HashMap<mir::LocalNodeId<mir::Local>, cir::StackSlot>,
    ) -> CodegenCraneliftResult<()> {
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
    ) -> CodegenCraneliftResult<()> {
        // first, create all blocks
        for &block_id in &self.function.blocks {
            let block = builder.create_block();
            block_map.insert(block_id, block);
        }

        // set entry block and add function parameters as entry block parameters
        let entry_block_id =
            self.function
                .entry
                .ok_or_else(|| CodegenCraneliftError::Internal {
                    message: "cannot lower external function without entry block".to_string(),
                })?;
        let entry_block = block_map[&entry_block_id];

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
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        block_id: mir::LocalNodeId<mir::Block>,
        value_map: &mut HashMap<mir::Value, cir::Value>,
        block_map: &HashMap<mir::LocalNodeId<mir::Block>, cir::Block>,
        local_map: &HashMap<mir::LocalNodeId<mir::Local>, cir::StackSlot>,
        type_map: &HashMap<mir::Value, mir::LocalNodeId<mir::Type>>,
    ) -> CodegenCraneliftResult<()> {
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

        Ok(())
    }

    /// Lower a MIR instruction to Cranelift IR.
    ///
    /// Each instruction that produces a value records its result in `value_map`.
    /// Instructions that consume values look them up in `value_map`.
    fn lower_instruction(
        &mut self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        builder: &mut FunctionBuilder<'_>,
        value_map: &mut HashMap<mir::Value, cir::Value>,
        local_map: &HashMap<mir::LocalNodeId<mir::Local>, cir::StackSlot>,
        type_map: &HashMap<mir::Value, mir::LocalNodeId<mir::Type>>,
    ) -> CodegenCraneliftResult<()> {
        let instruction = self.tree.get(instruction_id);
        match instruction {
            // const -> iconst/fconst (type-specific immediate load)
            mir::Instruction::Const { destination, value } => {
                let result = self.lower_constant(instruction_id.into_any(), value, builder)?;
                value_map.insert(*destination, result);
            }

            // binary -> iadd/isub/imul/etc (arithmetic) or icmp (comparison)
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

            // unary -> ineg/fneg/bnot (type-specific unary operation)
            mir::Instruction::Unary {
                destination,
                operator,
                argument,
            } => {
                let argument_value = value_map[argument];
                let result = self.lower_unary_op(*operator, argument_value, builder)?;
                value_map.insert(*destination, result);
            }

            // cast -> sextend/uextend/ireduce/bitcast/etc (type conversion)
            mir::Instruction::Cast {
                destination,
                kind,
                argument,
                to_type,
            } => {
                let argument_value = value_map[argument];
                let target_type = lower_type(self.tree, *to_type, self.pointer_bytes)?;
                let result = self.lower_cast(*kind, argument_value, target_type, builder)?;
                value_map.insert(*destination, result);
            }

            // local_get -> stack_load (load from stack slot)
            mir::Instruction::LocalGet { destination, local } => {
                let slot = local_map[local];
                let local_data = self.tree.get(*local);
                let ty = lower_type(self.tree, local_data.ty, self.pointer_bytes)?;
                let result = builder.ins().stack_load(ty, slot, 0);
                value_map.insert(*destination, result);
            }

            // local_set -> stack_store (store to stack slot)
            mir::Instruction::LocalSet { local, value } => {
                let slot = local_map[local];
                let store_value = value_map[value];
                builder.ins().stack_store(store_value, slot, 0);
            }

            // global_addr -> symbol_value (get address of mutable global)
            mir::Instruction::GlobalAddr {
                destination,
                global,
            } => {
                let global_value = self.get_or_declare_global(*global, builder)?;
                let ptr = builder
                    .ins()
                    .global_value(self.pointer_type(), global_value);
                value_map.insert(*destination, ptr);
            }

            // global_const -> symbol_value + load (for immutable globals)
            mir::Instruction::GlobalConst {
                destination,
                global,
            } => {
                let global_data = self.tree.get(*global);
                let global_value = self.get_or_declare_global(*global, builder)?;
                let ptr = builder
                    .ins()
                    .global_value(self.pointer_type(), global_value);

                // load the value from the global
                let result_type = lower_type(self.tree, global_data.ty, self.pointer_bytes)?;
                let result = builder
                    .ins()
                    .load(result_type, cir::MemFlags::trusted(), ptr, 0);
                value_map.insert(*destination, result);
            }

            // load -> load (memory read through pointer)
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

            // store -> store (memory write through pointer)
            mir::Instruction::Store { pointer, value } => {
                let ptr_value = value_map[pointer];
                let store_value = value_map[value];
                builder
                    .ins()
                    .store(cir::MemFlags::new(), store_value, ptr_value, 0);
            }

            // extract_field -> load at computed offset (struct/tuple field read)
            mir::Instruction::FieldGet {
                destination,
                aggregate,
                index,
            } => {
                // aggregate type
                let aggregate_type_id =
                    type_map
                        .get(aggregate)
                        .ok_or_else(|| CodegenCraneliftError::MissingType {
                            node: instruction_id.into_any(),
                            message: Some("could not infer type for aggregate in FieldGet".into()),
                        })?;
                let aggregate_type = self.tree.get(*aggregate_type_id);

                // field offset and type
                let (field_offset, field_type_id) = match aggregate_type {
                    mir::Type::Struct { fields } => {
                        let field_id = fields.get(*index as usize).ok_or_else(|| {
                            CodegenCraneliftError::out_of_bounds(
                                instruction_id.into_any(),
                                *index,
                                fields.len(),
                            )
                        })?;
                        let field = self.tree.get(*field_id);
                        (field.offset, field.ty)
                    }
                    mir::Type::Tuple { elements } => {
                        let element_type_id = elements.get(*index as usize).ok_or_else(|| {
                            CodegenCraneliftError::out_of_bounds(
                                instruction_id.into_any(),
                                *index,
                                elements.len(),
                            )
                        })?;
                        let offset = compute_tuple_element_offset(
                            self.tree,
                            elements,
                            *index,
                            self.pointer_bytes,
                        )?;
                        (offset, *element_type_id)
                    }
                    _ => {
                        return Err(CodegenCraneliftError::Internal {
                            message: "FieldGet on non-aggregate type".into(),
                        });
                    }
                };

                // load from aggregate_ptr + offset
                let field_type = lower_type(self.tree, field_type_id, self.pointer_bytes)?;
                let aggregate_ptr = value_map[aggregate];
                let result = builder.ins().load(
                    field_type,
                    cir::MemFlags::new(),
                    aggregate_ptr,
                    field_offset as i32,
                );
                value_map.insert(*destination, result);
            }

            // insert_field -> store at computed offset (struct/tuple field write)
            mir::Instruction::FieldSet {
                destination,
                aggregate,
                index,
                value,
            } => {
                // aggregate type
                let aggregate_type_id =
                    type_map
                        .get(aggregate)
                        .ok_or_else(|| CodegenCraneliftError::MissingType {
                            node: instruction_id.into_any(),
                            message: Some("could not infer type for aggregate in FieldSet".into()),
                        })?;
                let aggregate_type = self.tree.get(*aggregate_type_id);

                // field
                let field_offset = match aggregate_type {
                    mir::Type::Struct { fields } => {
                        let field_id = fields.get(*index as usize).ok_or_else(|| {
                            CodegenCraneliftError::out_of_bounds(
                                instruction_id.into_any(),
                                *index,
                                fields.len(),
                            )
                        })?;
                        let field = self.tree.get(*field_id);
                        field.offset
                    }
                    mir::Type::Tuple { elements } => compute_tuple_element_offset(
                        self.tree,
                        elements,
                        *index,
                        self.pointer_bytes,
                    )?,
                    _ => {
                        return Err(CodegenCraneliftError::Internal {
                            message: "FieldSet on non-aggregate type".into(),
                        });
                    }
                };

                // store to aggregate_ptr + offset
                let aggregate_ptr = value_map[aggregate];
                let store_value = value_map[value];
                builder.ins().store(
                    cir::MemFlags::new(),
                    store_value,
                    aggregate_ptr,
                    field_offset as i32,
                );
                // the result is the aggregate pointer (in-place mutation)
                value_map.insert(*destination, aggregate_ptr);
            }

            // extract_element -> load at ptr + index * elem_size (array element read)
            mir::Instruction::ElementGet {
                destination,
                array,
                index,
            } => {
                // array type
                let array_type_id =
                    type_map
                        .get(array)
                        .ok_or_else(|| CodegenCraneliftError::MissingType {
                            node: instruction_id.into_any(),
                            message: Some("could not infer type for array in ElementGet".into()),
                        })?;
                let array_type = self.tree.get(*array_type_id);

                // element type
                let element_type_id = match array_type {
                    mir::Type::Array { element, .. } => *element,
                    _ => {
                        return Err(CodegenCraneliftError::Internal {
                            message: "ElementGet on non-array type".into(),
                        });
                    }
                };
                let element_type = lower_type(self.tree, element_type_id, self.pointer_bytes)?;
                let element_size = element_type.bytes() as i64;

                // array pointer + index * element_size
                let array_ptr = value_map[array];
                let idx_value = value_map[index];
                let offset = builder.ins().imul_imm(idx_value, element_size);
                let element_ptr = builder.ins().iadd(array_ptr, offset);
                let result = builder
                    .ins()
                    .load(element_type, cir::MemFlags::new(), element_ptr, 0);
                value_map.insert(*destination, result);
            }

            // insert_element -> store at ptr + index * elem_size (array element write)
            mir::Instruction::ElementSet {
                destination,
                array,
                index,
                value,
            } => {
                // array type
                let array_type_id =
                    type_map
                        .get(array)
                        .ok_or_else(|| CodegenCraneliftError::MissingType {
                            node: instruction_id.into_any(),
                            message: Some("could not infer type for array in ElementSet".into()),
                        })?;
                let array_type = self.tree.get(*array_type_id);

                // element type
                let element_type_id = match array_type {
                    mir::Type::Array { element, .. } => *element,
                    _ => {
                        return Err(CodegenCraneliftError::Internal {
                            message: "ElementSet on non-array type".into(),
                        });
                    }
                };
                let element_type = lower_type(self.tree, element_type_id, self.pointer_bytes)?;
                let element_size = element_type.bytes() as i64;

                // array pointer + index * element_size
                let array_ptr = value_map[array];
                let idx_value = value_map[index];
                let store_value = value_map[value];
                let offset = builder.ins().imul_imm(idx_value, element_size);
                let element_ptr = builder.ins().iadd(array_ptr, offset);
                builder
                    .ins()
                    .store(cir::MemFlags::new(), store_value, element_ptr, 0);
                // the result is the array pointer itself
                value_map.insert(*destination, array_ptr);
            }

            // call -> call (direct function call via pre-declared FuncRef)
            mir::Instruction::Call {
                destination,
                function,
                arguments,
            } => {
                let function_ref = self.function_ref_map.get(function).ok_or_else(|| {
                    CodegenCraneliftError::Internal {
                        message: format!("function {function:?} not declared"),
                    }
                })?;

                // gather argument values
                let argument_values: Vec<cir::Value> =
                    arguments.iter().map(|v| value_map[v]).collect();

                // make the call
                let call_instruction = builder.ins().call(*function_ref, &argument_values);

                // get return value if any
                if let Some(dest) = destination {
                    let results = builder.inst_results(call_instruction);
                    if !results.is_empty() {
                        value_map.insert(*dest, results[0]);
                    }
                }
            }

            // call_indirect -> call_indirect (indirect call through function pointer)
            mir::Instruction::CallIndirect {
                destination,
                callee,
                arguments,
            } => {
                // callee type
                let callee_type_id =
                    type_map
                        .get(callee)
                        .ok_or_else(|| CodegenCraneliftError::MissingType {
                            node: instruction_id.into_any(),
                            message: Some("could not infer type for indirect call callee".into()),
                        })?;
                let callee_type = self.tree.get(*callee_type_id);

                // function pointer type
                let (params, result) = match callee_type {
                    mir::Type::FunctionPointer { parameters, result } => (parameters, *result),
                    _ => {
                        return Err(CodegenCraneliftError::Internal {
                            message: "CallIndirect callee is not a function pointer".into(),
                        });
                    }
                };

                // signature
                let call_conv = self.isa.default_call_conv();
                let mut signature = cir::Signature::new(call_conv);
                for parameter_ty in params {
                    let ty = lower_type(self.tree, *parameter_ty, self.pointer_bytes)?;
                    signature.params.push(cir::AbiParam::new(ty));
                }
                let result_type = self.tree.get(result);
                if !matches!(result_type, mir::Type::Void) {
                    let ty = lower_type(self.tree, result, self.pointer_bytes)?;
                    signature.returns.push(cir::AbiParam::new(ty));
                }
                let sig_ref = builder.import_signature(signature);
                let callee_value = value_map[callee];
                let argument_values: Vec<cir::Value> =
                    arguments.iter().map(|v| value_map[v]).collect();

                // make the call
                let call_inst =
                    builder
                        .ins()
                        .call_indirect(sig_ref, callee_value, &argument_values);

                // get return value if any
                if let Some(dest) = destination {
                    let results = builder.inst_results(call_inst);
                    if !results.is_empty() {
                        value_map.insert(*dest, results[0]);
                    }
                }
            }

            // stack_allocate -> create_sized_stack_slot + stack_addr (alloca equivalent)
            mir::Instruction::StackAlloc {
                destination,
                layout,
            } => {
                let ty = lower_type(self.tree, *layout, self.pointer_bytes)?;
                let size = ty.bytes();

                let slot = builder.create_sized_stack_slot(cir::StackSlotData::new(
                    cir::StackSlotKind::ExplicitSlot,
                    size,
                    0,
                ));

                let address = builder.ins().stack_addr(self.pointer_type(), slot, 0);
                value_map.insert(*destination, address);
            }

            // managed_allocate -> requires GC runtime, not supported
            mir::Instruction::ManagedAlloc { .. } | mir::Instruction::ManagedAllocArray { .. } => {
                return Err(CodegenCraneliftError::unsupported_instruction(
                    "require runtime support",
                    instruction_id.into_any(),
                ));
            }

            // raw_allocate, raw_free -> all require runtime/external support, not implemented
            mir::Instruction::RawAlloc { .. } | mir::Instruction::RawFree { .. } => {
                return Err(CodegenCraneliftError::unsupported_instruction(
                    "require allocator support",
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
    ) -> CodegenCraneliftResult<()> {
        match terminator {
            // return -> return (function exit with optional value)
            mir::Terminator::Return { value } => {
                if let Some(value) = value {
                    let return_value = value_map[value];
                    builder.ins().return_(&[return_value]);
                } else {
                    builder.ins().return_(&[]);
                }
            }

            // jump -> jump (unconditional branch with block args)
            mir::Terminator::Jump { target, arguments } => {
                let target_block = block_map[target];
                let arguments: Vec<cir::BlockArg> = arguments
                    .iter()
                    .map(|v| cir::BlockArg::from(value_map[v]))
                    .collect();
                builder.ins().jump(target_block, &arguments);
            }

            // branch -> brif (conditional branch with block args)
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

                builder.ins().brif(
                    cond_value,
                    then_block,
                    &then_arguments,
                    else_block,
                    &else_arguments,
                );
            }

            // switch -> br_table or brif chain (multi-way branch)
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

            // unreachable -> trap (program abort for impossible paths)
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
    ) -> CodegenCraneliftResult<()> {
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
        // (Cranelift's Switch doesn't support block arguments directly,
        //  so we need to create intermediate blocks for cases with arguments)
        let has_block_arguments =
            !default_arguments.is_empty() || cases.iter().any(|c| !c.arguments.is_empty());
        // fall back to chain of brif for cases with arguments
        if has_block_arguments {
            self.lower_switch_with_arguments(
                switch_value,
                default_block,
                default_arguments,
                cases,
                builder,
                value_map,
                block_map,
            )
        }
        // use Cranelift's Switch for simple cases (no block arguments)
        else {
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
    ) -> CodegenCraneliftResult<()> {
        let default_arguments: Vec<cir::BlockArg> = default_arguments
            .iter()
            .map(|v| cir::BlockArg::from(value_map[v]))
            .collect();

        for case in cases {
            // case condition
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

            // case block
            let next_block = builder.create_block();
            let empty_arguments: Vec<cir::BlockArg> = vec![];
            builder.ins().brif(
                is_match,
                case_block,
                &case_arguments,
                next_block,
                &empty_arguments,
            );
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

    /// Get or declare a global value reference for use in this function.
    fn get_or_declare_global(
        &mut self,
        global_id: mir::LocalNodeId<mir::Global>,
        builder: &mut FunctionBuilder<'_>,
    ) -> CodegenCraneliftResult<cir::GlobalValue> {
        if let Some(&gv) = self.global_map.get(&global_id) {
            return Ok(gv);
        }

        let data_id = self.cl_global_data_ids.get(&global_id).ok_or_else(|| {
            CodegenCraneliftError::Internal {
                message: format!("global {global_id:?} not found in data id map"),
            }
        })?;

        let gv = self.cl_module.declare_data_in_func(*data_id, builder.func);
        self.global_map.insert(global_id, gv);
        Ok(gv)
    }

    /// Lower a constant to Cranelift IR.
    fn lower_constant(
        &mut self,
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

            // string constants should be lowered as globals with GlobalInitializer::Bytes
            mir::Constant::String { .. } => Err(CodegenCraneliftError::unsupported_type(
                "inline string constants not supported; use global with Bytes initializer"
                    .to_string(),
                node_id,
            )),

            // char constant: unicode codepoint as i32
            mir::Constant::Char { value } => {
                Ok(builder.ins().iconst(cir::types::I32, *value as i64))
            }
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
