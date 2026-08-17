use destack_artifact::MirOptimized;
use destack_bytecode as bytecode;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::{EmitError, ObjectEmitter};

use super::super::TypeEmitter;

/// One encoded tensor command and its optional destination.
#[derive(Debug)]
pub(crate) struct TensorCommand {
    /// Encoded tensor operation.
    pub(crate) instruction: bytecode::InstructionBuilder,
    /// Physical destination registers when the operation produces a value.
    pub(crate) destination: Option<bytecode::RegisterSpan>,
}

/// Encode MIR tensor operations into the shared executable descriptor.
pub(crate) struct TensorEmitter<'a> {
    /// Module receiving diagnostics.
    module: ModuleId,
    /// Optimized MIR being emitted.
    optimized: &'a MirOptimized,
    /// Common object identity assignments.
    object: &'a ObjectEmitter,
    /// Object-local bytecode type identities.
    types: TypeEmitter<'a>,
    /// Function owning the operation.
    function: &'a mir::Function,
    /// Physical registers keyed by MIR value.
    registers: &'a [Option<bytecode::RegisterSpan>],
}

impl<'a> TensorEmitter<'a> {
    /// Create one tensor command emitter.
    pub(crate) fn new(
        module: ModuleId,
        optimized: &'a MirOptimized,
        object: &'a ObjectEmitter,
        function: &'a mir::Function,
        registers: &'a [Option<bytecode::RegisterSpan>],
    ) -> Self {
        Self {
            module,
            optimized,
            object,
            types: TypeEmitter::new(module, optimized, object),
            function,
            registers,
        }
    }

    /// Encode one MIR tensor instruction.
    pub(crate) fn emit(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
    ) -> Result<TensorCommand, EmitError> {
        match instruction {
            mir::Instruction::TensorSplat { destination, value } => {
                self.splat(instruction_id, *destination, *value)
            }
            mir::Instruction::TensorLoad {
                destination,
                view,
                indices,
            } => self.read(
                bytecode::TensorOperation::Load,
                *destination,
                *view,
                *indices,
            ),
            mir::Instruction::TensorExtract {
                destination,
                tensor,
                indices,
            } => self.read(
                bytecode::TensorOperation::Extract,
                *destination,
                *tensor,
                *indices,
            ),
            mir::Instruction::TensorStore {
                view,
                indices,
                value,
            } => self.store(*view, *indices, *value),
            mir::Instruction::TensorFill { view, value } => self.fill(*view, *value),
            mir::Instruction::TensorCopy { target, source } => self.copy(*target, *source),
            mir::Instruction::TensorReshape {
                destination,
                tensor,
                shape,
            } => self.reshape(instruction_id, *destination, *tensor, *shape),
            mir::Instruction::TensorBroadcast {
                destination,
                tensor,
                dimensions,
            } => self.permutation(
                instruction_id,
                bytecode::TensorOperation::Broadcast,
                *destination,
                *tensor,
                *dimensions,
            ),
            mir::Instruction::TensorTranspose {
                destination,
                tensor,
                permutation,
            } => self.permutation(
                instruction_id,
                bytecode::TensorOperation::Transpose,
                *destination,
                *tensor,
                *permutation,
            ),
            mir::Instruction::TensorCast {
                destination,
                tensor,
                ..
            } => self.bitcast(instruction_id, *destination, *tensor),
            mir::Instruction::TensorView {
                destination,
                view,
                arguments,
                offsets_count,
                sizes_count,
                strides_count,
            } => self.view(
                *destination,
                *view,
                *arguments,
                [*offsets_count, *sizes_count, *strides_count],
            ),
            mir::Instruction::TensorSlice {
                destination,
                tensor,
                arguments,
                offsets_count,
                sizes_count,
                strides_count,
            } => self.slice(
                instruction_id,
                *destination,
                *tensor,
                *arguments,
                [*offsets_count, *sizes_count, *strides_count],
            ),
            mir::Instruction::TensorPad {
                destination,
                tensor,
                arguments,
                low_count,
                high_count,
                interior_count,
                value,
            } => self.pad(
                instruction_id,
                *destination,
                *tensor,
                *arguments,
                [*low_count, *high_count, *interior_count],
                *value,
            ),
            mir::Instruction::TensorConcat {
                destination,
                tensors,
                axis,
            } => self.concat(instruction_id, *destination, *tensors, *axis),
            mir::Instruction::TensorCompare {
                destination,
                operator,
                left,
                right,
            } => self.compare(instruction_id, *destination, *operator, *left, *right),
            mir::Instruction::TensorSelect {
                destination,
                mask,
                then_value,
                else_value,
            } => self.select(
                instruction_id,
                *destination,
                *mask,
                *then_value,
                *else_value,
            ),
            mir::Instruction::TensorReduce {
                destination,
                operator,
                tensor,
                initial,
                axes,
            } => self.reduce(
                instruction_id,
                *destination,
                *operator,
                *tensor,
                *initial,
                *axes,
            ),
            mir::Instruction::TensorIndexReduce {
                destination,
                operator,
                tensor,
                axis,
                tie_break,
            } => self.index_reduce(
                instruction_id,
                *destination,
                *operator,
                *tensor,
                *axis,
                *tie_break,
            ),
            mir::Instruction::TensorDot {
                destination,
                left,
                right,
                immediate,
            } => self.contract(instruction_id, *destination, *left, *right, *immediate),
            mir::Instruction::TensorConvolution {
                destination,
                input,
                kernel,
                immediate,
            } => self.convolution(instruction_id, *destination, *input, *kernel, *immediate),
            mir::Instruction::TensorGather {
                destination,
                operand,
                indices,
                immediate,
            } => self.gather(instruction_id, *destination, *operand, *indices, *immediate),
            mir::Instruction::TensorScatter {
                destination,
                operand,
                indices,
                updates,
                immediate,
                mode,
            } => self.scatter(
                instruction_id,
                *destination,
                *operand,
                *indices,
                *updates,
                *immediate,
                *mode,
            ),
            mir::Instruction::TensorConvert {
                destination,
                mode,
                tensor,
            } => self.convert(instruction_id, *destination, *mode, *tensor),
            _ => Err(self.internal("expected tensor instruction")),
        }
    }

    /// Encode one scalar splat.
    fn splat(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        value: mir::Value,
    ) -> Result<TensorCommand, EmitError> {
        let mut instruction = self.allocation(instruction_id, bytecode::TensorOperation::Splat)?;
        instruction.register(self.word(value)?);
        self.allocation_id(instruction_id, &mut instruction)?;

        self.command(instruction, Some(destination))
    }

    /// Encode one indexed element read.
    fn read(
        &self,
        operation: bytecode::TensorOperation,
        destination: mir::Value,
        tensor: mir::Value,
        indices: mir::ValueSlice,
    ) -> Result<TensorCommand, EmitError> {
        let mut instruction = self.instruction(operation);
        instruction.tensor(self.tensor(tensor)?);
        let indices = self.words(indices)?;
        instruction
            .registers(&indices)
            .map_err(|error| self.bytecode_error(error))?;

        self.command(instruction, Some(destination))
    }

    /// Encode one indexed element store.
    fn store(
        &self,
        view: mir::Value,
        indices: mir::ValueSlice,
        value: mir::Value,
    ) -> Result<TensorCommand, EmitError> {
        let mut instruction = self.instruction(bytecode::TensorOperation::Store);
        instruction.tensor(self.tensor(view)?);
        let indices = self.words(indices)?;
        instruction
            .registers(&indices)
            .map_err(|error| self.bytecode_error(error))?;
        instruction.register(self.word(value)?);

        self.command(instruction, None)
    }

    /// Encode one tensor fill.
    fn fill(&self, view: mir::Value, value: mir::Value) -> Result<TensorCommand, EmitError> {
        let mut instruction = self.instruction(bytecode::TensorOperation::Fill);
        instruction.tensor(self.tensor(view)?);
        instruction.register(self.word(value)?);

        self.command(instruction, None)
    }

    /// Encode one tensor copy.
    fn copy(&self, target: mir::Value, source: mir::Value) -> Result<TensorCommand, EmitError> {
        let mut instruction = self.instruction(bytecode::TensorOperation::Copy);
        instruction.tensor(self.tensor(target)?);
        instruction.tensor(self.tensor(source)?);

        self.command(instruction, None)
    }

    /// Encode one tensor reshape.
    fn reshape(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        tensor: mir::Value,
        shape: mir::ValueSlice,
    ) -> Result<TensorCommand, EmitError> {
        let mut instruction =
            self.allocation(instruction_id, bytecode::TensorOperation::Reshape)?;
        instruction.tensor(self.tensor(tensor)?);
        let shape = self.words(shape)?;
        instruction
            .registers(&shape)
            .map_err(|error| self.bytecode_error(error))?;
        self.allocation_id(instruction_id, &mut instruction)?;

        self.command(instruction, Some(destination))
    }

    /// Encode one tensor dimension permutation.
    fn permutation(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        operation: bytecode::TensorOperation,
        destination: mir::Value,
        tensor: mir::Value,
        dimensions: mir::IndexSlice,
    ) -> Result<TensorCommand, EmitError> {
        let dimensions = self.indices(dimensions)?;
        let mut instruction = self.allocation(instruction_id, operation)?;
        instruction.tensor(self.tensor(tensor)?);
        instruction
            .u16s(&dimensions)
            .map_err(|error| self.bytecode_error(error))?;
        self.allocation_id(instruction_id, &mut instruction)?;

        self.command(instruction, Some(destination))
    }

    /// Encode one tensor bitcast.
    fn bitcast(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        tensor: mir::Value,
    ) -> Result<TensorCommand, EmitError> {
        let mut instruction =
            self.allocation(instruction_id, bytecode::TensorOperation::Bitcast)?;
        instruction.tensor(self.tensor(tensor)?);
        self.allocation_id(instruction_id, &mut instruction)?;

        self.command(instruction, Some(destination))
    }

    /// Encode one borrowed tensor view.
    fn view(
        &self,
        destination: mir::Value,
        view: mir::Value,
        arguments: mir::ValueSlice,
        counts: [u16; 3],
    ) -> Result<TensorCommand, EmitError> {
        let arguments = self.split_words(arguments, counts)?;
        let result_type = self.type_id(self.value_type(destination)?)?;
        let mut instruction = self.instruction(bytecode::TensorOperation::View);
        instruction.tensor(self.tensor(view)?);
        for values in arguments {
            instruction
                .registers(&values)
                .map_err(|error| self.bytecode_error(error))?;
        }
        instruction.relocation(bytecode::RelocationTag::LAYOUT, result_type.0);

        self.command(instruction, Some(destination))
    }

    /// Encode one allocated tensor slice.
    fn slice(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        tensor: mir::Value,
        arguments: mir::ValueSlice,
        counts: [u16; 3],
    ) -> Result<TensorCommand, EmitError> {
        let arguments = self.split_words(arguments, counts)?;
        let mut instruction = self.allocation(instruction_id, bytecode::TensorOperation::Slice)?;
        instruction.tensor(self.tensor(tensor)?);
        for values in arguments {
            instruction
                .registers(&values)
                .map_err(|error| self.bytecode_error(error))?;
        }
        self.allocation_id(instruction_id, &mut instruction)?;

        self.command(instruction, Some(destination))
    }

    /// Encode one padded tensor allocation.
    fn pad(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        tensor: mir::Value,
        arguments: mir::ValueSlice,
        counts: [u16; 3],
        value: mir::Value,
    ) -> Result<TensorCommand, EmitError> {
        let arguments = self.split_words(arguments, counts)?;
        let mut instruction = self.allocation(instruction_id, bytecode::TensorOperation::Pad)?;
        instruction.tensor(self.tensor(tensor)?);
        instruction.register(self.word(value)?);
        for values in arguments {
            instruction
                .registers(&values)
                .map_err(|error| self.bytecode_error(error))?;
        }
        self.allocation_id(instruction_id, &mut instruction)?;

        self.command(instruction, Some(destination))
    }

    /// Encode one tensor concatenation.
    fn concat(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        tensors: mir::ValueSlice,
        axis: u32,
    ) -> Result<TensorCommand, EmitError> {
        let tensors = self.tensors(tensors)?;
        let axis = self.index(axis)?;
        let mut instruction = self.allocation(instruction_id, bytecode::TensorOperation::Concat)?;
        instruction
            .tensors(&tensors)
            .map_err(|error| self.bytecode_error(error))?;
        instruction.u16(axis);
        self.allocation_id(instruction_id, &mut instruction)?;

        self.command(instruction, Some(destination))
    }

    /// Encode one tensor comparison.
    fn compare(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        operator: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
    ) -> Result<TensorCommand, EmitError> {
        let scalar = self.tensor_scalar(left)?;
        let operation = self
            .element_operation(operator, scalar)
            .ok_or_else(|| self.internal("invalid tensor comparison"))?;
        let mut instruction =
            self.allocation(instruction_id, bytecode::TensorOperation::Compare)?;
        instruction.tensor(self.tensor(left)?);
        instruction.tensor(self.tensor(right)?);
        instruction.u16(operation.code());
        self.allocation_id(instruction_id, &mut instruction)?;

        self.command(instruction, Some(destination))
    }

    /// Encode one tensor element selection.
    fn select(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        mask: mir::Value,
        then_value: mir::Value,
        else_value: mir::Value,
    ) -> Result<TensorCommand, EmitError> {
        let mut instruction = self.allocation(instruction_id, bytecode::TensorOperation::Select)?;
        instruction.tensor(self.tensor(mask)?);
        instruction.tensor(self.tensor(then_value)?);
        instruction.tensor(self.tensor(else_value)?);
        self.allocation_id(instruction_id, &mut instruction)?;

        self.command(instruction, Some(destination))
    }

    /// Encode one tensor reduction.
    fn reduce(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        operator: mir::TensorReduceOperator,
        tensor: mir::Value,
        initial: mir::Value,
        axes: mir::IndexSlice,
    ) -> Result<TensorCommand, EmitError> {
        let axes = self.indices(axes)?;
        let mut instruction = self.allocation(instruction_id, bytecode::TensorOperation::Reduce)?;
        instruction.tensor(self.tensor(tensor)?);
        instruction.register(self.word(initial)?);
        instruction.u16(Self::reduce_operation(operator) as u16);
        instruction
            .u16s(&axes)
            .map_err(|error| self.bytecode_error(error))?;
        self.allocation_id(instruction_id, &mut instruction)?;

        self.command(instruction, Some(destination))
    }

    /// Encode one tensor index reduction.
    fn index_reduce(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        operator: mir::TensorIndexReduceOperator,
        tensor: mir::Value,
        axis: u32,
        tie_break: mir::TensorIndexTieBreak,
    ) -> Result<TensorCommand, EmitError> {
        let operator = match operator {
            mir::TensorIndexReduceOperator::Min => bytecode::IndexReduceOperation::Minimum,
            mir::TensorIndexReduceOperator::Max => bytecode::IndexReduceOperation::Maximum,
        };
        let tie_break = match tie_break {
            mir::TensorIndexTieBreak::First => bytecode::TieBreak::First,
            mir::TensorIndexTieBreak::Last => bytecode::TieBreak::Last,
        };
        let mut instruction =
            self.allocation(instruction_id, bytecode::TensorOperation::IndexReduce)?;
        instruction.tensor(self.tensor(tensor)?);
        instruction.u16(operator as u16);
        instruction.u16(self.index(axis)?);
        instruction.u16(tie_break as u16);
        self.allocation_id(instruction_id, &mut instruction)?;

        self.command(instruction, Some(destination))
    }

    /// Encode one tensor contraction.
    fn contract(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        left: mir::Value,
        right: mir::Value,
        immediate: mir::TensorImmediateId,
    ) -> Result<TensorCommand, EmitError> {
        let mir::TensorImmediate::Dot {
            lhs_batch,
            rhs_batch,
            lhs_contracting,
            rhs_contracting,
        } = self.optimized.tree.get_tensor_immediate(immediate)
        else {
            return Err(self.internal("tensor dot has invalid immediate"));
        };
        let axes = [
            self.indices(*lhs_batch)?,
            self.indices(*rhs_batch)?,
            self.indices(*lhs_contracting)?,
            self.indices(*rhs_contracting)?,
        ];
        let mut instruction =
            self.allocation(instruction_id, bytecode::TensorOperation::Contract)?;
        instruction.tensor(self.tensor(left)?);
        instruction.tensor(self.tensor(right)?);
        for values in axes {
            instruction
                .u16s(&values)
                .map_err(|error| self.bytecode_error(error))?;
        }
        self.allocation_id(instruction_id, &mut instruction)?;

        self.command(instruction, Some(destination))
    }

    /// Encode one tensor convolution.
    #[allow(clippy::too_many_arguments)]
    fn convolution(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        input: mir::Value,
        kernel: mir::Value,
        immediate: mir::TensorImmediateId,
    ) -> Result<TensorCommand, EmitError> {
        let mir::TensorImmediate::Convolution {
            input_batch,
            input_feature,
            input_spatial,
            kernel_input_feature,
            kernel_output_feature,
            kernel_spatial,
            output_batch,
            output_feature,
            output_spatial,
            strides,
            padding_low,
            padding_high,
            lhs_dilation,
            rhs_dilation,
            window_reversal,
            feature_group_count,
            batch_group_count,
        } = self.optimized.tree.get_tensor_immediate(immediate)
        else {
            return Err(self.internal("tensor convolution has invalid immediate"));
        };
        let dimensions = [
            (*input_batch, *input_feature, *input_spatial),
            (
                *kernel_input_feature,
                *kernel_output_feature,
                *kernel_spatial,
            ),
            (*output_batch, *output_feature, *output_spatial),
        ];
        let mut instruction =
            self.allocation(instruction_id, bytecode::TensorOperation::Convolution)?;
        instruction.tensor(self.tensor(input)?);
        instruction.tensor(self.tensor(kernel)?);
        for (batch, feature, spatial) in dimensions {
            instruction.u16(self.index(batch)?);
            instruction.u16(self.index(feature)?);
            instruction
                .u16s(&self.indices(spatial)?)
                .map_err(|error| self.bytecode_error(error))?;
        }
        for values in [
            self.extents(*strides),
            self.extents(*padding_low),
            self.extents(*padding_high),
            self.extents(*lhs_dilation),
            self.extents(*rhs_dilation),
        ] {
            instruction
                .u64s(values)
                .map_err(|error| self.bytecode_error(error))?;
        }
        let reversal = self
            .optimized
            .tree
            .get_flags(*window_reversal)
            .iter()
            .map(|flag| u16::from(*flag != 0))
            .collect::<Vec<_>>();
        instruction
            .u16s(&reversal)
            .map_err(|error| self.bytecode_error(error))?;
        instruction.u32(*feature_group_count);
        instruction.u32(*batch_group_count);
        self.allocation_id(instruction_id, &mut instruction)?;

        self.command(instruction, Some(destination))
    }

    /// Encode one tensor gather.
    fn gather(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        operand: mir::Value,
        indices: mir::Value,
        immediate: mir::TensorImmediateId,
    ) -> Result<TensorCommand, EmitError> {
        let mir::TensorImmediate::Gather {
            offset_dims,
            collapsed_slice_dims,
            start_index_map,
            index_vector_dim,
            slice_sizes,
        } = self.optimized.tree.get_tensor_immediate(immediate)
        else {
            return Err(self.internal("tensor gather has invalid immediate"));
        };
        let axes = [
            self.indices(*offset_dims)?,
            self.indices(*collapsed_slice_dims)?,
            self.indices(*start_index_map)?,
        ];
        let slice_sizes = self
            .optimized
            .tree
            .get_indices(*slice_sizes)
            .iter()
            .map(|size| u64::from(*size))
            .collect::<Vec<_>>();
        let mut instruction = self.allocation(instruction_id, bytecode::TensorOperation::Gather)?;
        instruction.tensor(self.tensor(operand)?);
        instruction.tensor(self.tensor(indices)?);
        for values in axes {
            instruction
                .u16s(&values)
                .map_err(|error| self.bytecode_error(error))?;
        }
        instruction.u16(self.index(*index_vector_dim)?);
        instruction
            .u64s(&slice_sizes)
            .map_err(|error| self.bytecode_error(error))?;
        self.allocation_id(instruction_id, &mut instruction)?;

        self.command(instruction, Some(destination))
    }

    /// Encode one tensor scatter.
    #[allow(clippy::too_many_arguments)]
    fn scatter(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        operand: mir::Value,
        indices: mir::Value,
        updates: mir::Value,
        immediate: mir::TensorImmediateId,
        mode: mir::TensorScatterMode,
    ) -> Result<TensorCommand, EmitError> {
        let mir::TensorImmediate::Scatter {
            update_window_dims,
            inserted_window_dims,
            scatter_dims_to_operand_dims,
            index_vector_dim,
        } = self.optimized.tree.get_tensor_immediate(immediate)
        else {
            return Err(self.internal("tensor scatter has invalid immediate"));
        };
        let axes = [
            self.indices(*update_window_dims)?,
            self.indices(*inserted_window_dims)?,
            self.indices(*scatter_dims_to_operand_dims)?,
        ];
        let mut instruction =
            self.allocation(instruction_id, bytecode::TensorOperation::Scatter)?;
        instruction.tensor(self.tensor(operand)?);
        instruction.tensor(self.tensor(indices)?);
        instruction.tensor(self.tensor(updates)?);
        for values in axes {
            instruction
                .u16s(&values)
                .map_err(|error| self.bytecode_error(error))?;
        }
        instruction.u16(self.index(*index_vector_dim)?);
        instruction.u16(Self::scatter_operation(mode) as u16);
        self.allocation_id(instruction_id, &mut instruction)?;

        self.command(instruction, Some(destination))
    }

    /// Encode one tensor element conversion.
    fn convert(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        mode: mir::ConvertMode,
        tensor: mir::Value,
    ) -> Result<TensorCommand, EmitError> {
        let mut instruction =
            self.allocation(instruction_id, bytecode::TensorOperation::Convert)?;
        instruction.tensor(self.tensor(tensor)?);
        instruction.u16(Self::convert_mode(mode) as u16);
        self.allocation_id(instruction_id, &mut instruction)?;

        self.command(instruction, Some(destination))
    }

    /// Create one allocating tensor instruction.
    fn allocation(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        operation: bytecode::TensorOperation,
    ) -> Result<bytecode::InstructionBuilder, EmitError> {
        let point = self.object.instruction_point(instruction_id);
        if self.object.allocation_index(point).is_none() {
            return Err(self.internal("missing tensor allocation site"));
        }

        Ok(self.instruction(operation))
    }

    /// Create one tensor instruction.
    fn instruction(&self, operation: bytecode::TensorOperation) -> bytecode::InstructionBuilder {
        bytecode::InstructionBuilder::new(bytecode::Opcode::tensor(operation))
    }

    /// Append one tensor allocation identity.
    fn allocation_id(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mut bytecode::InstructionBuilder,
    ) -> Result<(), EmitError> {
        let point = self.object.instruction_point(instruction_id);
        let allocation = self
            .object
            .allocation_index(point)
            .ok_or_else(|| self.internal("missing tensor allocation site"))?;
        instruction.relocation(bytecode::RelocationTag::ALLOCATION, allocation);

        Ok(())
    }

    /// Finish one tensor command.
    fn command(
        &self,
        instruction: bytecode::InstructionBuilder,
        destination: Option<mir::Value>,
    ) -> Result<TensorCommand, EmitError> {
        let destination = destination.map(|value| self.register(value)).transpose()?;

        Ok(TensorCommand {
            instruction,
            destination,
        })
    }

    /// Return one tensor operand and its object-local layout.
    fn tensor(&self, value: mir::Value) -> Result<bytecode::TensorOperand, EmitError> {
        let ty = self.type_id(self.value_type(value)?)?;

        Ok(bytecode::TensorOperand::new(
            self.register(value)?,
            bytecode::LayoutId(ty.0),
        ))
    }

    /// Return tensor operands for one MIR value slice.
    fn tensors(&self, values: mir::ValueSlice) -> Result<Vec<bytecode::TensorOperand>, EmitError> {
        self.optimized
            .tree
            .get_values(values)
            .iter()
            .map(|value| self.tensor(*value))
            .collect()
    }

    /// Return single-word registers for one MIR value slice.
    fn words(&self, values: mir::ValueSlice) -> Result<Vec<bytecode::RegisterId>, EmitError> {
        self.optimized
            .tree
            .get_values(values)
            .iter()
            .map(|value| self.word(*value))
            .collect()
    }

    /// Split one MIR value slice into exact single-word groups.
    fn split_words<const N: usize>(
        &self,
        values: mir::ValueSlice,
        counts: [u16; N],
    ) -> Result<[Vec<bytecode::RegisterId>; N], EmitError> {
        let words = self.words(values)?;
        let expected = counts
            .iter()
            .map(|count| usize::from(*count))
            .sum::<usize>();
        if words.len() != expected {
            return Err(self.internal("tensor operand counts do not match arguments"));
        }

        let mut start = 0;
        Ok(counts.map(|count| {
            let end = start + usize::from(count);
            let values = words[start..end].to_vec();
            start = end;

            values
        }))
    }

    /// Return one MIR value's physical register range.
    fn register(&self, value: mir::Value) -> Result<bytecode::RegisterSpan, EmitError> {
        self.registers
            .get(value.id() as usize)
            .copied()
            .flatten()
            .ok_or_else(|| self.internal("missing tensor value register"))
    }

    /// Return one single-word MIR value register.
    fn word(&self, value: mir::Value) -> Result<bytecode::RegisterId, EmitError> {
        let registers = self.register(value)?;
        if registers.word_count != 1 {
            return Err(self.internal("expected one-register tensor operand"));
        }

        Ok(registers.start)
    }

    /// Return one MIR value's type.
    fn value_type(&self, value: mir::Value) -> Result<mir::TypeId, EmitError> {
        self.function
            .value_type(value)
            .ok_or_else(|| self.internal("missing tensor value type"))
    }

    /// Return one object-local bytecode type identity.
    fn type_id(&self, ty: mir::TypeId) -> Result<bytecode::TypeId, EmitError> {
        self.types.type_id(ty)
    }

    /// Return one tensor element representation.
    fn tensor_scalar(&self, value: mir::Value) -> Result<bytecode::Scalar, EmitError> {
        self.types
            .register_type(self.value_type(value)?)?
            .tensor_scalar()
            .ok_or_else(|| self.internal("expected tensor value"))
    }

    /// Return one MIR index as a bytecode immediate.
    fn index(&self, index: u32) -> Result<u16, EmitError> {
        u16::try_from(index).map_err(|_| self.internal("tensor index exceeds u16"))
    }

    /// Return one MIR index slice as bytecode immediates.
    fn indices(&self, indices: mir::IndexSlice) -> Result<Vec<u16>, EmitError> {
        self.optimized
            .tree
            .get_indices(indices)
            .iter()
            .map(|index| self.index(*index))
            .collect()
    }

    /// Return one MIR extent slice.
    fn extents(&self, extents: mir::ExtentSlice) -> &[u64] {
        self.optimized.tree.get_extents(extents)
    }

    /// Return one bytecode tensor comparison operation.
    fn element_operation(
        &self,
        operator: mir::BinaryOperator,
        scalar: bytecode::Scalar,
    ) -> Option<bytecode::ElementOperation> {
        if scalar.is_float() {
            let operation = match operator {
                mir::BinaryOperator::Equal => bytecode::FloatOperation::Equal,
                mir::BinaryOperator::NotEqual => bytecode::FloatOperation::NotEqual,
                mir::BinaryOperator::LessThan => bytecode::FloatOperation::LessThan,
                mir::BinaryOperator::LessEqual => bytecode::FloatOperation::LessEqual,
                mir::BinaryOperator::GreaterThan => bytecode::FloatOperation::GreaterThan,
                mir::BinaryOperator::GreaterEqual => bytecode::FloatOperation::GreaterEqual,
                _ => return None,
            };

            Some(bytecode::ElementOperation::float(operation))
        } else {
            let operation = match operator {
                mir::BinaryOperator::Equal => bytecode::IntegerOperation::Equal,
                mir::BinaryOperator::NotEqual => bytecode::IntegerOperation::NotEqual,
                mir::BinaryOperator::LessThan => bytecode::IntegerOperation::LessThan,
                mir::BinaryOperator::LessEqual => bytecode::IntegerOperation::LessEqual,
                mir::BinaryOperator::GreaterThan => bytecode::IntegerOperation::GreaterThan,
                mir::BinaryOperator::GreaterEqual => bytecode::IntegerOperation::GreaterEqual,
                _ => return None,
            };

            Some(bytecode::ElementOperation::integer(operation))
        }
    }

    /// Return one bytecode tensor reduction operation.
    fn reduce_operation(operator: mir::TensorReduceOperator) -> bytecode::ReduceOperation {
        match operator {
            mir::TensorReduceOperator::Add => bytecode::ReduceOperation::Add,
            mir::TensorReduceOperator::Multiply => bytecode::ReduceOperation::Multiply,
            mir::TensorReduceOperator::Min => bytecode::ReduceOperation::Minimum,
            mir::TensorReduceOperator::Max => bytecode::ReduceOperation::Maximum,
            mir::TensorReduceOperator::And => bytecode::ReduceOperation::And,
            mir::TensorReduceOperator::Or => bytecode::ReduceOperation::Or,
            mir::TensorReduceOperator::Xor => bytecode::ReduceOperation::Xor,
        }
    }

    /// Return one bytecode tensor scatter operation.
    fn scatter_operation(mode: mir::TensorScatterMode) -> bytecode::ScatterOperation {
        match mode {
            mir::TensorScatterMode::Replace => bytecode::ScatterOperation::Replace,
            mir::TensorScatterMode::Add => bytecode::ScatterOperation::Add,
            mir::TensorScatterMode::Multiply => bytecode::ScatterOperation::Multiply,
            mir::TensorScatterMode::Min => bytecode::ScatterOperation::Minimum,
            mir::TensorScatterMode::Max => bytecode::ScatterOperation::Maximum,
            mir::TensorScatterMode::And => bytecode::ScatterOperation::And,
            mir::TensorScatterMode::Or => bytecode::ScatterOperation::Or,
            mir::TensorScatterMode::Xor => bytecode::ScatterOperation::Xor,
        }
    }

    /// Return one bytecode numeric conversion mode.
    fn convert_mode(mode: mir::ConvertMode) -> bytecode::ConvertMode {
        match mode {
            mir::ConvertMode::Exact => bytecode::ConvertMode::Exact,
            mir::ConvertMode::RoundTiesEven => bytecode::ConvertMode::RoundTiesEven,
            mir::ConvertMode::RoundTowardZero => bytecode::ConvertMode::RoundTowardZero,
            mir::ConvertMode::RoundFloor => bytecode::ConvertMode::RoundFloor,
            mir::ConvertMode::RoundCeil => bytecode::ConvertMode::RoundCeil,
            mir::ConvertMode::Saturate => bytecode::ConvertMode::Saturate,
        }
    }

    /// Build one bytecode assembly diagnostic.
    fn bytecode_error(&self, error: bytecode::Error) -> EmitError {
        self.internal(&error.to_string())
    }

    /// Build one internal tensor emission diagnostic.
    fn internal(&self, message: &str) -> EmitError {
        ObjectEmitter::internal(self.module, message)
    }
}
