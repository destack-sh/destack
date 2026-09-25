use destack_core::{FxIndexMap, FxIndexSet};

use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{FuncId, Module};
use cranelift_object::ObjectModule;
use destack_artifact::MirOptimized;
use destack_mir as mir;
use destack_native as native;
use destack_source::ModuleId;

use crate::{EmitError, ObjectEmitter};

use super::super::object::SymbolTable;
use super::super::r#type::{TypeEmitter, ValueType};
use super::memory::AliasRegion;
use super::{Local, StackMap, Value};

/// Emit one MIR function into Cranelift IR.
pub(crate) struct FunctionEmitter<'a> {
    /// Module receiving diagnostics.
    pub(super) module: ModuleId,
    /// Optimized MIR being emitted.
    pub(super) optimized: &'a MirOptimized,
    /// Object-local identities and frame states.
    pub(super) object: &'a ObjectEmitter,
    /// Native type projection.
    pub(super) types: &'a TypeEmitter<'a>,
    /// Cranelift object module.
    pub(super) output: &'a mut ObjectModule,
    /// Object-local native symbols.
    pub(super) symbols: &'a mut SymbolTable,
    /// Declared internal functions.
    pub(super) functions: &'a FxIndexMap<mir::FunctionId, FuncId>,
    /// Declared platform imports.
    pub(super) imports: &'a mut FxIndexMap<FuncId, native::Import>,
    /// Function-local platform function references.
    pub(super) platform_functions: FxIndexMap<native::Import, cir::FuncRef>,
    /// Function-local references to linked Program indices.
    pub(super) indices: FxIndexMap<native::Index, cir::GlobalValue>,
    /// MIR function identity.
    pub(super) function_id: mir::FunctionId,
    /// MIR function declaration.
    pub(super) function: &'a mir::Function,
    /// Object-local function identity.
    pub(super) function_index: u32,
    /// Physical values keyed by MIR SSA identity.
    pub(super) values: Vec<Option<Value>>,
    /// Canonical locals keyed by MIR identity.
    pub(super) locals: FxIndexMap<mir::LocalId, Local>,
    /// Cranelift blocks keyed by MIR identity.
    pub(super) blocks: FxIndexMap<mir::BlockId, cir::Block>,
    /// Hidden native activation.
    pub(super) activation: Option<cir::Value>,
    /// Hidden callable environment.
    pub(super) environment: Option<cir::Value>,
    /// Hidden composite result address.
    pub(super) result: Option<cir::Value>,
    /// Platform unwind object retained while cleanup executes.
    pub(super) unwind: Option<cir::StackSlot>,
    /// First object-local frame-map id owned by this function.
    pub(super) frame_base: u32,
    /// Cranelift stack maps awaiting post-allocation locations.
    pub(super) stack_maps: Vec<StackMap>,
    /// Next function-local stack-slot identity.
    pub(super) next_key: u32,
    /// The trusted access flags of each alias region, declared when the body emission starts.
    pub(super) memory_flags: [cir::MemFlagsData; AliasRegion::ALL.len()],
}

impl<'a> FunctionEmitter<'a> {
    /// Create one native function emitter.
    pub(crate) fn new(
        module: ModuleId,
        optimized: &'a MirOptimized,
        object: &'a ObjectEmitter,
        types: &'a TypeEmitter<'a>,
        output: &'a mut ObjectModule,
        symbols: &'a mut SymbolTable,
        functions: &'a FxIndexMap<mir::FunctionId, FuncId>,
        imports: &'a mut FxIndexMap<FuncId, native::Import>,
        function_id: mir::FunctionId,
        function: &'a mir::Function,
        function_index: u32,
        frame_base: u32,
    ) -> Result<Self, EmitError> {
        // require a defined function body
        let body = function
            .body()
            .ok_or_else(|| Self::internal(module, "native function has no body"))?;
        let values = vec![None; body.value_capacity()];

        Ok(Self {
            module,
            optimized,
            object,
            types,
            output,
            symbols,
            functions,
            imports,
            platform_functions: FxIndexMap::default(),
            indices: FxIndexMap::default(),
            function_id,
            function,
            function_index,
            values,
            locals: FxIndexMap::default(),
            blocks: FxIndexMap::default(),
            activation: None,
            environment: None,
            result: None,
            unwind: None,
            frame_base,
            stack_maps: Vec::new(),
            next_key: 0,
            memory_flags: [cir::MemFlagsData::trusted(); AliasRegion::ALL.len()],
        })
    }

    /// Emit one typed native function body.
    pub(crate) fn emit(mut self, target: &mut cir::Function) -> Result<Vec<StackMap>, EmitError> {
        self.memory_flags = AliasRegion::ALL.map(|region| region.declare(target));

        let mut context = FunctionBuilderContext::new();
        let mut builder = cranelift_frontend::FunctionBuilder::new(target, &mut context);
        self.create_blocks(&mut builder)?;
        self.bind_parameters(&mut builder)?;
        self.create_locals(&mut builder)?;
        self.emit_blocks(&mut builder)?;
        builder.seal_all_blocks();
        builder.finalize(self.types.frontend_config());

        Ok(self.stack_maps)
    }

    /// Emit one canonical engine-transition entry.
    pub(crate) fn emit_entry(
        module: ModuleId,
        types: &TypeEmitter<'_>,
        output: &mut ObjectModule,
        function_id: FuncId,
        function: &mir::Function,
        target: &mut cir::Function,
    ) -> Result<(), EmitError> {
        let mut context = FunctionBuilderContext::new();
        let mut builder = cranelift_frontend::FunctionBuilder::new(target, &mut context);
        let block = builder.create_block();
        builder.append_block_params_for_function_params(block);
        builder.switch_to_block(block);
        builder.seal_block(block);

        // unpack the fixed outer ABI parameters
        let parameters = builder.block_params(block).to_vec();
        let activation = parameters[0];
        let arguments = parameters[1];
        let result_address = parameters[2];
        let callee = output.declare_func_in_func(function_id, builder.func);
        let mut call_arguments = vec![activation];

        // pass a canonical result address before typed arguments
        let result_type = types.result(function.return_type)?;
        if result_type.is_some_and(ValueType::is_indirect) {
            call_arguments.insert(1, result_address);
        }

        // unpack the hidden callable environment before explicit parameters
        let mut byte_offset = 0i32;
        if let Some(environment) = function.environment {
            let value_type = types.value(environment)?;
            Self::append_entry_value(
                value_type,
                arguments,
                byte_offset,
                &mut call_arguments,
                &mut builder,
            );
            byte_offset += i32::try_from(value_type.word_count() * 8)
                .map_err(|_| Self::internal(module, "native environment range is too large"))?;
        }

        // unpack explicit parameters in canonical argument order
        for parameter in &function.parameters {
            let value_type = types.value(parameter.ty)?;
            Self::append_entry_value(
                value_type,
                arguments,
                byte_offset,
                &mut call_arguments,
                &mut builder,
            );
            byte_offset += i32::try_from(value_type.word_count() * 8)
                .map_err(|_| Self::internal(module, "native argument range is too large"))?;
        }

        // call the typed body and marshal its direct result
        let call = builder.ins().call(callee, &call_arguments);
        let results = builder.inst_results(call).to_vec();
        let flags = cir::MemFlagsData::trusted();
        match result_type {
            Some(ValueType::Direct { .. }) => {
                builder.ins().store(flags, results[0], result_address, 0);
            }
            Some(ValueType::ScalarPair { fields, .. }) => {
                for (field, value) in fields.into_iter().zip(results.iter().copied()) {
                    builder
                        .ins()
                        .store(flags, value, result_address, field.offset as i32);
                }
            }
            Some(ValueType::Indirect { .. }) | None => {}
        }
        builder.ins().return_(&[]);
        builder.finalize(types.frontend_config());

        Ok(())
    }

    /// Append one canonical entry value to a typed call.
    fn append_entry_value(
        value_type: ValueType,
        arguments: cir::Value,
        byte_offset: i32,
        call_arguments: &mut Vec<cir::Value>,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) {
        let flags = cir::MemFlagsData::trusted();
        match value_type {
            ValueType::Direct { ty, .. } => {
                let value = builder.ins().load(ty, flags, arguments, byte_offset);
                call_arguments.push(value);
            }
            ValueType::ScalarPair { fields, .. } => {
                for field in fields {
                    let offset = byte_offset + field.offset as i32;
                    let value = builder.ins().load(field.ty, flags, arguments, offset);
                    call_arguments.push(value);
                }
            }
            ValueType::Indirect { .. } => {
                let address = builder.ins().iadd_imm_u(arguments, i64::from(byte_offset));
                call_arguments.push(address);
            }
        }
    }

    /// Create every natively reachable Cranelift block before emitting edges.
    pub(super) fn create_blocks(
        &mut self,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // read the MIR entry block
        let entry = self
            .function
            .entry()
            .ok_or_else(|| self.invalid("native function has no entry block"))?;
        let reachable = self.reachable_blocks(entry);
        for block_id in self.function.blocks() {
            if !reachable.contains(block_id) {
                continue;
            }

            self.blocks.insert(*block_id, builder.create_block());
        }

        // add nonentry block parameters in MIR order
        for block_id in self.function.blocks() {
            if *block_id == entry || !reachable.contains(block_id) {
                continue;
            }
            let block = self.optimized.tree.get(*block_id);
            let target = self.blocks[block_id];
            for parameter in &block.parameters {
                let value_type = self.types.value(parameter.ty)?;
                match value_type {
                    ValueType::Direct { ty, .. } => {
                        builder.append_block_param(target, ty);
                    }
                    ValueType::ScalarPair { fields, .. } => {
                        for field in fields {
                            builder.append_block_param(target, field.ty);
                        }
                    }
                    ValueType::Indirect { .. } => {
                        builder.append_block_param(target, self.types.pointer());
                    }
                }
            }
        }

        Ok(())
    }

    /// Return the MIR blocks executable without crossing an engine transition.
    fn reachable_blocks(&self, entry: mir::BlockId) -> FxIndexSet<mir::BlockId> {
        let mut reachable = FxIndexSet::default();
        let mut pending = vec![entry];

        // walk only control flow retained by the native body
        while let Some(block_id) = pending.pop() {
            if !reachable.insert(block_id) {
                continue;
            }

            pending.extend(self.successors(block_id));
        }

        reachable
    }

    /// Return successors reached without deoptimization or stopping.
    fn successors(&self, block_id: mir::BlockId) -> Vec<mir::BlockId> {
        // check whether a breakpoint stops the block
        let block = self.optimized.tree.get(block_id);
        let stops = block.instructions.iter().any(|instruction| {
            matches!(
                self.optimized.tree.get(*instruction),
                mir::Instruction::Breakpoint
            )
        });
        if stops {
            return Vec::new();
        }

        // read the terminator after checking for a breakpoint
        let terminator = self.optimized.tree.get(block.terminator);

        terminator.successors(&self.optimized.tree).into_vec()
    }

    /// Bind hidden and explicit function parameters.
    pub(super) fn bind_parameters(
        &mut self,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let entry_id = self
            .function
            .entry()
            .ok_or_else(|| self.invalid("native function has no entry block"))?;
        let entry = self.blocks[&entry_id];
        let existing = builder.block_params(entry).to_vec();
        let signature = builder.func.signature.clone();
        // prepend hidden and explicit ABI parameters to the MIR entry block
        let mut parameters = Vec::with_capacity(signature.params.len() + existing.len());
        for parameter in &signature.params {
            parameters.push(builder.append_block_param(entry, parameter.value_type));
        }
        parameters.extend(existing);
        self.activation = Some(parameters[0]);
        let mut index = 1;
        builder.switch_to_block(entry);
        if self
            .types
            .result(self.function.return_type)?
            .is_some_and(ValueType::is_indirect)
        {
            self.result = Some(parameters[index]);
            index += 1;
        }
        if self.function.environment.is_some() {
            self.environment = Some(parameters[index]);
            index += 1;
        }
        for parameter in &self.function.parameters {
            let value_type = self.types.value(parameter.ty)?;
            let value = Value::from_parameters(value_type, &parameters, &mut index)
                .ok_or_else(|| self.invalid("native function parameters do not match its ABI"))?;
            self.set(parameter.value, value)?;
        }

        builder.seal_block(entry);

        Ok(())
    }

    /// Emit every MIR block in canonical operation order.
    pub(super) fn emit_blocks(
        &mut self,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // order the function blocks for emission
        let mut blocks = self.function.blocks().to_vec();
        self.object.order_blocks(&mut blocks);

        // emit instructions and terminators in block order
        for block_id in blocks {
            let Some(&target) = self.blocks.get(&block_id) else {
                continue;
            };

            if builder.current_block() != Some(target) {
                builder.switch_to_block(target);
            }
            let block = self.optimized.tree.get(block_id);
            if Some(block_id) != self.function.entry() {
                let parameters = builder.block_params(target).to_vec();
                let mut index = 0;
                for parameter in &block.parameters {
                    let value_type = self.types.value(parameter.ty)?;
                    let value = Value::from_parameters(value_type, &parameters, &mut index)
                        .ok_or_else(|| {
                            self.invalid("native block parameters do not match the MIR block")
                        })?;
                    self.set(parameter.value, value)?;
                }
            }

            // emit straight-line instructions
            let mut is_active = true;
            for instruction_id in &block.instructions {
                let instruction = self.optimized.tree.get(*instruction_id);
                is_active = self.emit_instruction(*instruction_id, instruction, builder)?;
                if !is_active {
                    break;
                }
            }

            // terminate the block exactly once
            if is_active {
                // read the terminator after checking for a breakpoint
                let terminator = self.optimized.tree.get(block.terminator);
                self.emit_terminator(block_id, terminator, builder)?;
            }
        }

        Ok(())
    }

    /// Build one unexpected native emission diagnostic.
    pub(super) fn invalid(&self, message: &str) -> EmitError {
        EmitError::UnexpectedConstruct {
            anchor: self.module.into(),
            module: self.module,
            message: message.to_owned(),
        }
    }

    /// Build one internal native emission diagnostic.
    pub(super) fn internal(module: ModuleId, message: impl Into<String>) -> EmitError {
        EmitError::Internal {
            anchor: module.into(),
            module,
            message: message.into(),
        }
    }
}
