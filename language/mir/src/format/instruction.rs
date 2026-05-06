use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    AtomicScope, FormatMirNode, FunctionReference, GlobalReference, Instruction, LocalNodeId,
    MemoryFlags, MemoryOrdering, MemoryScope, MemorySpaceSet, MirFormatter,
    TensorConvolutionDimensionNumbers, TensorConvolutionWindow, TensorDotDimensionNumbers,
    TensorGatherDimensionNumbers, TensorScatterDimensionNumbers, TypeReference, ValueReference,
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
                format_typed_destination(*destination, f)?;
                write!(f, [space(), token("="), space(), value])
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
                let crate::LocalReference::Local(local) = *local else {
                    return write!(f, [token("<error>")]);
                };
                let local_index = f.context().local_index(local);
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("local.get"),
                        space(),
                        text(&format!("local{local_index}"))
                    ]
                )
            }

            Instruction::LocalAddr {
                destination, local, ..
            } => {
                let crate::LocalReference::Local(local) = *local else {
                    return write!(f, [token("<error>")]);
                };
                let local_index = f.context().local_index(local);
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("local.address"),
                        space(),
                        text(&format!("local{local_index}"))
                    ]
                )
            }

            Instruction::LocalSet { local, value } => {
                let crate::LocalReference::Local(local) = *local else {
                    return write!(f, [token("<error>")]);
                };
                let local_index = f.context().local_index(local);
                write!(
                    f,
                    [
                        token("local.set"),
                        space(),
                        text(&format!("local{local_index}")),
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
            Instruction::CallableBind {
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
                        token("callable.bind"),
                        space()
                    ]
                )?;
                format_function_reference(*function, f)?;
                write!(f, [token(","), space(), environment])
            }
            Instruction::CallableEnvironment { destination } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [space(), token("="), space(), token("callable.environment")]
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

            Instruction::TensorStore {
                view,
                indices,
                value,
            } => {
                write!(
                    f,
                    [token("tensor.store"), space(), view, token(","), space()]
                )?;
                let args = f.context().tree.get_arguments(*indices);
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

            Instruction::Drop { value } => write!(f, [token("drop"), space(), value]),

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

            Instruction::Assume { condition } => {
                write!(f, [token("assume"), space(), condition])
            }

            Instruction::FieldGet {
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
                        token("field.get"),
                        space(),
                        aggregate,
                        token(","),
                        space(),
                        text(&index.to_string())
                    ]
                )
            }

            Instruction::FieldAddr {
                destination,
                aggregate,
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
                        token("field.address"),
                        space(),
                        aggregate,
                        token(","),
                        space(),
                        text(&index.to_string())
                    ]
                )
            }

            Instruction::FieldSet {
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
                        token("field.set"),
                        space(),
                        aggregate,
                        token(","),
                        space(),
                        text(&index.to_string()),
                        token(","),
                        space(),
                        value
                    ]
                )
            }

            Instruction::ElementGet {
                destination,
                array,
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
                        array,
                        token(","),
                        space(),
                        text(&index.to_string())
                    ]
                )
            }

            Instruction::ElementAddr {
                destination,
                array,
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
                        array,
                        token(","),
                        space(),
                        index
                    ]
                )
            }

            Instruction::ElementSet {
                destination,
                array,
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
                        array,
                        token(","),
                        space(),
                        text(&index.to_string()),
                        token(","),
                        space(),
                        value
                    ]
                )
            }

            Instruction::Struct {
                destination,
                ty,
                fields,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("struct"),
                        space(),
                        ty,
                        space()
                    ]
                )?;
                let args = f.context().tree.get_arguments(*fields);
                format_value_list(args, f)
            }

            Instruction::Tuple {
                destination,
                ty,
                elements,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("tuple"),
                        space(),
                        ty,
                        space()
                    ]
                )?;
                let args = f.context().tree.get_arguments(*elements);
                format_value_list(args, f)
            }

            Instruction::Array {
                destination,
                ty,
                elements,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("array"),
                        space(),
                        ty,
                        space()
                    ]
                )?;
                let args = f.context().tree.get_arguments(*elements);
                format_value_list(args, f)
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
                let args = f.context().tree.get_arguments(*indices);
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
                let args = f.context().tree.get_arguments(*indices);
                format_value_bracket_list(args, f)
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
                let args = f.context().tree.get_arguments(*shape);
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
                let args = f.context().tree.get_arguments(*arguments);
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
                let args = f.context().tree.get_arguments(*arguments);
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
                let args = f.context().tree.get_arguments(*arguments);
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
                let args = f.context().tree.get_arguments(*tensors);
                format_named_value_group("tensors", args, f)?;
                write!(
                    f,
                    [
                        token(","),
                        space(),
                        token("axis"),
                        token("("),
                        text(&axis.to_string()),
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
                format_named_u32_group("axes", axes, f)
            }

            Instruction::TensorDot {
                destination,
                left,
                right,
                dimensions,
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
                format_tensor_dot_dimensions(dimensions, f)
            }

            Instruction::TensorConvolution {
                destination,
                input,
                kernel,
                dimensions,
                window,
                feature_group_count,
                batch_group_count,
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
                format_tensor_convolution_dimensions(dimensions, f)?;
                format_tensor_convolution_window(window, f)?;
                format_tensor_convolution_groups(*feature_group_count, *batch_group_count, f)
            }

            Instruction::TensorGather {
                destination,
                operand,
                indices,
                dimensions,
                slice_sizes,
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
                format_tensor_gather_dimensions(dimensions, f)?;
                write!(f, [token(","), space()])?;
                format_named_u32_group("sliceSizes", slice_sizes, f)
            }

            Instruction::TensorScatter {
                destination,
                operand,
                indices,
                updates,
                dimensions,
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
                format_tensor_scatter_dimensions(dimensions, f)?;
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

            Instruction::Call {
                destination,
                function,
                call,
                ..
            } => {
                if let Some(dst) = destination {
                    format_typed_destination(*dst, f)?;
                    write!(f, [space(), token("="), space()])?;
                }
                write!(f, [token("call"), space()])?;
                format_function_reference(*function, f)?;
                let args = f.context().tree.get_arguments(call.arguments);
                format_value_list(args, f)?;
                format_call_signature_suffix(call.signature, f)
            }

            Instruction::CallVirtual {
                destination,
                receiver,
                call,
                declaring_type,
                slot,
                ..
            } => {
                if let Some(dst) = destination {
                    format_typed_destination(*dst, f)?;
                    write!(f, [space(), token("="), space()])?;
                }
                write!(
                    f,
                    [
                        token("call.virtual"),
                        space(),
                        receiver,
                        token(","),
                        space(),
                        declaring_type,
                        token(","),
                        space(),
                        text(&slot.0.to_string())
                    ]
                )?;
                let args = f.context().tree.get_arguments(call.arguments);
                format_value_list(args, f)?;
                format_call_signature_suffix(call.signature, f)
            }

            Instruction::CallInterface {
                destination,
                receiver,
                call,
                declaring_type,
                slot,
                ..
            } => {
                if let Some(dst) = destination {
                    format_typed_destination(*dst, f)?;
                    write!(f, [space(), token("="), space()])?;
                }
                write!(
                    f,
                    [
                        token("call.interface"),
                        space(),
                        receiver,
                        token(","),
                        space(),
                        declaring_type,
                        token(","),
                        space(),
                        text(&slot.0.to_string())
                    ]
                )?;
                let args = f.context().tree.get_arguments(call.arguments);
                format_value_list(args, f)?;
                format_call_signature_suffix(call.signature, f)
            }

            Instruction::CallIndirect {
                destination,
                callee,
                call,
                ..
            } => {
                if let Some(dst) = destination {
                    format_typed_destination(*dst, f)?;
                    write!(f, [space(), token("="), space()])?;
                }
                write!(f, [token("call.indirect"), space(), callee])?;
                let args = f.context().tree.get_arguments(call.arguments);
                format_value_list(args, f)?;
                format_call_signature_suffix(call.signature, f)
            }

            Instruction::New {
                destination,
                layout,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [space(), token("="), space(), token("new"), space(), layout]
                )
            }

            Instruction::NewSlice {
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
                        token("new.slice"),
                        space(),
                        element,
                        token(","),
                        space(),
                        length
                    ]
                )
            }

            Instruction::RawAlloc {
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
                        token("raw.alloc"),
                        space(),
                        layout
                    ]
                )
            }

            Instruction::RawFree { pointer } => {
                write!(f, [token("raw.free"), space(), pointer])
            }

            Instruction::StackAlloc {
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
                        token("stack.alloc"),
                        space(),
                        layout
                    ]
                )
            }

            Instruction::AtomicLoad {
                destination,
                pointer,
                ordering,
                scope,
                memory_scope,
                flags,
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
                format_atomic_suffix(*ordering, *scope, *memory_scope, *flags, f)
            }

            Instruction::AtomicStore {
                pointer,
                value,
                ordering,
                scope,
                memory_scope,
                flags,
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
                format_atomic_suffix(*ordering, *scope, *memory_scope, *flags, f)
            }

            Instruction::AtomicCompareExchange {
                destination,
                pointer,
                expected,
                new_value,
                is_weak,
                ordering,
                scope,
                memory_scope,
                flags,
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
                format_atomic_suffix(*ordering, *scope, *memory_scope, *flags, f)
            }

            Instruction::AtomicRmw {
                destination,
                operator,
                pointer,
                value,
                ordering,
                scope,
                memory_scope,
                flags,
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
                format_atomic_suffix(*ordering, *scope, *memory_scope, *flags, f)
            }

            Instruction::AtomicFence {
                ordering,
                scope,
                memory_scope,
                flags,
            } => {
                write!(f, [token("atomic.fence")])?;
                format_atomic_fence_suffix(*ordering, *scope, *memory_scope, *flags, f)
            }

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
                let args = f.context().tree.get_arguments(*arguments);
                format_intrinsic_args(args, f)
            }
        }
    }
}

fn format_typed_destination<'a>(
    destination: ValueReference,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let ValueReference::Value(destination) = destination else {
        return write!(
            f,
            [destination, token(":"), space(), token("<missing-type>")]
        );
    };

    let ty = f
        .context()
        .value_type(destination)
        .unwrap_or_else(|| panic!("missing value type for instruction destination"));
    write!(f, [destination, token(":"), space(), ty])
}

/// Format a function reference.
fn format_function_reference<'a>(
    function_id: FunctionReference,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [function_id])
}

/// Format a global reference.
fn format_global_reference<'a>(
    global_id: GlobalReference,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [global_id])
}

/// Format a parenthesized, comma-separated list of values.
fn format_value_list<'a>(
    values: &[ValueReference],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token("(")])?;
    for (i, val) in values.iter().enumerate() {
        if i > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [val])?;
    }
    write!(f, [token(")")])
}

/// Format a required call signature suffix.
fn format_call_signature_suffix<'a>(
    signature: TypeReference,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token(":"), space()])?;

    match signature {
        TypeReference::Type(signature) => match f.context().tree.get(signature) {
            crate::Type::FunctionSignature { parameters, result } => {
                write!(f, [token("(")])?;
                for (index, parameter) in parameters.iter().enumerate() {
                    if index > 0 {
                        write!(f, [token(","), space()])?;
                    }
                    write!(f, [*parameter])?;
                }
                write!(f, [token(")"), space(), token("->"), space(), *result])
            }
            _ => write!(f, [signature]),
        },
        TypeReference::Missing | TypeReference::Error => write!(f, [signature]),
    }
}

/// Format a bracketed, comma-separated list of values.
fn format_value_bracket_list<'a>(
    values: &[ValueReference],
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
        write!(f, [text(&val.to_string())])?;
    }
    write!(f, [token("]")])
}

/// Format a named value list like `name=[v0, v1]`.
fn format_named_value_group<'a>(
    name: &str,
    values: &[ValueReference],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [text(name), token("(")])?;
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
    name: &str,
    values: &[u32],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [text(name), token("(")])?;
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [text(&value.to_string())])?;
    }
    write!(f, [token(")")])
}

/// Format tensor dot dimension numbers.
fn format_tensor_dot_dimensions<'a>(
    dimensions: &TensorDotDimensionNumbers,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token("dims"), token("(")])?;
    format_named_u32_group("lhsBatch", &dimensions.lhs_batch, f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_group("rhsBatch", &dimensions.rhs_batch, f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_group("lhsContract", &dimensions.lhs_contracting, f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_group("rhsContract", &dimensions.rhs_contracting, f)?;
    write!(f, [token(")")])
}

/// Format tensor convolution dimension numbers.
fn format_tensor_convolution_dimensions<'a>(
    dimensions: &TensorConvolutionDimensionNumbers,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token("dims"), token("(")])?;
    write!(
        f,
        [
            token("inputBatch"),
            token("("),
            text(&dimensions.input_batch.to_string()),
            token(")"),
            token(","),
            space(),
            token("inputFeature"),
            token("("),
            text(&dimensions.input_feature.to_string()),
            token(")"),
            token(","),
            space()
        ]
    )?;
    format_named_u32_group("inputSpatial", &dimensions.input_spatial, f)?;
    write!(
        f,
        [
            token(","),
            space(),
            token("kernelInputFeature"),
            token("("),
            text(&dimensions.kernel_input_feature.to_string()),
            token(")"),
            token(","),
            space(),
            token("kernelOutputFeature"),
            token("("),
            text(&dimensions.kernel_output_feature.to_string()),
            token(")"),
            token(","),
            space()
        ]
    )?;
    format_named_u32_group("kernelSpatial", &dimensions.kernel_spatial, f)?;
    write!(
        f,
        [
            token(","),
            space(),
            token("outputBatch"),
            token("("),
            text(&dimensions.output_batch.to_string()),
            token(")"),
            token(","),
            space(),
            token("outputFeature"),
            token("("),
            text(&dimensions.output_feature.to_string()),
            token(")"),
            token(","),
            space()
        ]
    )?;
    format_named_u32_group("outputSpatial", &dimensions.output_spatial, f)?;
    write!(f, [token(")")])
}

/// Format tensor convolution window parameters.
fn format_tensor_convolution_window<'a>(
    window: &TensorConvolutionWindow,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token(","), space()])?;
    write!(f, [token("window"), token("(")])?;
    format_named_u64_group("strides", &window.strides, f)?;
    write!(f, [token(","), space()])?;
    format_named_u64_group("paddingLow", &window.padding_low, f)?;
    write!(f, [token(","), space()])?;
    format_named_u64_group("paddingHigh", &window.padding_high, f)?;
    write!(f, [token(","), space()])?;
    format_named_u64_group("lhsDilation", &window.lhs_dilation, f)?;
    write!(f, [token(","), space()])?;
    format_named_u64_group("rhsDilation", &window.rhs_dilation, f)?;
    write!(f, [token(","), space()])?;
    format_named_bool_group("windowReversal", &window.window_reversal, f)?;
    write!(f, [token(")")])
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
            text(&feature_group_count.to_string()),
            token(")"),
            token(","),
            space(),
            token("batch"),
            token("("),
            text(&batch_group_count.to_string()),
            token(")")
        ]
    )?;
    write!(f, [token(")")])
}

/// Format tensor gather dimension numbers.
fn format_tensor_gather_dimensions<'a>(
    dimensions: &TensorGatherDimensionNumbers,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token("dims"), token("(")])?;
    format_named_u32_group("offsetDims", &dimensions.offset_dims, f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_group("collapsedSliceDims", &dimensions.collapsed_slice_dims, f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_group("startIndexMap", &dimensions.start_index_map, f)?;
    write!(
        f,
        [
            token(","),
            space(),
            token("indexVectorDim"),
            token("("),
            text(&dimensions.index_vector_dim.to_string()),
            token(")")
        ]
    )?;
    write!(f, [token(")")])
}

/// Format tensor scatter dimension numbers.
fn format_tensor_scatter_dimensions<'a>(
    dimensions: &TensorScatterDimensionNumbers,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token("dims"), token("(")])?;
    format_named_u32_group("updateWindowDims", &dimensions.update_window_dims, f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_group("insertedWindowDims", &dimensions.inserted_window_dims, f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_group(
        "scatterDimsToOperandDims",
        &dimensions.scatter_dims_to_operand_dims,
        f,
    )?;
    write!(
        f,
        [
            token(","),
            space(),
            token("indexVectorDim"),
            token("("),
            text(&dimensions.index_vector_dim.to_string()),
            token(")")
        ]
    )?;
    write!(f, [token(")")])
}

/// Format a named u64 group like `name(0, 1)`.
fn format_named_u64_group<'a>(
    name: &str,
    values: &[u64],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [text(name), token("(")])?;
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [text(&value.to_string())])?;
    }
    write!(f, [token(")")])
}

/// Format a named boolean group like `name(true, false)`.
fn format_named_bool_group<'a>(
    name: &str,
    values: &[bool],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [text(name), token("(")])?;
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [token(if *value { "true" } else { "false" })])?;
    }
    write!(f, [token(")")])
}

/// Split packed tensor range arguments.
fn split_tensor_ranges(
    values: &[ValueReference],
    offsets_count: u16,
    sizes_count: u16,
    strides_count: u16,
) -> (&[ValueReference], &[ValueReference], &[ValueReference]) {
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
    values: &[ValueReference],
    low_count: u16,
    high_count: u16,
    interior_count: u16,
) -> (&[ValueReference], &[ValueReference], &[ValueReference]) {
    let low_end = low_count as usize;
    let high_end = low_end + high_count as usize;
    let interior_end = high_end + interior_count as usize;

    let low = values.get(..low_end).unwrap_or(&[]);
    let high = values.get(low_end..high_end).unwrap_or(&[]);
    let interior = values.get(high_end..interior_end).unwrap_or(&[]);

    (low, high, interior)
}

/// Format intrinsic arguments with optional memory ordering.
fn format_intrinsic_args<'a>(
    values: &[ValueReference],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token("(")])?;
    for (i, val) in values.iter().enumerate() {
        if i > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [val])?;
    }
    write!(f, [token(")")])
}

/// Format one atomic ordering, scope, memory scope, and flags suffix.
fn format_atomic_suffix<'a>(
    ordering: MemoryOrdering,
    scope: AtomicScope,
    memory_scope: MemoryScope,
    flags: MemoryFlags,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token(","), space()])?;
    write!(f, [token(ordering.to_str())])?;
    write!(f, [token(","), space(), token(scope.to_str())])?;
    write!(f, [token(","), space(), token(memory_scope.to_str())])?;
    write!(f, [token(","), space()])?;
    format_memory_flags(flags, f)
}

/// Format one atomic fence ordering, scope, memory scope, and flags suffix.
fn format_atomic_fence_suffix<'a>(
    ordering: MemoryOrdering,
    scope: AtomicScope,
    memory_scope: MemoryScope,
    flags: MemoryFlags,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [space(), token(ordering.to_str())])?;
    write!(f, [token(","), space(), token(scope.to_str())])?;
    write!(f, [token(","), space(), token(memory_scope.to_str())])?;
    write!(f, [token(","), space()])?;
    format_memory_flags(flags, f)
}

/// Format memory flags for atomics and barriers.
fn format_memory_flags<'a>(flags: MemoryFlags, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    // collect formatted names
    let names = collect_memory_flag_names(flags);

    // render the list or a single token
    if names.len() == 1 {
        write!(f, [token(names[0])])
    } else {
        let joined = names.join(", ");
        write!(f, [token("["), text(&joined), token("]")])
    }
}

/// Collect memory flag names in formatting order.
fn collect_memory_flag_names(flags: MemoryFlags) -> Vec<&'static str> {
    // collect location names first
    let mut names = collect_effect_space_names(flags.spaces);

    // append predicates
    if flags.is_volatile {
        names.push("volatile");
    }
    if flags.makes_available {
        names.push("makeAvailable");
    }
    if flags.makes_visible {
        names.push("makeVisible");
    }

    names
}

/// Collect named memory spaces in formatting order.
fn collect_effect_space_names(spaces: MemorySpaceSet) -> Vec<&'static str> {
    // special cases for named sets
    if spaces == MemorySpaceSet::NONE {
        return vec!["none"];
    }
    if spaces == MemorySpaceSet::ANY {
        return vec!["any"];
    }

    // collect named spaces in canonical order
    let mut names = Vec::new();
    let ordered = [
        ("heap", MemorySpaceSet::HEAP),
        ("rawHeap", MemorySpaceSet::RAW_HEAP),
        ("stack", MemorySpaceSet::STACK),
        ("static", MemorySpaceSet::STATIC),
        ("shared", MemorySpaceSet::SHARED),
        ("local", MemorySpaceSet::LOCAL),
        ("constant", MemorySpaceSet::CONSTANT),
        ("io", MemorySpaceSet::IO),
    ];
    for (name, set) in ordered {
        if spaces.contains(set) {
            names.push(name);
        }
    }

    names
}
