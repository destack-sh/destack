use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use super::call::format_call;
use super::value::{
    format_constant_for_type, format_function_id, format_global_id, format_type_id,
};

use crate::{
    AtomicAccess, CompareExchangeAccess, ExecutionScope, FenceAccess, FormatMirNode, FunctionId,
    GlobalId, Instruction, LocalNodeId, MirFormatter, StorageSet, TensorImmediate,
    TensorImmediateId, Value,
};

impl<'a> FormatMirNode<'a, Instruction> for Instruction {
    fn format_node(
        &self,
        _id: LocalNodeId<Instruction>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        match self {
            Instruction::Error => Err(FormatError::SyntaxError {
                message: "cannot format recovered MIR instruction",
            }),

            Instruction::Const { destination, value } => {
                let destination_type = typed_destination_type(*destination, f)?;
                format_typed_destination(*destination, f)?;
                write!(f, [space(), token("="), space()])?;
                format_constant_for_type(value, destination_type, f)
            }

            Instruction::Binary {
                destination,
                operator,
                left,
                right,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token(operator.to_str()),
                        space(),
                        left,
                        token(","),
                        space(),
                        right
                    ]
                )
            }

            Instruction::Unary {
                destination,
                operator,
                argument,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token(operator.to_str()),
                        space(),
                        argument
                    ]
                )
            }

            Instruction::Cast {
                destination,
                operator,
                argument,
                to_type,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token(operator.to_str()),
                        space(),
                        argument,
                        space(),
                        token("->"),
                        space(),
                        to_type
                    ]
                )
            }

            Instruction::Select {
                destination,
                condition,
                then_value,
                else_value,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("select"),
                        space(),
                        condition,
                        token(","),
                        space(),
                        then_value,
                        token(","),
                        space(),
                        else_value
                    ]
                )
            }

            Instruction::LocalGet { destination, local } => {
                let local_index = f.context().local_index(*local);
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("local.get"),
                        space(),
                        copied_text(&format!("l{local_index}"))
                    ]
                )
            }

            Instruction::LocalAddr {
                destination, local, ..
            } => {
                let local_index = f.context().local_index(*local);
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("local.address"),
                        space(),
                        copied_text(&format!("l{local_index}"))
                    ]
                )
            }

            Instruction::LocalSet { local, value } => {
                let local_index = f.context().local_index(*local);
                write!(
                    f,
                    [
                        token("local.set"),
                        space(),
                        copied_text(&format!("l{local_index}")),
                        token(","),
                        space(),
                        value
                    ]
                )
            }

            Instruction::GlobalAddr {
                destination,
                global,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("global.address"),
                        space()
                    ]
                )?;
                format_global_reference(*global, f)?;
                Ok(())
            }

            Instruction::FunctionAddr {
                destination,
                function,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("function.address"),
                        space()
                    ]
                )?;
                format_function_reference(*function, f)
            }
            Instruction::FunctionBind {
                destination,
                function,
                environment,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("function.bind"),
                        space()
                    ]
                )?;
                format_function_reference(*function, f)?;
                write!(f, [token(","), space(), environment])
            }
            Instruction::FunctionPointer {
                destination,
                function,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("function.pointer"),
                        space()
                    ]
                )?;
                write!(f, [function])
            }
            Instruction::FunctionEnvironment {
                destination,
                function,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("function.environment"),
                        space()
                    ]
                )?;
                write!(f, [function])
            }
            Instruction::FunctionEnvironmentCurrent { destination } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("function.environment.current")
                    ]
                )
            }

            Instruction::Load {
                destination,
                pointer,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("load"),
                        space(),
                        pointer
                    ]
                )
            }

            Instruction::Store { pointer, value } => {
                write!(
                    f,
                    [token("store"), space(), pointer, token(","), space(), value]
                )
            }

            Instruction::Aggregate {
                destination,
                values,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [space(), token("="), space(), token("aggregate"), space()]
                )?;
                let args = f.context().tree.get_values(*values);
                format_value_list(args, f)
            }

            Instruction::FieldGet {
                destination,
                aggregate,
                field,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("field.get"),
                        space(),
                        aggregate,
                        token(","),
                        space(),
                        copied_text(&field.to_string())
                    ]
                )
            }

            Instruction::FieldSet {
                destination,
                aggregate,
                field,
                value,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("field.set"),
                        space(),
                        aggregate,
                        token(","),
                        space(),
                        copied_text(&field.to_string()),
                        token(","),
                        space(),
                        value
                    ]
                )
            }

            Instruction::FieldAddr {
                destination,
                aggregate,
                field,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("field.address"),
                        space(),
                        aggregate,
                        token(","),
                        space(),
                        copied_text(&field.to_string())
                    ]
                )
            }

            Instruction::ElementGet {
                destination,
                aggregate,
                index,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("element.get"),
                        space(),
                        aggregate,
                        token(","),
                        space(),
                        copied_text(&index.to_string())
                    ]
                )
            }

            Instruction::ElementSet {
                destination,
                aggregate,
                index,
                value,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("element.set"),
                        space(),
                        aggregate,
                        token(","),
                        space(),
                        copied_text(&index.to_string()),
                        token(","),
                        space(),
                        value
                    ]
                )
            }

            Instruction::ElementAddr {
                destination,
                base,
                index,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("element.address"),
                        space(),
                        base,
                        token(","),
                        space(),
                        index
                    ]
                )
            }

            Instruction::SliceView {
                destination,
                source,
                start,
                length,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("slice.view"),
                        space(),
                        source,
                        token(","),
                        space(),
                        start,
                        token(","),
                        space(),
                        length
                    ]
                )
            }

            Instruction::SliceLength { destination, slice } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("slice.length"),
                        space(),
                        slice
                    ]
                )
            }

            Instruction::DynamicBind {
                destination,
                payload,
                concrete,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("dynamic.bind"),
                        space(),
                        payload,
                        token(","),
                        space(),
                        concrete
                    ]
                )
            }

            Instruction::DynamicPayload {
                destination,
                dynamic,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("dynamic.payload"),
                        space(),
                        dynamic
                    ]
                )
            }

            Instruction::DynamicType {
                destination,
                dynamic,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("dynamic.type"),
                        space(),
                        dynamic
                    ]
                )
            }

            Instruction::VectorSplat { destination, value } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("vector.splat"),
                        space(),
                        value
                    ]
                )
            }

            Instruction::VectorExtract {
                destination,
                vector,
                index,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("vector.extract"),
                        space(),
                        vector,
                        token(","),
                        space(),
                        index
                    ]
                )
            }

            Instruction::VectorInsert {
                destination,
                vector,
                index,
                value,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("vector.insert"),
                        space(),
                        vector,
                        token(","),
                        space(),
                        index,
                        token(","),
                        space(),
                        value
                    ]
                )
            }

            Instruction::VectorShuffle {
                destination,
                left,
                right,
                mask,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("vector.shuffle"),
                        space(),
                        left,
                        token(","),
                        space(),
                        right,
                        token(","),
                        space()
                    ]
                )?;
                let mask = f.context().tree.get_indices(*mask);
                format_u32_bracket_list(mask, f)
            }
            Instruction::VectorSelect {
                destination,
                mask,
                then_value,
                else_value,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("vector.select"),
                        space(),
                        mask,
                        token(","),
                        space(),
                        then_value,
                        token(","),
                        space(),
                        else_value
                    ]
                )
            }

            Instruction::VectorReduce {
                destination,
                operator,
                vector,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("vector.reduce"),
                        space(),
                        token(operator.to_str()),
                        token(","),
                        space(),
                        vector
                    ]
                )
            }
            Instruction::VectorCompare {
                destination,
                operator,
                left,
                right,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("vector.compare"),
                        space(),
                        token(operator.to_str()),
                        token(","),
                        space(),
                        left,
                        token(","),
                        space(),
                        right
                    ]
                )
            }
            Instruction::VectorConvert {
                destination,
                mode,
                vector,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("vector.convert"),
                        space(),
                        token(mode.to_str()),
                        token(","),
                        space(),
                        vector
                    ]
                )
            }

            Instruction::TensorSplat { destination, value } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tensor.splat"),
                        space(),
                        value
                    ]
                )
            }

            Instruction::TensorLoad {
                destination,
                view,
                indices,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tensor.load"),
                        space(),
                        view,
                        token(","),
                        space()
                    ]
                )?;
                let args = f.context().tree.get_values(*indices);
                format_value_bracket_list(args, f)
            }
            Instruction::TensorExtract {
                destination,
                tensor,
                indices,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tensor.extract"),
                        space(),
                        tensor,
                        token(","),
                        space()
                    ]
                )?;
                let args = f.context().tree.get_values(*indices);
                format_value_bracket_list(args, f)
            }

            Instruction::TensorStore {
                view,
                indices,
                value,
            } => {
                write!(
                    f,
                    [token("tensor.store"), space(), view, token(","), space()]
                )?;
                let args = f.context().tree.get_values(*indices);
                format_value_bracket_list(args, f)?;
                write!(f, [token(","), space(), value])
            }

            Instruction::TensorFill { view, value } => {
                write!(
                    f,
                    [
                        token("tensor.fill"),
                        space(),
                        view,
                        token(","),
                        space(),
                        value
                    ]
                )
            }

            Instruction::TensorCopy { target, source } => {
                write!(
                    f,
                    [
                        token("tensor.copy"),
                        space(),
                        target,
                        token(","),
                        space(),
                        source
                    ]
                )
            }

            Instruction::TensorReshape {
                destination,
                tensor,
                shape,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tensor.reshape"),
                        space(),
                        tensor
                    ]
                )?;
                let args = f.context().tree.get_values(*shape);
                if !args.is_empty() {
                    write!(f, [token(","), space()])?;
                    format_named_value_group("shape", args, f)?;
                }
                Ok(())
            }

            Instruction::TensorBroadcast {
                destination,
                tensor,
                dimensions,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tensor.broadcast"),
                        space(),
                        tensor,
                        token(","),
                        space()
                    ]
                )?;
                let dimensions = f.context().tree.get_indices(*dimensions);
                format_named_u32_group("dimensions", dimensions, f)
            }

            Instruction::TensorTranspose {
                destination,
                tensor,
                permutation,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tensor.transpose"),
                        space(),
                        tensor,
                        token(","),
                        space()
                    ]
                )?;
                let permutation = f.context().tree.get_indices(*permutation);
                format_named_u32_group("permutation", permutation, f)
            }
            Instruction::TensorCast {
                destination,
                tensor,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tensor.cast"),
                        space(),
                        tensor
                    ]
                )
            }
            Instruction::TensorView {
                destination,
                view,
                arguments,
                offsets_count,
                sizes_count,
                strides_count,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tensor.view"),
                        space(),
                        view,
                        token(","),
                        space()
                    ]
                )?;
                let args = f.context().tree.get_values(*arguments);
                let (offsets, sizes, strides) =
                    split_tensor_ranges(args, *offsets_count, *sizes_count, *strides_count);
                format_named_value_group("offsets", offsets, f)?;
                write!(f, [token(","), space()])?;
                format_named_value_group("sizes", sizes, f)?;
                write!(f, [token(","), space()])?;
                format_named_value_group("strides", strides, f)
            }

            Instruction::TensorSlice {
                destination,
                tensor,
                arguments,
                offsets_count,
                sizes_count,
                strides_count,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tensor.slice"),
                        space(),
                        tensor,
                        token(","),
                        space()
                    ]
                )?;
                let args = f.context().tree.get_values(*arguments);
                let (offsets, sizes, strides) =
                    split_tensor_ranges(args, *offsets_count, *sizes_count, *strides_count);
                format_named_value_group("offsets", offsets, f)?;
                write!(f, [token(","), space()])?;
                format_named_value_group("sizes", sizes, f)?;
                write!(f, [token(","), space()])?;
                format_named_value_group("strides", strides, f)
            }

            Instruction::TensorPad {
                destination,
                tensor,
                arguments,
                low_count,
                high_count,
                interior_count,
                value,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tensor.pad"),
                        space(),
                        tensor,
                        token(","),
                        space()
                    ]
                )?;
                format_named_value_group("value", &[*value], f)?;
                write!(f, [token(","), space()])?;
                let args = f.context().tree.get_values(*arguments);
                let (low, high, interior) =
                    split_tensor_padding(args, *low_count, *high_count, *interior_count);
                format_named_value_group("low", low, f)?;
                write!(f, [token(","), space()])?;
                format_named_value_group("high", high, f)?;
                write!(f, [token(","), space()])?;
                format_named_value_group("interior", interior, f)
            }

            Instruction::TensorConcat {
                destination,
                tensors,
                axis,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tensor.concat"),
                        space()
                    ]
                )?;
                let args = f.context().tree.get_values(*tensors);
                format_named_value_group("tensors", args, f)?;
                write!(
                    f,
                    [
                        token(","),
                        space(),
                        token("axis"),
                        token("("),
                        copied_text(&axis.to_string()),
                        token(")")
                    ]
                )
            }
            Instruction::TensorCompare {
                destination,
                operator,
                left,
                right,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tensor.compare"),
                        space(),
                        token(operator.to_str()),
                        token(","),
                        space(),
                        left,
                        token(","),
                        space(),
                        right
                    ]
                )
            }
            Instruction::TensorSelect {
                destination,
                mask,
                then_value,
                else_value,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tensor.select"),
                        space(),
                        mask,
                        token(","),
                        space(),
                        then_value,
                        token(","),
                        space(),
                        else_value
                    ]
                )
            }

            Instruction::TensorReduce {
                destination,
                operator,
                tensor,
                initial,
                axes,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tensor.reduce"),
                        space(),
                        token(operator.to_str()),
                        token(","),
                        space(),
                        tensor,
                        token(","),
                        space(),
                        initial,
                        token(","),
                        space()
                    ]
                )?;
                let axes = f.context().tree.get_indices(*axes);
                format_named_u32_group("axes", axes, f)
            }

            Instruction::TensorIndexReduce {
                destination,
                operator,
                tensor,
                axis,
                tie_break,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tensor.indexReduce"),
                        space(),
                        token(operator.to_str()),
                        token(","),
                        space(),
                        tensor,
                        token(","),
                        space()
                    ]
                )?;
                format_named_u32_group("axis", &[*axis], f)?;
                write!(
                    f,
                    [
                        token(","),
                        space(),
                        token("tieBreak"),
                        token("("),
                        token(tie_break.to_str()),
                        token(")")
                    ]
                )
            }

            Instruction::TensorDot {
                destination,
                left,
                right,
                immediate,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tensor.dot"),
                        space(),
                        left,
                        token(","),
                        space(),
                        right,
                        token(","),
                        space()
                    ]
                )?;
                format_tensor_dot_immediate(*immediate, f)
            }

            Instruction::TensorConvolution {
                destination,
                input,
                kernel,
                immediate,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tensor.convolution"),
                        space(),
                        input,
                        token(","),
                        space(),
                        kernel,
                        token(","),
                        space()
                    ]
                )?;
                format_tensor_convolution_immediate(*immediate, f)
            }

            Instruction::TensorGather {
                destination,
                operand,
                indices,
                immediate,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tensor.gather"),
                        space(),
                        operand,
                        token(","),
                        space(),
                        indices,
                        token(","),
                        space()
                    ]
                )?;
                format_tensor_gather_immediate(*immediate, f)
            }

            Instruction::TensorScatter {
                destination,
                operand,
                indices,
                updates,
                immediate,
                mode,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tensor.scatter"),
                        space(),
                        operand,
                        token(","),
                        space(),
                        indices,
                        token(","),
                        space(),
                        updates,
                        token(","),
                        space()
                    ]
                )?;
                format_tensor_scatter_immediate(*immediate, f)?;
                write!(
                    f,
                    [
                        token(","),
                        space(),
                        token("mode"),
                        token("("),
                        token(mode.to_str()),
                        token(")")
                    ]
                )
            }

            Instruction::TensorConvert {
                destination,
                mode,
                tensor,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tensor.convert"),
                        space(),
                        token(mode.to_str()),
                        token(","),
                        space(),
                        tensor
                    ]
                )
            }

            Instruction::Call { destination, call } => {
                if let Some(dst) = destination {
                    format_typed_destination(*dst, f)?;
                    write!(f, [space(), token("="), space()])?;
                }

                format_call(
                    call,
                    ["call", "call.indirect", "call.virtual", "call.dynamic"],
                    f,
                )
            }

            Instruction::Drop { value } => {
                write!(f, [token("drop"), space(), value])
            }

            Instruction::NewZeroed {
                destination,
                layout,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("new.zeroed"),
                        space(),
                        layout
                    ]
                )
            }

            Instruction::NewUninit {
                destination,
                layout,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("new.uninit"),
                        space(),
                        layout
                    ]
                )
            }

            Instruction::NewComplete {
                destination, value, ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("new.complete"),
                        space(),
                        value
                    ]
                )
            }

            Instruction::NewSliceZeroed {
                destination,
                element,
                length,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("new.slice.zeroed"),
                        space(),
                        element,
                        token(","),
                        space(),
                        length
                    ]
                )
            }

            Instruction::NewSliceUninit {
                destination,
                element,
                length,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("new.slice.uninit"),
                        space(),
                        element,
                        token(","),
                        space(),
                        length
                    ]
                )
            }

            Instruction::Free { value } => write!(f, [token("free"), space(), value]),

            Instruction::FrameAllocZeroed {
                destination,
                layout,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("frame.alloc.zeroed"),
                        space(),
                        layout
                    ]
                )
            }

            Instruction::FrameAllocUninit {
                destination,
                layout,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("frame.alloc.uninit"),
                        space(),
                        layout
                    ]
                )
            }

            Instruction::Pin {
                destination, value, ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [space(), token("="), space(), token("pin"), space(), value]
                )
            }

            Instruction::Unpin { value } => write!(f, [token("unpin"), space(), value]),

            Instruction::BarrierWrite {
                object,
                offset,
                byte_len,
            } => write!(
                f,
                [
                    token("barrier.write"),
                    space(),
                    object,
                    token(","),
                    space(),
                    offset,
                    token(","),
                    space(),
                    byte_len
                ]
            ),

            Instruction::AtomicLoad {
                destination,
                pointer,
                access,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("atomic.load"),
                        space(),
                        pointer
                    ]
                )?;
                format_atomic_access(*access, f)
            }

            Instruction::AtomicStore {
                pointer,
                value,
                access,
            } => {
                write!(
                    f,
                    [
                        token("atomic.store"),
                        space(),
                        pointer,
                        token(","),
                        space(),
                        value
                    ]
                )?;
                format_atomic_access(*access, f)
            }

            Instruction::AtomicCompareExchange {
                destination,
                pointer,
                expected,
                new_value,
                is_weak,
                access,
            } => {
                format_typed_destination(*destination, f)?;
                write!(f, [space(), token("="), space()])?;
                let opcode = if *is_weak {
                    "atomic.cas.weak"
                } else {
                    "atomic.cas"
                };
                write!(
                    f,
                    [
                        token(opcode),
                        space(),
                        pointer,
                        token(","),
                        space(),
                        expected,
                        token(","),
                        space(),
                        new_value
                    ]
                )?;
                format_atomic_compare_exchange_access(*access, f)
            }

            Instruction::AtomicRmw {
                destination,
                operator,
                pointer,
                value,
                access,
            } => {
                format_typed_destination(*destination, f)?;
                write!(f, [space(), token("="), space(), token("atomic.rmw.")])?;
                write!(
                    f,
                    [
                        token(operator.to_str()),
                        space(),
                        pointer,
                        token(","),
                        space(),
                        value
                    ]
                )?;
                format_atomic_access(*access, f)
            }

            Instruction::AtomicFence { access } => {
                write!(f, [token("atomic.fence")])?;
                format_fence_access(*access, f)
            }

            Instruction::Assume { condition } => {
                write!(f, [token("assume"), space(), condition])
            }

            Instruction::ProfileIncrement { counter } => {
                write!(
                    f,
                    [
                        token("profile.increment"),
                        space(),
                        copied_text(&format!("counter({})", counter.0))
                    ]
                )
            }

            Instruction::ProfileSample { counter, value } => {
                write!(
                    f,
                    [
                        token("profile.sample"),
                        space(),
                        copied_text(&format!("counter({})", counter.0)),
                        token(","),
                        space(),
                        value
                    ]
                )
            }

            Instruction::Breakpoint => write!(f, [token("breakpoint")]),

            Instruction::Intrinsic {
                destination,
                intrinsic,
                arguments,
            } => {
                if let Some(dst) = destination {
                    format_typed_destination(*dst, f)?;
                    write!(f, [space(), token("="), space()])?;
                }
                write!(f, [token("intrinsic."), token(intrinsic.to_str())])?;
                let args = f.context().tree.get_values(*arguments);
                format_intrinsic_args(args, f)
            }
        }
    }
}

fn format_typed_destination<'a>(
    destination: Value,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let ty = f
        .context()
        .value_type(destination)
        .ok_or(FormatError::SyntaxError {
            message: "missing value type for instruction destination",
        })?;
    write!(f, [destination, token(":"), space()])?;
    format_type_id(ty, f)
}

fn typed_destination_type<'a>(
    destination: Value,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<LocalNodeId<crate::Type>> {
    f.context()
        .value_type(destination)
        .ok_or(FormatError::SyntaxError {
            message: "missing value type for instruction destination",
        })
}

/// Format a function reference.
fn format_function_reference<'a>(
    function_id: FunctionId,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    format_function_id(function_id, f)
}

/// Format a global reference.
fn format_global_reference<'a>(
    global_id: GlobalId,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    format_global_id(global_id, f)
}

/// Format a parenthesized, comma-separated list of values.
fn format_value_list<'a>(values: &[Value], f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    write!(f, [token("(")])?;
    for (i, val) in values.iter().enumerate() {
        if i > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [val])?;
    }
    write!(f, [token(")")])
}

/// Format a bracketed, comma-separated list of values.
fn format_value_bracket_list<'a>(
    values: &[Value],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token("[")])?;
    for (i, val) in values.iter().enumerate() {
        if i > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [val])?;
    }
    write!(f, [token("]")])
}

/// Format a bracketed, comma-separated list of u32 values.
fn format_u32_bracket_list<'a>(values: &[u32], f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    write!(f, [token("[")])?;
    for (i, val) in values.iter().enumerate() {
        if i > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [copied_text(&val.to_string())])?;
    }
    write!(f, [token("]")])
}

/// Format a named value list like `name=[v0, v1]`.
fn format_named_value_group<'a>(
    name: &'static str,
    values: &[Value],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token(name), token("(")])?;
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [*value])?;
    }
    write!(f, [token(")")])
}

/// Format a named u32 group like `name(0, 1)`.
fn format_named_u32_group<'a>(
    name: &'static str,
    values: &[u32],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token(name), token("(")])?;
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [copied_text(&value.to_string())])?;
    }
    write!(f, [token(")")])
}

/// Format tensor dot dimension numbers.
fn format_tensor_dot_immediate<'a>(
    immediate: TensorImmediateId,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let TensorImmediate::Dot {
        lhs_batch,
        rhs_batch,
        lhs_contracting,
        rhs_contracting,
    } = f.context().tree.get_tensor_immediate(immediate)
    else {
        return Err(FormatError::SyntaxError {
            message: "tensor.dot instruction has non-dot immediate",
        });
    };

    write!(f, [token("dims"), token("(")])?;
    format_named_u32_group("lhsBatch", f.context().tree.get_indices(*lhs_batch), f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_group("rhsBatch", f.context().tree.get_indices(*rhs_batch), f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_group(
        "lhsContract",
        f.context().tree.get_indices(*lhs_contracting),
        f,
    )?;
    write!(f, [token(","), space()])?;
    format_named_u32_group(
        "rhsContract",
        f.context().tree.get_indices(*rhs_contracting),
        f,
    )?;
    write!(f, [token(")")])
}

/// Format tensor convolution dimension numbers.
fn format_tensor_convolution_immediate<'a>(
    immediate: TensorImmediateId,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let TensorImmediate::Convolution {
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
    } = f.context().tree.get_tensor_immediate(immediate)
    else {
        return Err(FormatError::SyntaxError {
            message: "tensor.convolution instruction has non-convolution immediate",
        });
    };

    write!(f, [token("dims"), token("(")])?;
    format_named_u32_single("inputBatch", *input_batch, f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_single("inputFeature", *input_feature, f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_group(
        "inputSpatial",
        f.context().tree.get_indices(*input_spatial),
        f,
    )?;
    write!(f, [token(","), space()])?;
    format_named_u32_single("kernelInputFeature", *kernel_input_feature, f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_single("kernelOutputFeature", *kernel_output_feature, f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_group(
        "kernelSpatial",
        f.context().tree.get_indices(*kernel_spatial),
        f,
    )?;
    write!(f, [token(","), space()])?;
    format_named_u32_single("outputBatch", *output_batch, f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_single("outputFeature", *output_feature, f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_group(
        "outputSpatial",
        f.context().tree.get_indices(*output_spatial),
        f,
    )?;
    write!(f, [token(")")])?;

    write!(f, [token(","), space(), token("window"), token("(")])?;
    format_named_u64_group("strides", f.context().tree.get_extents(*strides), f)?;
    write!(f, [token(","), space()])?;
    format_named_u64_group("paddingLow", f.context().tree.get_extents(*padding_low), f)?;
    write!(f, [token(","), space()])?;
    format_named_u64_group(
        "paddingHigh",
        f.context().tree.get_extents(*padding_high),
        f,
    )?;
    write!(f, [token(","), space()])?;
    format_named_u64_group(
        "lhsDilation",
        f.context().tree.get_extents(*lhs_dilation),
        f,
    )?;
    write!(f, [token(","), space()])?;
    format_named_u64_group(
        "rhsDilation",
        f.context().tree.get_extents(*rhs_dilation),
        f,
    )?;
    write!(f, [token(","), space()])?;
    format_named_flag_group(
        "windowReversal",
        f.context().tree.get_flags(*window_reversal),
        f,
    )?;
    write!(f, [token(")")])?;

    format_tensor_convolution_groups(*feature_group_count, *batch_group_count, f)
}

/// Format tensor convolution groups.
fn format_tensor_convolution_groups<'a>(
    feature_group_count: u32,
    batch_group_count: u32,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token(","), space(), token("groups"), token("(")])?;
    write!(
        f,
        [
            token("feature"),
            token("("),
            copied_text(&feature_group_count.to_string()),
            token(")"),
            token(","),
            space(),
            token("batch"),
            token("("),
            copied_text(&batch_group_count.to_string()),
            token(")")
        ]
    )?;
    write!(f, [token(")")])
}

/// Format tensor gather dimension numbers.
fn format_tensor_gather_immediate<'a>(
    immediate: TensorImmediateId,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let TensorImmediate::Gather {
        offset_dims,
        collapsed_slice_dims,
        start_index_map,
        index_vector_dim,
        slice_sizes,
    } = f.context().tree.get_tensor_immediate(immediate)
    else {
        return Err(FormatError::SyntaxError {
            message: "tensor.gather instruction has non-gather immediate",
        });
    };

    write!(f, [token("dims"), token("(")])?;
    format_named_u32_group("offsetDims", f.context().tree.get_indices(*offset_dims), f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_group(
        "collapsedSliceDims",
        f.context().tree.get_indices(*collapsed_slice_dims),
        f,
    )?;
    write!(f, [token(","), space()])?;
    format_named_u32_group(
        "startIndexMap",
        f.context().tree.get_indices(*start_index_map),
        f,
    )?;
    write!(f, [token(","), space()])?;
    format_named_u32_single("indexVectorDim", *index_vector_dim, f)?;
    write!(f, [token(")")])?;

    write!(f, [token(","), space()])?;
    format_named_u32_group("sliceSizes", f.context().tree.get_indices(*slice_sizes), f)
}

/// Format tensor scatter dimension numbers.
fn format_tensor_scatter_immediate<'a>(
    immediate: TensorImmediateId,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let TensorImmediate::Scatter {
        update_window_dims,
        inserted_window_dims,
        scatter_dims_to_operand_dims,
        index_vector_dim,
    } = f.context().tree.get_tensor_immediate(immediate)
    else {
        return Err(FormatError::SyntaxError {
            message: "tensor.scatter instruction has non-scatter immediate",
        });
    };

    write!(f, [token("dims"), token("(")])?;
    format_named_u32_group(
        "updateWindowDims",
        f.context().tree.get_indices(*update_window_dims),
        f,
    )?;
    write!(f, [token(","), space()])?;
    format_named_u32_group(
        "insertedWindowDims",
        f.context().tree.get_indices(*inserted_window_dims),
        f,
    )?;
    write!(f, [token(","), space()])?;
    format_named_u32_group(
        "scatterDimsToOperandDims",
        f.context().tree.get_indices(*scatter_dims_to_operand_dims),
        f,
    )?;
    write!(f, [token(","), space()])?;
    format_named_u32_single("indexVectorDim", *index_vector_dim, f)?;
    write!(f, [token(")")])
}

/// Format a named u32 value like `name(0)`.
fn format_named_u32_single<'a>(
    name: &'static str,
    value: u32,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(
        f,
        [
            token(name),
            token("("),
            copied_text(&value.to_string()),
            token(")")
        ]
    )
}

/// Format a named u64 group like `name(0, 1)`.
fn format_named_u64_group<'a>(
    name: &'static str,
    values: &[u64],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token(name), token("(")])?;
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [copied_text(&value.to_string())])?;
    }
    write!(f, [token(")")])
}

/// Format a named flag group like `name(true, false)`.
fn format_named_flag_group<'a>(
    name: &'static str,
    values: &[u8],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token(name), token("(")])?;
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [token(if *value != 0 { "true" } else { "false" })])?;
    }
    write!(f, [token(")")])
}

/// Split packed tensor range arguments.
fn split_tensor_ranges(
    values: &[Value],
    offsets_count: u16,
    sizes_count: u16,
    strides_count: u16,
) -> (&[Value], &[Value], &[Value]) {
    let offsets_end = offsets_count as usize;
    let sizes_end = offsets_end + sizes_count as usize;
    let strides_end = sizes_end + strides_count as usize;

    let offsets = values.get(..offsets_end).unwrap_or(&[]);
    let sizes = values.get(offsets_end..sizes_end).unwrap_or(&[]);
    let strides = values.get(sizes_end..strides_end).unwrap_or(&[]);

    (offsets, sizes, strides)
}

/// Split packed tensor padding arguments.
fn split_tensor_padding(
    values: &[Value],
    low_count: u16,
    high_count: u16,
    interior_count: u16,
) -> (&[Value], &[Value], &[Value]) {
    let low_end = low_count as usize;
    let high_end = low_end + high_count as usize;
    let interior_end = high_end + interior_count as usize;

    let low = values.get(..low_end).unwrap_or(&[]);
    let high = values.get(low_end..high_end).unwrap_or(&[]);
    let interior = values.get(high_end..interior_end).unwrap_or(&[]);

    (low, high, interior)
}

/// Format intrinsic arguments with optional memory ordering.
fn format_intrinsic_args<'a>(values: &[Value], f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    write!(f, [token("(")])?;
    for (i, val) in values.iter().enumerate() {
        if i > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [val])?;
    }
    write!(f, [token(")")])
}

/// Format one atomic access suffix.
fn format_atomic_access<'a>(
    access: AtomicAccess,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token(","), space(), token(access.ordering.to_str())])?;

    format_atomic_context(access, f)
}

/// Format one compare exchange access suffix.
fn format_atomic_compare_exchange_access<'a>(
    access: CompareExchangeAccess,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    format_atomic_access(access.success, f)?;

    if access.failure_ordering != access.success.ordering {
        write!(
            f,
            [
                token(","),
                space(),
                token("failure"),
                token("("),
                token(access.failure_ordering.to_str()),
                token(")")
            ]
        )?;
    }

    Ok(())
}

/// Format one fence access suffix.
fn format_fence_access<'a>(access: FenceAccess, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    write!(f, [space(), token(access.ordering.to_str())])?;
    format_fence_context(access, f)
}

/// Format non-default execution scope.
fn format_atomic_context<'a>(
    access: AtomicAccess,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    if access.scope != ExecutionScope::default() {
        write!(
            f,
            [
                token(","),
                space(),
                token("scope"),
                token("("),
                token(access.scope.to_str()),
                token(")")
            ]
        )?;
    }

    Ok(())
}

/// Format non-default fence scope and storage.
fn format_fence_context<'a>(access: FenceAccess, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    if access.scope != ExecutionScope::default() {
        write!(
            f,
            [
                token(","),
                space(),
                token("scope"),
                token("("),
                token(access.scope.to_str()),
                token(")")
            ]
        )?;
    }

    if access.storage != StorageSet::ANY {
        write!(
            f,
            [
                token(","),
                space(),
                token("storage"),
                token("("),
                copied_text(&format_storage_set(access.storage)),
                token(")")
            ]
        )?;
    }

    Ok(())
}

/// Collect named storage regions in formatting order.
fn collect_effect_space_names(spaces: StorageSet) -> Vec<&'static str> {
    // special cases for named sets
    if spaces == StorageSet::NONE {
        return vec!["none"];
    }
    if spaces == StorageSet::ANY {
        return vec!["any"];
    }

    // collect named spaces in canonical order
    let mut names = Vec::new();
    let ordered = [
        ("local", StorageSet::LOCAL),
        ("shared", StorageSet::SHARED),
        ("frame", StorageSet::FRAME),
        ("static", StorageSet::STATIC),
        ("device", StorageSet::DEVICE),
        ("workgroup", StorageSet::WORKGROUP),
    ];
    for (name, set) in ordered {
        if spaces.contains(set) {
            names.push(name);
        }
    }

    names
}

/// Format one storage set.
fn format_storage_set(storage: StorageSet) -> String {
    let names = collect_effect_space_names(storage);

    if names.len() == 1 {
        names[0].to_string()
    } else {
        format!("[{}]", names.join(", "))
    }
}
