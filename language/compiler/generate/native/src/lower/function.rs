use std::collections::HashMap;
use std::sync::Arc;

use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::isa::TargetIsa;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Switch};
use cranelift_module::{DataId, FuncId, Module};
use cranelift_object::ObjectModule;
use destack_core::StringPool;
use destack_mir as mir;

use super::layout::{compute_tuple_element_offset, compute_type_layout};
use super::r#type::lower_type;
use crate::{CodegenCraneliftError, CodegenCraneliftResult, trap};

/// Context for lowering a single MIR function to Cranelift IR.
#[allow(dead_code)]
pub(crate) struct FunctionLowerer<'a> {
    /// The MIR tree.
    tree: &'a mir::Tree,
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
    /// Global data id mapping (MIR global to Cranelift DataId).
    cl_global_data_ids: &'a HashMap<mir::LocalNodeId<mir::Global>, DataId>,
    /// Map from MIR function id to Cranelift FuncRef (populated during lowering).
    function_ref_map: HashMap<mir::LocalNodeId<mir::Function>, cir::FuncRef>,
    /// Map from MIR global id to Cranelift GlobalValue (populated during lowering).
    global_map: HashMap<mir::LocalNodeId<mir::Global>, cir::GlobalValue>,
    /// Pointer size in bytes for this target.
    pointer_bytes: u8,
    /// Function environment type when closure.environment is used.
    environment_type: Option<mir::LocalNodeId<mir::Type>>,
    /// Cranelift value for the closure environment parameter.
    environment_param: Option<cir::Value>,
}

#[allow(clippy::too_many_arguments)]
impl<'a> FunctionLowerer<'a> {
    /// Create a new function lowerer.
    pub(crate) fn new(
        tree: &'a mir::Tree,
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
            environment_type: None,
            environment_param: None,
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

        // capture closure environment type before lowering
        self.environment_type =
            self.optional_type_id(self.function.environment, "closure environment type")?;

        // phase 0.5: pre-declare all referenced functions in the current function
        // (must be done before creating the FunctionBuilder)
        self.declare_referenced_functions(target)?;

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

    /// Pre-declare all functions that this function references.
    /// Populates `self.function_ref_map` with the mapping.
    fn declare_referenced_functions(
        &mut self,
        target: &mut cir::Function,
    ) -> CodegenCraneliftResult<()> {
        // scan all blocks for direct callee references
        for &block_id in &self.function.blocks {
            let block = self.tree.get(block_id);

            // check instructions for Call and FunctionAddr
            for &inst_id in &block.instructions {
                let inst = self.tree.get(inst_id);
                // direct call declarations
                if let mir::Instruction::Call { function, .. } = inst {
                    let function = self.function_id(*function, "direct call callee")?;
                    if !self.function_ref_map.contains_key(&function) {
                        self.declare_function_ref(function, target)?;
                    }
                }
                // declare addressable functions
                if let mir::Instruction::FunctionAddr { function, .. }
                | mir::Instruction::ClosureBind { function, .. } = inst
                {
                    let function = self.function_id(*function, "addressable callee")?;
                    if !self.function_ref_map.contains_key(&function) {
                        self.declare_function_ref(function, target)?;
                    }
                }
            }

            // check terminator for direct callees
            let terminator = self.tree.get(block.terminator);

            match terminator {
                mir::Terminator::Call { function, .. }
                | mir::Terminator::TailCall { function, .. } => {
                    let function = self.function_id(*function, "terminator callee")?;
                    if !self.function_ref_map.contains_key(&function) {
                        self.declare_function_ref(function, target)?;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Declare a function reference for use in this function.
    fn declare_function_ref(
        &mut self,
        function: mir::LocalNodeId<mir::Function>,
        target: &mut cir::Function,
    ) -> CodegenCraneliftResult<()> {
        let callee_function_id =
            self.cl_function_ids
                .get(&function)
                .ok_or_else(|| CodegenCraneliftError::Internal {
                    message: format!("unknown function {function:?}"),
                })?;
        let function_ref = self
            .cl_module
            .declare_func_in_func(*callee_function_id, target);
        self.function_ref_map.insert(function, function_ref);
        Ok(())
    }

    /// Create Cranelift stack slots for MIR locals.
    fn create_locals(
        &self,
        builder: &mut FunctionBuilder<'_>,
        local_map: &mut HashMap<mir::LocalNodeId<mir::Local>, cir::StackSlot>,
    ) -> CodegenCraneliftResult<()> {
        for &local_id in &self.function.locals {
            let local = self.tree.get(local_id);
            let ty = lower_type(
                self.tree,
                self.type_id(local.ty, "local type")?,
                self.pointer_bytes,
            )?;
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
        &mut self,
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
            let ty = lower_type(
                self.tree,
                self.type_id(param.ty, "function parameter type")?,
                self.pointer_bytes,
            )?;
            let value = builder.append_block_param(entry_block, ty);
            let parameter = self.value_id(param.value, "function parameter value")?;
            value_map.insert(parameter, value);
        }

        // append the closure environment parameter when present
        if self.environment_type.is_some() {
            let environment_param = builder.append_block_param(entry_block, self.pointer_type());
            self.environment_param = Some(environment_param);
        }

        // now add MIR block parameters for non-entry blocks
        // (entry block parameters are already handled via function parameters above)
        for &block_id in &self.function.blocks {
            // skip entry block, its parameters come from function parameters
            if block_id == entry_block_id {
                continue;
            }

            let mir_block = self.tree.get(block_id);
            let target_block = block_map[&block_id];

            // add block parameters (these are for phi nodes / join points)
            for param in &mir_block.parameters {
                let ty = lower_type(
                    self.tree,
                    self.type_id(param.ty, "block parameter type")?,
                    self.pointer_bytes,
                )?;
                let value = builder.append_block_param(target_block, ty);
                let parameter = self.value_id(param.value, "block parameter value")?;
                value_map.insert(parameter, value);
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
    ) -> CodegenCraneliftResult<()> {
        let mir_block = self.tree.get(block_id);
        let target_block = block_map[&block_id];

        // switch to this block (may already be current for entry)
        if builder.current_block() != Some(target_block) {
            builder.switch_to_block(target_block);
        }

        // lower instructions
        for &instruction_id in &mir_block.instructions {
            self.lower_instruction(instruction_id, builder, value_map, local_map)?;
        }

        // lower terminator
        let terminator = self.tree.get(mir_block.terminator);
        self.lower_terminator(terminator, builder, value_map, block_map)?;

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
    ) -> CodegenCraneliftResult<()> {
        let instruction = self.tree.get(instruction_id);

        // helper for unsupported instructions
        let unsupported = |name: &str| {
            Err(CodegenCraneliftError::unsupported_instruction(
                name.to_string(),
                instruction_id.into_any(),
            ))
        };
        match instruction {
            mir::Instruction::Error => {
                return Err(CodegenCraneliftError::Internal {
                    message: "recovered MIR instruction reached native lowering".into(),
                });
            }
            // const: const or fconst (type-specific immediate load)
            mir::Instruction::Const { destination, value } => {
                // turn null into null pointer
                if matches!(value, mir::Constant::Null) {
                    let null_ptr = builder.ins().iconst(self.pointer_type(), 0);
                    self.insert_lowered_value(
                        value_map,
                        *destination,
                        null_ptr,
                        "const destination",
                    )?;
                } else {
                    let result = self.lower_constant(instruction_id.into_any(), value, builder)?;
                    self.insert_lowered_value(
                        value_map,
                        *destination,
                        result,
                        "const destination",
                    )?;
                }
            }

            // binary: int.add/int.sub/int.mul/etc (arithmetic) or icmp (comparison)
            mir::Instruction::Binary {
                destination,
                operator,
                left,
                right,
            } => {
                let left_value = self.lowered_value(*left, value_map, "binary left operand")?;
                let right_value = self.lowered_value(*right, value_map, "binary right operand")?;
                let result = self.lower_binary_op(*operator, left_value, right_value, builder)?;
                self.insert_lowered_value(value_map, *destination, result, "binary destination")?;
            }

            // unary: ineg/fneg/bnot (type-specific unary operation)
            mir::Instruction::Unary {
                destination,
                operator,
                argument,
            } => {
                let argument_value = self.lowered_value(*argument, value_map, "unary argument")?;
                let result = self.lower_unary_op(*operator, argument_value, builder)?;
                self.insert_lowered_value(value_map, *destination, result, "unary destination")?;
            }

            // cast: sextend/uextend/ireduce/bitcast/etc (type conversion)
            mir::Instruction::Cast {
                destination,
                operator,
                argument,
                to_type,
            } => {
                let argument_value = self.lowered_value(*argument, value_map, "cast argument")?;
                let target_type = lower_type(
                    self.tree,
                    self.type_id(*to_type, "cast destination type")?,
                    self.pointer_bytes,
                )?;
                let result = self.lower_cast(
                    instruction_id.into_any(),
                    *operator,
                    argument_value,
                    target_type,
                    builder,
                )?;
                self.insert_lowered_value(value_map, *destination, result, "cast destination")?;
            }

            // select: conditional value selection
            mir::Instruction::Select {
                destination,
                condition,
                then_value,
                else_value,
            } => {
                let cond = self.lowered_value(*condition, value_map, "select condition")?;
                let t_val = self.lowered_value(*then_value, value_map, "select then value")?;
                let f_val = self.lowered_value(*else_value, value_map, "select else value")?;
                let result = builder.ins().select(cond, t_val, f_val);
                self.insert_lowered_value(value_map, *destination, result, "select destination")?;
            }

            // local_get: stack_load (load from stack slot)
            mir::Instruction::LocalGet { destination, local } => {
                let local = self.local_id(*local, "local get local")?;
                let slot = local_map[&local];
                let local_data = self.tree.get(local);
                let ty = lower_type(
                    self.tree,
                    self.type_id(local_data.ty, "local type")?,
                    self.pointer_bytes,
                )?;
                let result = builder.ins().stack_load(ty, slot, 0);
                self.insert_lowered_value(
                    value_map,
                    *destination,
                    result,
                    "local get destination",
                )?;
            }

            // local_set: stack_store (store to stack slot)
            mir::Instruction::LocalSet { local, value } => {
                let local = self.local_id(*local, "local set local")?;
                let slot = local_map[&local];
                let store_value = self.lowered_value(*value, value_map, "local set value")?;
                builder.ins().stack_store(store_value, slot, 0);
            }

            // local_addr: stack_addr (address of stack slot)
            mir::Instruction::LocalAddr {
                destination, local, ..
            } => {
                let local = self.local_id(*local, "local address local")?;
                let slot = local_map[&local];
                let result = builder.ins().stack_addr(self.pointer_type(), slot, 0);
                self.insert_lowered_value(
                    value_map,
                    *destination,
                    result,
                    "local address destination",
                )?;
            }

            // global_addr: symbol_value (get address of mutable global)
            mir::Instruction::GlobalAddr {
                destination,
                global,
                ..
            } => {
                let global = self.global_id(*global, "global address global")?;
                let global_value = self.get_or_declare_global(global, builder)?;
                let ptr = builder
                    .ins()
                    .global_value(self.pointer_type(), global_value);
                self.insert_lowered_value(
                    value_map,
                    *destination,
                    ptr,
                    "global address destination",
                )?;
            }

            // function_addr: get a function pointer for indirect calls
            mir::Instruction::FunctionAddr {
                destination,
                function,
            } => {
                let function = self.function_id(*function, "function address callee")?;
                let function_ref = self.function_ref_map.get(&function).ok_or_else(|| {
                    CodegenCraneliftError::Internal {
                        message: format!("function {function:?} not declared"),
                    }
                })?;
                let address = builder.ins().func_addr(self.pointer_type(), *function_ref);
                self.insert_lowered_value(
                    value_map,
                    *destination,
                    address,
                    "function address destination",
                )?;
            }
            mir::Instruction::ClosureBind {
                destination,
                function,
                environment,
            } => {
                let destination = self.value_id(*destination, "closure.bind destination")?;
                let destination_type =
                    self.value_type_or_error(destination, instruction_id.into_any())?;
                let mir::Type::Closure { .. } = self.tree.get(destination_type) else {
                    return Err(CodegenCraneliftError::Internal {
                        message: "closure.bind result must be a closure value".into(),
                    });
                };

                let function = self.function_id(*function, "closure.bind callee")?;
                let function_ref = self.function_ref_map.get(&function).ok_or_else(|| {
                    CodegenCraneliftError::Internal {
                        message: format!("function {function:?} not declared"),
                    }
                })?;
                let code_value = builder.ins().func_addr(self.pointer_type(), *function_ref);
                let environment_value =
                    self.lowered_value(*environment, value_map, "closure.bind environment")?;
                let environment_value =
                    if builder.func.dfg.value_type(environment_value) == self.pointer_type() {
                        environment_value
                    } else {
                        builder.ins().bitcast(
                            self.pointer_type(),
                            cir::MemFlags::new(),
                            environment_value,
                        )
                    };
                let function_node = function.into_any();

                // allocate the closure aggregate and store semantic components
                let layout = compute_type_layout(self.tree, destination_type, self.pointer_bytes)?;
                let align_shift = layout.alignment.trailing_zeros() as u8;
                let slot = builder.create_sized_stack_slot(cir::StackSlotData::new(
                    cir::StackSlotKind::ExplicitSlot,
                    layout.size,
                    align_shift,
                ));
                let slot_addr = builder.ins().stack_addr(self.pointer_type(), slot, 0);

                let (function_offset, _function_type) =
                    self.aggregate_field_offset_and_type(destination_type, 0, function_node)?;
                let (environment_offset, _environment_type) =
                    self.aggregate_field_offset_and_type(destination_type, 1, function_node)?;

                builder.ins().store(
                    cir::MemFlags::new(),
                    code_value,
                    slot_addr,
                    function_offset as i32,
                );
                builder.ins().store(
                    cir::MemFlags::new(),
                    environment_value,
                    slot_addr,
                    environment_offset as i32,
                );
                value_map.insert(destination, slot_addr);
            }

            // closure.environment: load the hidden environment parameter
            mir::Instruction::ClosureEnvironment { destination } => {
                let environment_param =
                    self.environment_param
                        .ok_or_else(|| CodegenCraneliftError::Internal {
                            message: "closure.environment used without environment parameter"
                                .to_string(),
                        })?;
                let destination = self.value_id(*destination, "closure environment destination")?;
                let destination_type =
                    self.value_type_or_error(destination, instruction_id.into_any())?;
                let destination_ty = lower_type(self.tree, destination_type, self.pointer_bytes)?;
                let environment_value = if destination_ty == self.pointer_type() {
                    environment_param
                } else {
                    builder
                        .ins()
                        .bitcast(destination_ty, cir::MemFlags::new(), environment_param)
                };
                value_map.insert(destination, environment_value);
            }

            // load: memory read through pointer
            mir::Instruction::Load {
                destination,
                pointer,
                result_type,
            } => {
                let ptr_value = self.lowered_value(*pointer, value_map, "load pointer")?;
                let loaded_type = lower_type(
                    self.tree,
                    self.type_id(*result_type, "load result type")?,
                    self.pointer_bytes,
                )?;

                let result = builder
                    .ins()
                    .load(loaded_type, cir::MemFlags::new(), ptr_value, 0);
                self.insert_lowered_value(value_map, *destination, result, "load destination")?;
            }

            // store: memory write through pointer
            mir::Instruction::Store { pointer, value } => {
                let ptr_value = self.lowered_value(*pointer, value_map, "store pointer")?;
                let store_value = self.lowered_value(*value, value_map, "store value")?;
                builder
                    .ins()
                    .store(cir::MemFlags::new(), store_value, ptr_value, 0);
            }

            // pins need runtime safepoint support before native lowering
            mir::Instruction::Pin { value, .. } | mir::Instruction::Unpin { value } => {
                let _ = self.lowered_value(*value, value_map, "pin value")?;
                return Err(CodegenCraneliftError::unsupported_instruction(
                    "pin",
                    instruction_id.into_any(),
                ));
            }

            // drop markers must be elaborated before codegen
            mir::Instruction::Drop { .. } => {
                return Err(CodegenCraneliftError::unsupported_instruction(
                    "drop",
                    instruction_id.into_any(),
                ));
            }

            // assume: no op for codegen (optimizer handled it)
            mir::Instruction::Assume { .. } => {}

            // extract_field: load at computed offset (struct/tuple field read)
            mir::Instruction::FieldGet {
                destination,
                aggregate,
                index,
            } => {
                let destination = self.value_id(*destination, "field get destination")?;
                let aggregate = self.value_id(*aggregate, "field get aggregate")?;

                // aggregate type
                let aggregate_type_id =
                    self.value_type_or_error(aggregate, instruction_id.into_any())?;
                // field offset and type
                let (field_offset, field_type_id) = self.aggregate_field_offset_and_type(
                    aggregate_type_id,
                    *index,
                    instruction_id.into_any(),
                )?;

                // load from aggregate_ptr + offset
                let field_type = lower_type(self.tree, field_type_id, self.pointer_bytes)?;
                let aggregate_ptr = value_map[&aggregate];
                let result = builder.ins().load(
                    field_type,
                    cir::MemFlags::new(),
                    aggregate_ptr,
                    field_offset as i32,
                );
                value_map.insert(destination, result);
            }

            // field_addr: pointer to field
            mir::Instruction::FieldAddr {
                destination,
                aggregate,
                index,
                ..
            } => {
                let destination = self.value_id(*destination, "field address destination")?;
                let aggregate = self.value_id(*aggregate, "field address aggregate")?;

                // aggregate type
                let aggregate_type_id =
                    self.value_type_or_error(aggregate, instruction_id.into_any())?;
                let aggregate_type = self.tree.get(aggregate_type_id);
                let aggregate_layout_type_id = match aggregate_type {
                    mir::Type::Reference { pointee, .. } => {
                        self.type_id(*pointee, "field address pointee type")?
                    }
                    _ => aggregate_type_id,
                };

                // field offset and type
                let (field_offset, _field_type) = self.aggregate_field_offset_and_type(
                    aggregate_layout_type_id,
                    *index,
                    instruction_id.into_any(),
                )?;

                // pointer arithmetic on the aggregate pointer
                let aggregate_ptr = value_map[&aggregate];
                let field_ptr = if field_offset == 0 {
                    aggregate_ptr
                } else {
                    builder.ins().iadd_imm(aggregate_ptr, field_offset as i64)
                };
                value_map.insert(destination, field_ptr);
            }

            // insert_field: store at computed offset (struct/tuple field write)
            mir::Instruction::FieldSet {
                destination,
                aggregate,
                index,
                value,
            } => {
                let destination = self.value_id(*destination, "field set destination")?;
                let aggregate = self.value_id(*aggregate, "field set aggregate")?;
                let value = self.value_id(*value, "field set value")?;

                // aggregate type
                let aggregate_type_id =
                    self.value_type_or_error(aggregate, instruction_id.into_any())?;
                let aggregate_type = self.tree.get(aggregate_type_id);
                let aggregate_layout_type_id = match aggregate_type {
                    mir::Type::Reference { pointee, .. } => {
                        self.type_id(*pointee, "field set pointee type")?
                    }
                    _ => aggregate_type_id,
                };

                // field
                let (field_offset, _field_type) = self.aggregate_field_offset_and_type(
                    aggregate_layout_type_id,
                    *index,
                    instruction_id.into_any(),
                )?;

                // store to aggregate_ptr + offset
                let aggregate_ptr = value_map[&aggregate];
                let store_value = value_map[&value];
                builder.ins().store(
                    cir::MemFlags::new(),
                    store_value,
                    aggregate_ptr,
                    field_offset as i32,
                );
                // the result is the aggregate pointer (in-place mutation)
                value_map.insert(destination, aggregate_ptr);
            }

            // extract_element: load at ptr + index * elem_size (array element read)
            mir::Instruction::ElementGet {
                destination,
                array,
                index,
            } => {
                let destination = self.value_id(*destination, "element get destination")?;
                let array = self.value_id(*array, "element get array")?;

                // array type
                let array_type_id = self.value_type_or_error(array, instruction_id.into_any())?;
                let array_type = self.tree.get(array_type_id);

                // element type
                let element_type_id = match array_type {
                    mir::Type::Array { element, .. } => {
                        self.type_id(*element, "array element type")?
                    }
                    _ => {
                        return Err(CodegenCraneliftError::Internal {
                            message: "ElementGet on non-array type".into(),
                        });
                    }
                };
                let element_type = lower_type(self.tree, element_type_id, self.pointer_bytes)?;
                let element_size = element_type.bytes() as i64;
                let element_offset = element_size * i64::from(*index);

                // array pointer + index * element_size
                let array_ptr = value_map[&array];
                let result = builder.ins().load(
                    element_type,
                    cir::MemFlags::new(),
                    array_ptr,
                    element_offset as i32,
                );
                value_map.insert(destination, result);
            }

            // element_addr: pointer to element
            mir::Instruction::ElementAddr {
                destination,
                array,
                index,
                ..
            } => {
                let destination = self.value_id(*destination, "element address destination")?;
                let array = self.value_id(*array, "element address array")?;
                let index = self.value_id(*index, "element address index")?;

                // array type
                let array_type_id = self.value_type_or_error(array, instruction_id.into_any())?;
                let array_type = self.tree.get(array_type_id);
                let array_layout = match array_type {
                    mir::Type::Reference { pointee, .. } => self
                        .tree
                        .get(self.type_id(*pointee, "element address pointee type")?),
                    _ => array_type,
                };

                // element type
                let element_type_id = match array_layout {
                    mir::Type::Array { element, .. } => {
                        self.type_id(*element, "array element type")?
                    }
                    _ => {
                        return Err(CodegenCraneliftError::Internal {
                            message: "ElementAddr on non-array type".into(),
                        });
                    }
                };
                let element_type = lower_type(self.tree, element_type_id, self.pointer_bytes)?;
                let element_size = element_type.bytes() as i64;

                // array pointer + index * element_size
                let array_ptr = value_map[&array];
                let idx_value = value_map[&index];
                let offset = builder.ins().imul_imm(idx_value, element_size);
                let element_ptr = builder.ins().iadd(array_ptr, offset);
                value_map.insert(destination, element_ptr);
            }

            // insert_element: store at ptr + index * elem_size (array element write)
            mir::Instruction::ElementSet {
                destination,
                array,
                index,
                value,
            } => {
                let destination = self.value_id(*destination, "element set destination")?;
                let array = self.value_id(*array, "element set array")?;
                let value = self.value_id(*value, "element set value")?;

                // array type
                let array_type_id = self.value_type_or_error(array, instruction_id.into_any())?;
                let array_type = self.tree.get(array_type_id);

                // element type
                let element_type_id = match array_type {
                    mir::Type::Array { element, .. } => {
                        self.type_id(*element, "array element type")?
                    }
                    _ => {
                        return Err(CodegenCraneliftError::Internal {
                            message: "ElementSet on non-array type".into(),
                        });
                    }
                };
                let element_type = lower_type(self.tree, element_type_id, self.pointer_bytes)?;
                let element_size = element_type.bytes() as i64;
                let element_offset = element_size * i64::from(*index);

                // array pointer + index * element_size
                let array_ptr = value_map[&array];
                let store_value = value_map[&value];
                builder.ins().store(
                    cir::MemFlags::new(),
                    store_value,
                    array_ptr,
                    element_offset as i32,
                );
                // the result is the array pointer itself
                value_map.insert(destination, array_ptr);
            }

            // call: direct function call via pre-declared FuncRef
            mir::Instruction::Call {
                destination,
                function,
                call,
                ..
            } => {
                let function = self.function_id(*function, "direct call callee")?;
                let callee = self.tree.get(function);
                if callee.environment.is_some() {
                    return Err(CodegenCraneliftError::Internal {
                        message: "direct call cannot target a function with an environment".into(),
                    });
                }
                let function_ref = self.function_ref_map.get(&function).ok_or_else(|| {
                    CodegenCraneliftError::Internal {
                        message: format!("function {function:?} not declared"),
                    }
                })?;

                // gather argument values
                let args = self.tree.get_arguments(call.arguments);
                let argument_values: Vec<cir::Value> = args
                    .iter()
                    .map(|value| {
                        let value = self.value_id(*value, "direct call argument")?;
                        Ok(value_map[&value])
                    })
                    .collect::<CodegenCraneliftResult<_>>()?;

                // make the call
                let call_instruction = builder.ins().call(*function_ref, &argument_values);

                // get return value if any
                if let Some(dest) =
                    self.optional_value_id(*destination, "direct call destination")?
                {
                    let results = builder.inst_results(call_instruction);
                    if !results.is_empty() {
                        value_map.insert(dest, results[0]);
                    }
                }
            }

            // call_indirect: indirect call through function pointer
            mir::Instruction::CallIndirect {
                destination,
                callee,
                call,
                ..
            } => {
                let callee = self.value_id(*callee, "indirect call callee")?;
                let sig_ref = self.build_indirect_call_signature(
                    self.type_id(call.signature, "indirect call signature")?,
                    builder,
                    "indirect call",
                )?;
                let (callee_value, environment_value) = self.lower_indirect_callee(
                    callee,
                    self.type_id(call.signature, "indirect call signature")?,
                    value_map,
                    builder,
                )?;
                let args = self.tree.get_arguments(call.arguments);
                let mut argument_values: Vec<cir::Value> = args
                    .iter()
                    .map(|value| {
                        let value = self.value_id(*value, "indirect call argument")?;
                        Ok(value_map[&value])
                    })
                    .collect::<CodegenCraneliftResult<_>>()?;
                if let Some(environment_value) = environment_value {
                    argument_values.push(environment_value);
                }

                // make the call
                let call_inst =
                    builder
                        .ins()
                        .call_indirect(sig_ref, callee_value, &argument_values);

                // get return value if any
                if let Some(dest) =
                    self.optional_value_id(*destination, "indirect call destination")?
                {
                    let results = builder.inst_results(call_inst);
                    if !results.is_empty() {
                        value_map.insert(dest, results[0]);
                    }
                }
            }
            mir::Instruction::CallVirtual { .. } | mir::Instruction::CallDynamic { .. } => {
                return Err(CodegenCraneliftError::unsupported_instruction(
                    "class calls are not supported in cranelift yet",
                    instruction_id.into_any(),
                ));
            }

            // allocate frame storage with cranelift stack slots
            mir::Instruction::FrameAllocZeroed {
                destination,
                layout,
                ..
            }
            | mir::Instruction::FrameAllocUninit {
                destination,
                layout,
                ..
            } => {
                let destination = self.value_id(*destination, "frame alloc destination")?;
                let layout = self.type_id(*layout, "frame alloc layout")?;
                let ty = lower_type(self.tree, layout, self.pointer_bytes)?;
                let size = ty.bytes();

                let slot = builder.create_sized_stack_slot(cir::StackSlotData::new(
                    cir::StackSlotKind::ExplicitSlot,
                    size,
                    0,
                ));

                let address = builder.ins().stack_addr(self.pointer_type(), slot, 0);
                value_map.insert(destination, address);
            }

            // managed allocation requires runtime support
            mir::Instruction::NewZeroed { .. }
            | mir::Instruction::NewUninit { .. }
            | mir::Instruction::NewComplete { .. }
            | mir::Instruction::NewSliceZeroed { .. }
            | mir::Instruction::NewSliceUninit { .. } => {
                return Err(CodegenCraneliftError::unsupported_instruction(
                    "require runtime support",
                    instruction_id.into_any(),
                ));
            }

            // unique free requires runtime support
            mir::Instruction::Free { .. } => {
                return Err(CodegenCraneliftError::unsupported_instruction(
                    "require runtime support",
                    instruction_id.into_any(),
                ));
            }

            // struct/callable: allocate stack slot and store each field at its offset
            mir::Instruction::Struct {
                destination,
                ty,
                fields,
            } => {
                let destination = self.value_id(*destination, "struct destination")?;
                let ty = self.type_id(*ty, "struct type")?;
                let field_values = self.tree.get_arguments(*fields);
                let field_count = match self.tree.get(ty) {
                    mir::Type::Struct { fields, .. } => fields.len(),
                    mir::Type::Closure { .. } => 2,
                    _ => {
                        return Err(CodegenCraneliftError::Internal {
                            message: "Struct instruction with non-aggregate type".into(),
                        });
                    }
                };

                // compute layout and allocate stack slot
                let layout = compute_type_layout(self.tree, ty, self.pointer_bytes)?;
                let align_shift = layout.alignment.trailing_zeros() as u8;
                let slot = builder.create_sized_stack_slot(cir::StackSlotData::new(
                    cir::StackSlotKind::ExplicitSlot,
                    layout.size,
                    align_shift,
                ));
                let slot_addr = builder.ins().stack_addr(self.pointer_type(), slot, 0);

                // store each field at its offset
                if field_values.len() != field_count {
                    return Err(CodegenCraneliftError::Internal {
                        message: "Struct instruction field count mismatch".into(),
                    });
                }

                for (index, field_value) in field_values.iter().enumerate() {
                    let (offset, _field_type) = self.aggregate_field_offset_and_type(
                        ty,
                        index as u32,
                        instruction_id.into_any(),
                    )?;
                    let value =
                        self.lowered_value(*field_value, value_map, "struct field value")?;
                    builder
                        .ins()
                        .store(cir::MemFlags::new(), value, slot_addr, offset as i32);
                }

                value_map.insert(destination, slot_addr);
            }

            // tuple: allocate stack slot and store each element at computed offset
            mir::Instruction::Tuple {
                destination,
                ty,
                elements,
            } => {
                let destination = self.value_id(*destination, "tuple destination")?;
                let ty = self.type_id(*ty, "tuple type")?;
                let tuple_type = self.tree.get(ty);

                // get element type definitions
                let element_types = match tuple_type {
                    mir::Type::Tuple { elements, .. } => elements
                        .iter()
                        .map(|element| self.type_id(*element, "tuple element type"))
                        .collect::<CodegenCraneliftResult<Vec<_>>>()?,
                    _ => {
                        return Err(CodegenCraneliftError::Internal {
                            message: "Tuple instruction with non-tuple type".into(),
                        });
                    }
                };
                let element_values = self.tree.get_arguments(*elements);

                // compute layout and allocate stack slot
                let layout = compute_type_layout(self.tree, ty, self.pointer_bytes)?;
                let align_shift = layout.alignment.trailing_zeros() as u8;
                let slot = builder.create_sized_stack_slot(cir::StackSlotData::new(
                    cir::StackSlotKind::ExplicitSlot,
                    layout.size,
                    align_shift,
                ));
                let slot_addr = builder.ins().stack_addr(self.pointer_type(), slot, 0);

                // store each element at its computed offset
                for (i, element_value) in element_values.iter().enumerate() {
                    let offset = compute_tuple_element_offset(
                        self.tree,
                        &element_types,
                        i as u32,
                        self.pointer_bytes,
                    )?;
                    let value =
                        self.lowered_value(*element_value, value_map, "tuple element value")?;
                    builder
                        .ins()
                        .store(cir::MemFlags::new(), value, slot_addr, offset as i32);
                }

                value_map.insert(destination, slot_addr);
            }

            // array: allocate stack slot and store each element at index * element_size
            mir::Instruction::Array {
                destination,
                ty,
                elements,
            } => {
                let destination = self.value_id(*destination, "array destination")?;
                let ty = self.type_id(*ty, "array type")?;
                let array_type = self.tree.get(ty);

                // get element type
                let element_type_id = match array_type {
                    mir::Type::Array { element, .. } => {
                        self.type_id(*element, "array element type")?
                    }
                    _ => {
                        return Err(CodegenCraneliftError::Internal {
                            message: "Array instruction with non-array type".into(),
                        });
                    }
                };
                let element_values = self.tree.get_arguments(*elements);

                // compute layout and allocate stack slot
                let layout = compute_type_layout(self.tree, ty, self.pointer_bytes)?;
                let align_shift = layout.alignment.trailing_zeros() as u8;
                let slot = builder.create_sized_stack_slot(cir::StackSlotData::new(
                    cir::StackSlotKind::ExplicitSlot,
                    layout.size,
                    align_shift,
                ));
                let slot_addr = builder.ins().stack_addr(self.pointer_type(), slot, 0);

                // compute element size
                let element_type = lower_type(self.tree, element_type_id, self.pointer_bytes)?;
                let element_size = element_type.bytes();

                // store each element at index * element_size
                for (i, element_value) in element_values.iter().enumerate() {
                    let offset = (i as u32) * element_size;
                    let value =
                        self.lowered_value(*element_value, value_map, "array element value")?;
                    builder
                        .ins()
                        .store(cir::MemFlags::new(), value, slot_addr, offset as i32);
                }

                value_map.insert(destination, slot_addr);
            }

            // slice views require descriptor lowering
            mir::Instruction::Slice { .. } => return unsupported("slice"),

            // vector ops: lower only after explicit lowering (#Incomplete)
            mir::Instruction::VectorSplat { .. } => return unsupported("vector.splat"),
            mir::Instruction::VectorExtract { .. } => return unsupported("vector.extract"),
            mir::Instruction::VectorInsert { .. } => return unsupported("vector.insert"),
            mir::Instruction::VectorShuffle { .. } => return unsupported("vector.shuffle"),
            mir::Instruction::VectorSelect { .. } => return unsupported("vector.select"),
            mir::Instruction::VectorReduce { .. } => return unsupported("vector.reduce"),
            mir::Instruction::VectorCompare { .. } => return unsupported("vector.compare"),
            mir::Instruction::VectorConvert { .. } => return unsupported("vector.convert"),

            // tensor ops need explicit lowering
            mir::Instruction::TensorLoad { .. } => return unsupported("tensor.load"),
            mir::Instruction::TensorSplat { .. } => return unsupported("tensor.splat"),
            mir::Instruction::TensorExtract { .. } => return unsupported("tensor.extract"),
            mir::Instruction::TensorStore { .. } => return unsupported("tensor.store"),
            mir::Instruction::TensorFill { .. } => return unsupported("tensor.fill"),
            mir::Instruction::TensorCopy { .. } => return unsupported("tensor.copy"),
            mir::Instruction::TensorReshape { .. } => return unsupported("tensor.reshape"),
            mir::Instruction::TensorBroadcast { .. } => return unsupported("tensor.broadcast"),
            mir::Instruction::TensorTranspose { .. } => return unsupported("tensor.transpose"),
            mir::Instruction::TensorCast { .. } => return unsupported("tensor.cast"),
            mir::Instruction::TensorView { .. } => return unsupported("tensor.view"),
            mir::Instruction::TensorSlice { .. } => return unsupported("tensor.slice"),
            mir::Instruction::TensorPad { .. } => return unsupported("tensor.pad"),
            mir::Instruction::TensorConcat { .. } => return unsupported("tensor.concat"),
            mir::Instruction::TensorReduce { .. } => return unsupported("tensor.reduce"),
            mir::Instruction::TensorIndexReduce { .. } => {
                return unsupported("tensor.indexReduce");
            }
            mir::Instruction::TensorDot { .. } => return unsupported("tensor.dot"),
            mir::Instruction::TensorConvolution { .. } => return unsupported("tensor.convolution"),
            mir::Instruction::TensorGather { .. } => return unsupported("tensor.gather"),
            mir::Instruction::TensorScatter { .. } => return unsupported("tensor.scatter"),
            mir::Instruction::TensorCompare { .. } => return unsupported("tensor.compare"),
            mir::Instruction::TensorSelect { .. } => return unsupported("tensor.select"),
            mir::Instruction::TensorConvert { .. } => return unsupported("tensor.convert"),
            mir::Instruction::AtomicLoad { .. } => return unsupported("atomic.load"),
            mir::Instruction::AtomicStore { .. } => return unsupported("atomic.store"),
            mir::Instruction::AtomicCompareExchange { .. } => return unsupported("atomic.cas"),
            mir::Instruction::AtomicRmw { operator, .. } => {
                return unsupported(&format!("atomic.rmw.{}", operator.to_str()));
            }
            mir::Instruction::AtomicFence { .. } => return unsupported("atomic.fence"),
            mir::Instruction::BarrierWrite { .. } => return unsupported("barrier.write"),

            // intrinsic: depends on the specific intrinsic
            mir::Instruction::Intrinsic { intrinsic, .. } => {
                return unsupported(&format!("intrinsic.{}", intrinsic.to_str()));
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
            mir::Terminator::Error => {
                return Err(CodegenCraneliftError::Internal {
                    message: "recovered MIR terminator reached native lowering".into(),
                });
            }
            // return: function exit with optional value
            mir::Terminator::Return { value } => {
                if let Some(value) = self.optional_value_id(*value, "return value")? {
                    let return_value = value_map[&value];
                    builder.ins().return_(&[return_value]);
                } else {
                    builder.ins().return_(&[]);
                }
            }

            // jump: unconditional branch with block args
            mir::Terminator::Jump { target } => {
                let target_id = self.block_id(target.block, "jump target")?;
                let target_block = block_map[&target_id];
                let arguments: Vec<cir::BlockArg> = target
                    .arguments
                    .iter()
                    .map(|value| {
                        let value = self.value_id(*value, "jump argument")?;
                        Ok(cir::BlockArg::from(value_map[&value]))
                    })
                    .collect::<CodegenCraneliftResult<_>>()?;
                builder.ins().jump(target_block, &arguments);
            }

            // branch: brif (conditional branch with block args)
            mir::Terminator::Branch {
                condition,
                then_target,
                else_target,
            } => {
                let condition = self.value_id(*condition, "branch condition")?;
                let cond_value = value_map[&condition];
                let then_block_id = self.block_id(then_target.block, "branch then target")?;
                let else_block_id = self.block_id(else_target.block, "branch else target")?;
                let then_block = block_map[&then_block_id];
                let else_block = block_map[&else_block_id];
                let then_arguments: Vec<cir::BlockArg> = then_target
                    .arguments
                    .iter()
                    .map(|value| {
                        let value = self.value_id(*value, "branch then argument")?;
                        Ok(cir::BlockArg::from(value_map[&value]))
                    })
                    .collect::<CodegenCraneliftResult<_>>()?;
                let else_arguments: Vec<cir::BlockArg> = else_target
                    .arguments
                    .iter()
                    .map(|value| {
                        let value = self.value_id(*value, "branch else argument")?;
                        Ok(cir::BlockArg::from(value_map[&value]))
                    })
                    .collect::<CodegenCraneliftResult<_>>()?;

                builder.ins().brif(
                    cond_value,
                    then_block,
                    &then_arguments,
                    else_block,
                    &else_arguments,
                );
            }

            // check: explicit condition lowering is not implemented here yet
            mir::Terminator::Check {
                success: _,
                failure: _,
                ..
            } => {
                return Err(CodegenCraneliftError::Internal {
                    message: "check terminators are not supported in native codegen yet".into(),
                });
            }

            mir::Terminator::NewZeroedTry { .. }
            | mir::Terminator::NewUninitTry { .. }
            | mir::Terminator::NewSliceZeroedTry { .. }
            | mir::Terminator::NewSliceUninitTry { .. } => {
                return Err(CodegenCraneliftError::Internal {
                    message: "fallible allocation terminators require runtime allocation branches"
                        .into(),
                });
            }

            // switch: br_table or brif chain (multi-way branch)
            mir::Terminator::Switch {
                value,
                default,
                cases,
            } => {
                let value = self.value_id(*value, "switch value")?;
                let default_block_id = self.block_id(default.block, "switch default")?;
                self.lower_switch(
                    value_map[&value],
                    block_map[&default_block_id],
                    &default.arguments,
                    cases,
                    builder,
                    value_map,
                    block_map,
                )?;
            }

            // unreachable: trap (program abort for impossible paths)
            mir::Terminator::Unreachable => {
                builder.ins().trap(trap::UNREACHABLE);
            }

            // yield: coroutine suspension
            mir::Terminator::Yield { .. } => {
                // #Incomplete: implement cranelift coroutine support (lower in MIR?)
                builder.ins().trap(trap::UNREACHABLE);
            }

            // call terminators: explicit call CFG is not lowered yet
            mir::Terminator::Call { .. }
            | mir::Terminator::CallIndirect { .. }
            | mir::Terminator::CallVirtual { .. }
            | mir::Terminator::CallDynamic { .. } => {
                return Err(CodegenCraneliftError::Internal {
                    message: "call terminators are not supported in native codegen yet".into(),
                });
            }

            // panic still needs runtime support
            mir::Terminator::Panic { .. } | mir::Terminator::ResumePanic => {
                return Err(CodegenCraneliftError::Internal {
                    message: "panic unwinding requires native runtime lowering".into(),
                });
            }

            // abort trap lowers to a backend trap
            mir::Terminator::Trap { kind, .. } => match kind {
                mir::TrapKind::Abort => {
                    builder.ins().trap(trap::UNREACHABLE);
                }
            },

            // tail call: return_call (direct tail call)
            mir::Terminator::TailCall { function, call } => {
                let function = self.function_id(*function, "tail call callee")?;
                let callee = self.tree.get(function);
                if callee.environment.is_some() {
                    return Err(CodegenCraneliftError::Internal {
                        message: "direct call cannot target a function with an environment".into(),
                    });
                }
                let function_ref = self.function_ref_map.get(&function).ok_or_else(|| {
                    CodegenCraneliftError::Internal {
                        message: format!("function {function:?} not declared"),
                    }
                })?;

                // gather argument values
                let argument_values: Vec<cir::Value> = call
                    .arguments
                    .iter()
                    .map(|value| {
                        let value = self.value_id(*value, "tail call argument")?;
                        Ok(value_map[&value])
                    })
                    .collect::<CodegenCraneliftResult<_>>()?;

                // emit return_call
                builder.ins().return_call(*function_ref, &argument_values);
            }

            // tail call indirect: return_call_indirect (indirect tail call)
            mir::Terminator::TailCallIndirect { callee, call } => {
                let callee = self.value_id(*callee, "tail indirect callee")?;
                let signature = self.type_id(call.signature, "tail indirect signature")?;
                let sig_ref =
                    self.build_indirect_call_signature(signature, builder, "tail call")?;
                let (callee_value, environment_value) =
                    self.lower_indirect_callee(callee, signature, value_map, builder)?;
                let mut argument_values: Vec<cir::Value> = call
                    .arguments
                    .iter()
                    .map(|value| {
                        let value = self.value_id(*value, "tail indirect argument")?;
                        Ok(value_map[&value])
                    })
                    .collect::<CodegenCraneliftResult<_>>()?;
                if let Some(environment_value) = environment_value {
                    argument_values.push(environment_value);
                }
                builder
                    .ins()
                    .return_call_indirect(sig_ref, callee_value, &argument_values);
            }
            mir::Terminator::TailCallVirtual { .. } | mir::Terminator::TailCallDynamic { .. } => {
                return Err(CodegenCraneliftError::Internal {
                    message: "virtual tail calls are not supported in cranelift yet".into(),
                });
            }
        }

        Ok(())
    }

    /// Lower a switch terminator using Cranelift's Switch helper.
    fn lower_switch(
        &self,
        switch_value: cir::Value,
        default_block: cir::Block,
        default_arguments: &[mir::ValueReference],
        cases: &[mir::SwitchCase],
        builder: &mut FunctionBuilder<'_>,
        value_map: &HashMap<mir::Value, cir::Value>,
        block_map: &HashMap<mir::LocalNodeId<mir::Block>, cir::Block>,
    ) -> CodegenCraneliftResult<()> {
        // if no cases, just jump to default
        if cases.is_empty() {
            let default_arguments: Vec<cir::BlockArg> = default_arguments
                .iter()
                .map(|value| {
                    let value = self.value_id(*value, "switch default argument")?;
                    Ok(cir::BlockArg::from(value_map[&value]))
                })
                .collect::<CodegenCraneliftResult<_>>()?;
            builder.ins().jump(default_block, &default_arguments);
            return Ok(());
        }

        // check if any cases have block arguments
        // (Cranelift's Switch doesn't support block arguments directly,
        //  so we need to create intermediate blocks for cases with arguments)
        let has_block_arguments =
            !default_arguments.is_empty() || cases.iter().any(|c| !c.target.arguments.is_empty());
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
                let case_block_id = self.block_id(case.target.block, "switch case target")?;
                let case_block = block_map[&case_block_id];
                let case_value = self.integer_value(case.value, "switch case value")?;
                switch.set_entry(case_value as u128, case_block);
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
        default_arguments: &[mir::ValueReference],
        cases: &[mir::SwitchCase],
        builder: &mut FunctionBuilder<'_>,
        value_map: &HashMap<mir::Value, cir::Value>,
        block_map: &HashMap<mir::LocalNodeId<mir::Block>, cir::Block>,
    ) -> CodegenCraneliftResult<()> {
        let default_arguments: Vec<cir::BlockArg> = default_arguments
            .iter()
            .map(|value| {
                let value = self.value_id(*value, "switch default argument")?;
                Ok(cir::BlockArg::from(value_map[&value]))
            })
            .collect::<CodegenCraneliftResult<_>>()?;

        for case in cases {
            // case condition
            let case_value = self.integer_value(case.value, "switch case value")?;
            let case_value =
                i64::try_from(case_value).map_err(|_| CodegenCraneliftError::Internal {
                    message: "wide switch case with block arguments".to_string(),
                })?;
            let case_const = builder.ins().iconst(self.pointer_type(), case_value);
            let is_match =
                builder
                    .ins()
                    .icmp(cir::condcodes::IntCC::Equal, switch_value, case_const);
            let case_block_id = self.block_id(case.target.block, "switch case target")?;
            let case_block = block_map[&case_block_id];
            let case_arguments: Vec<cir::BlockArg> = case
                .target
                .arguments
                .iter()
                .map(|value| {
                    let value = self.value_id(*value, "switch case argument")?;
                    Ok(cir::BlockArg::from(value_map[&value]))
                })
                .collect::<CodegenCraneliftResult<_>>()?;

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
            // NOTE #Broken: cranelift codegen only supports 32-bit and 64-bit pointers
            _ => panic!("unsupported pointer size: {} bytes", self.pointer_bytes),
        }
    }

    /// Build a Cranelift signature for an indirect call from a function type.
    fn build_indirect_call_signature(
        &self,
        signature: mir::LocalNodeId<mir::Type>,
        builder: &mut FunctionBuilder<'_>,
        error_context: &str,
    ) -> CodegenCraneliftResult<cir::SigRef> {
        // closure abi
        let (signature, has_environment) = match self.tree.get(signature) {
            mir::Type::FunctionSignature { .. } => (signature, false),
            mir::Type::FunctionPointer { signature } => (
                self.type_id(*signature, "function pointer signature")?,
                false,
            ),
            mir::Type::Closure { signature, .. } => {
                (self.type_id(*signature, "closure signature")?, true)
            }
            _ => {
                return Err(CodegenCraneliftError::Internal {
                    message: format!("{error_context} signature is not a function type"),
                });
            }
        };

        // extract function pointer params and result
        let mir::Type::FunctionSignature {
            parameters, result, ..
        } = self.tree.get(signature)
        else {
            return Err(CodegenCraneliftError::Internal {
                message: format!("{error_context} signature is not a function signature"),
            });
        };

        // build signature
        let call_conv = self.isa.default_call_conv();
        let mut signature = cir::Signature::new(call_conv);
        for param_ty in parameters {
            let ty = lower_type(
                self.tree,
                self.type_id(*param_ty, "indirect call parameter type")?,
                self.pointer_bytes,
            )?;
            signature.params.push(cir::AbiParam::new(ty));
        }
        if has_environment {
            signature
                .params
                .push(cir::AbiParam::new(self.pointer_type()));
        }
        let result = self.type_id(*result, "indirect call result type")?;
        let result_type = self.tree.get(result);
        if !matches!(result_type, mir::Type::Void) {
            let ty = lower_type(self.tree, result, self.pointer_bytes)?;
            signature.returns.push(cir::AbiParam::new(ty));
        }

        Ok(builder.import_signature(signature))
    }

    /// Lower one closure value into code and optional environment operands.
    fn lower_indirect_callee(
        &self,
        callee: mir::Value,
        signature: mir::LocalNodeId<mir::Type>,
        value_map: &HashMap<mir::Value, cir::Value>,
        builder: &mut FunctionBuilder<'_>,
    ) -> CodegenCraneliftResult<(cir::Value, Option<cir::Value>)> {
        // plain function pointer
        if matches!(self.tree.get(signature), mir::Type::FunctionPointer { .. }) {
            let callee_value = value_map[&callee];
            return Ok((callee_value, None));
        }

        // closure aggregate
        let mir::Type::Closure {
            signature: function_type,
            environment,
        } = self.tree.get(signature)
        else {
            return Err(CodegenCraneliftError::Internal {
                message: "indirect call signature is not a function type".into(),
            });
        };
        let environment = self.type_id(*environment, "closure environment type")?;

        let callee_value = value_map[&callee];
        let signature_node = signature.into_any();
        let (function_offset, function_field_type) =
            self.aggregate_field_offset_and_type(signature, 0, signature_node)?;
        let (environment_offset, environment_field_type) =
            self.aggregate_field_offset_and_type(signature, 1, signature_node)?;

        let function_type = self.type_id(*function_type, "closure function type")?;

        if function_field_type != function_type {
            return Err(CodegenCraneliftError::Internal {
                message: "closure function field type mismatch".into(),
            });
        }

        if environment_field_type != environment {
            return Err(CodegenCraneliftError::Internal {
                message: "closure environment field type mismatch".into(),
            });
        }

        let function_ty = lower_type(self.tree, function_type, self.pointer_bytes)?;
        let environment_ty = self.pointer_type();
        let function_value = builder.ins().load(
            function_ty,
            cir::MemFlags::new(),
            callee_value,
            function_offset as i32,
        );
        let environment_value = builder.ins().load(
            environment_ty,
            cir::MemFlags::new(),
            callee_value,
            environment_offset as i32,
        );

        Ok((function_value, Some(environment_value)))
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
            mir::Constant::Null => Ok(builder.ins().iconst(self.pointer_type(), 0)),
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
                let value = i64::try_from(*value).map_err(|_| {
                    CodegenCraneliftError::unsupported_type(
                        format!("signed integer constant {value} does not fit i64"),
                        node_id,
                    )
                })?;

                Ok(builder.ins().iconst(ty, value))
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
                let value = u64::try_from(*value).map_err(|_| {
                    CodegenCraneliftError::unsupported_type(
                        format!("unsigned integer constant {value} does not fit u64"),
                        node_id,
                    )
                })?;

                Ok(builder.ins().iconst(ty, value as i64))
            }

            mir::Constant::Float { bits, format } => match format {
                mir::FloatType::Float16 => Ok(builder
                    .ins()
                    .f16const(cir::immediates::Ieee16::with_bits(*bits as u16))),
                mir::FloatType::Bfloat16 => Err(CodegenCraneliftError::unsupported_type(
                    "bfloat16 constant lowering is not supported",
                    node_id,
                )),
                mir::FloatType::Float32 => Ok(builder
                    .ins()
                    .f32const(cir::immediates::Ieee32::with_bits(*bits as u32))),
                mir::FloatType::Float64 => Ok(builder
                    .ins()
                    .f64const(cir::immediates::Ieee64::with_bits(*bits))),
            },

            // char constant: unicode codepoint as i32
            mir::Constant::Char { value } => {
                Ok(builder.ins().iconst(cir::types::I32, *value as i64))
            }
        }
    }

    /// Return one concrete MIR value from a recoverable reference.
    fn value_id(
        &self,
        value: mir::ValueReference,
        context: &str,
    ) -> CodegenCraneliftResult<mir::Value> {
        value
            .value()
            .ok_or_else(|| CodegenCraneliftError::Internal {
                message: format!("missing or malformed MIR value in native lowering: {context}"),
            })
    }

    /// Return one optional concrete MIR value from a recoverable reference.
    fn optional_value_id(
        &self,
        value: Option<mir::ValueReference>,
        context: &str,
    ) -> CodegenCraneliftResult<Option<mir::Value>> {
        value.map(|value| self.value_id(value, context)).transpose()
    }

    /// Return one concrete MIR type from a recoverable reference.
    fn type_id(
        &self,
        ty: mir::TypeReference,
        context: &str,
    ) -> CodegenCraneliftResult<mir::LocalNodeId<mir::Type>> {
        ty.ty().ok_or_else(|| CodegenCraneliftError::Internal {
            message: format!("missing or malformed MIR type in native lowering: {context}"),
        })
    }

    /// Return one optional concrete MIR type from a recoverable reference.
    fn optional_type_id(
        &self,
        ty: Option<mir::TypeReference>,
        context: &str,
    ) -> CodegenCraneliftResult<Option<mir::LocalNodeId<mir::Type>>> {
        ty.map(|ty| self.type_id(ty, context)).transpose()
    }

    /// Return one concrete MIR function from a recoverable reference.
    fn function_id(
        &self,
        function: mir::FunctionReference,
        context: &str,
    ) -> CodegenCraneliftResult<mir::LocalNodeId<mir::Function>> {
        function
            .function()
            .ok_or_else(|| CodegenCraneliftError::Internal {
                message: format!("missing or malformed MIR function in native lowering: {context}"),
            })
    }

    /// Return one concrete MIR block from a recoverable reference.
    fn block_id(
        &self,
        block: mir::BlockReference,
        context: &str,
    ) -> CodegenCraneliftResult<mir::LocalNodeId<mir::Block>> {
        block
            .block()
            .ok_or_else(|| CodegenCraneliftError::Internal {
                message: format!("missing or malformed MIR block in native lowering: {context}"),
            })
    }

    /// Return one concrete MIR local from a recoverable reference.
    fn local_id(
        &self,
        local: mir::LocalReference,
        context: &str,
    ) -> CodegenCraneliftResult<mir::LocalNodeId<mir::Local>> {
        local
            .local()
            .ok_or_else(|| CodegenCraneliftError::Internal {
                message: format!("missing or malformed MIR local in native lowering: {context}"),
            })
    }

    /// Return one concrete MIR global from a recoverable reference.
    fn global_id(
        &self,
        global: mir::GlobalReference,
        context: &str,
    ) -> CodegenCraneliftResult<mir::LocalNodeId<mir::Global>> {
        global
            .global()
            .ok_or_else(|| CodegenCraneliftError::Internal {
                message: format!("missing or malformed MIR global in native lowering: {context}"),
            })
    }

    /// Return one concrete MIR integer from a recoverable reference.
    fn integer_value(
        &self,
        value: mir::IntegerReference,
        context: &str,
    ) -> CodegenCraneliftResult<i128> {
        value
            .integer()
            .ok_or_else(|| CodegenCraneliftError::Internal {
                message: format!("missing or malformed MIR integer in native lowering: {context}"),
            })
    }

    /// Return one lowered Cranelift value for a MIR value reference.
    fn lowered_value(
        &self,
        value: mir::ValueReference,
        value_map: &HashMap<mir::Value, cir::Value>,
        context: &str,
    ) -> CodegenCraneliftResult<cir::Value> {
        let value = self.value_id(value, context)?;

        value_map
            .get(&value)
            .copied()
            .ok_or_else(|| CodegenCraneliftError::Internal {
                message: format!("missing lowered MIR value in native lowering: {context}"),
            })
    }

    /// Record one lowered Cranelift value for a MIR destination.
    fn insert_lowered_value(
        &self,
        value_map: &mut HashMap<mir::Value, cir::Value>,
        destination: mir::ValueReference,
        value: cir::Value,
        context: &str,
    ) -> CodegenCraneliftResult<()> {
        let destination = self.value_id(destination, context)?;
        value_map.insert(destination, value);

        Ok(())
    }

    /// Return the MIR type for a value or report a missing type.
    fn value_type_or_error(
        &self,
        value: mir::Value,
        node: mir::LocalNodeIdAny,
    ) -> CodegenCraneliftResult<mir::LocalNodeId<mir::Type>> {
        // fetch the SSA value type from the function table
        match self.function.value_type(value) {
            Some(ty) => Ok(ty),
            None => Err(CodegenCraneliftError::MissingType {
                node,
                message: Some(format!("missing value type for {value:?}")),
            }),
        }
    }

    /// Resolve an aggregate field offset and type from canonical layout metadata.
    fn aggregate_field_offset_and_type(
        &self,
        aggregate_type: mir::LocalNodeId<mir::Type>,
        index: u32,
        node: mir::LocalNodeIdAny,
    ) -> CodegenCraneliftResult<(u32, mir::LocalNodeId<mir::Type>)> {
        let field_type = match self.tree.get(aggregate_type) {
            mir::Type::Struct { fields, .. } => {
                let field_id = fields.get(index as usize).ok_or_else(|| {
                    CodegenCraneliftError::out_of_bounds(node, index, fields.len())
                })?;
                self.tree.get(*field_id).ty
            }
            mir::Type::Tuple { elements, .. } => *elements
                .get(index as usize)
                .ok_or_else(|| CodegenCraneliftError::out_of_bounds(node, index, elements.len()))?,
            mir::Type::Closure {
                signature,
                environment,
            } => match index {
                0 => *signature,
                1 => *environment,
                _ => return Err(CodegenCraneliftError::out_of_bounds(node, index, 2)),
            },
            _ => {
                return Err(CodegenCraneliftError::Internal {
                    message: "aggregate field lookup on non-aggregate type".into(),
                });
            }
        };

        let field_type = self.type_id(field_type, "aggregate field type")?;

        let layout = self.tree.type_layout(aggregate_type).ok_or_else(|| {
            CodegenCraneliftError::unsupported_type("missing aggregate layout metadata", node)
        })?;
        let fields = layout.shape.fields();
        let layout_field = layout
            .shape
            .fields()
            .iter()
            .find(|field| field.source_index == Some(index))
            .or_else(|| fields.get(index as usize))
            .ok_or_else(|| CodegenCraneliftError::out_of_bounds(node, index, fields.len()))?;

        if layout_field.ty != field_type {
            return Err(CodegenCraneliftError::Internal {
                message: "aggregate layout field type mismatch".into(),
            });
        }

        Ok((layout_field.offset, field_type))
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
        instruction_id: mir::LocalNodeIdAny,
        operator: mir::CastOperator,
        argument: cir::Value,
        to_type: cir::Type,
        builder: &mut FunctionBuilder<'_>,
    ) -> Result<cir::Value, CodegenCraneliftError> {
        let ins = builder.ins();

        let result = match operator {
            mir::CastOperator::Bitcast => ins.bitcast(to_type, cir::MemFlags::new(), argument),
            mir::CastOperator::Truncate => ins.ireduce(to_type, argument),
            mir::CastOperator::ZeroExtend => ins.uextend(to_type, argument),
            mir::CastOperator::SignExtend => ins.sextend(to_type, argument),
            mir::CastOperator::FloatToSignedInt => ins.fcvt_to_sint(to_type, argument),
            mir::CastOperator::FloatToUnsignedInt => ins.fcvt_to_uint(to_type, argument),
            mir::CastOperator::FloatToSignedIntSaturating => {
                ins.fcvt_to_sint_sat(to_type, argument)
            }
            mir::CastOperator::FloatToUnsignedIntSaturating => {
                ins.fcvt_to_uint_sat(to_type, argument)
            }
            mir::CastOperator::SignedIntToFloat => ins.fcvt_from_sint(to_type, argument),
            mir::CastOperator::UnsignedIntToFloat => ins.fcvt_from_uint(to_type, argument),
            mir::CastOperator::FloatTruncate => ins.fdemote(to_type, argument),
            mir::CastOperator::FloatExtend => ins.fpromote(to_type, argument),
            mir::CastOperator::FloatConvert => {
                return Err(CodegenCraneliftError::unsupported_instruction(
                    "native float format conversion is not supported",
                    instruction_id,
                ));
            }
            // pointer is already an integer in Cranelift
            mir::CastOperator::PointerToInt | mir::CastOperator::IntToPointer => argument,
        };

        Ok(result)
    }
}
