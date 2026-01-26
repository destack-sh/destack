use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    AtomicScope, FormatMirNode, Function, Global, Instruction, LocalNodeId, MemoryLocationSet,
    MemoryOrdering, MemoryScope, MemorySemantics, MirFormatter, TensorConvolutionDimensionNumbers,
    TensorConvolutionWindow, TensorDotDimensionNumbers, TensorGatherDimensionNumbers,
    TensorScatterDimensionNumbers, Value,
};

impl<'a> FormatMirNode<'a, Instruction> for Instruction {
    fn format_node(
        &self,
        _id: LocalNodeId<Instruction>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        match self {
            Instruction::Const { destination, value } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("iconst"),
                        space(),
                        value
                    ]
                )
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
                        text(&format!("local{local_index}"))
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
                        token("local.addr"),
                        space(),
                        text(&format!("local{local_index}"))
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
                    [space(), token("="), space(), token("global.addr"), space()]
                )?;
                format_global_reference(*global, f)?;
                Ok(())
            }

            Instruction::GlobalConst {
                destination,
                global,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [space(), token("="), space(), token("global.const"), space()]
                )?;
                format_global_reference(*global, f)
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
                        token("function.addr"),
                        space()
                    ]
                )?;
                format_function_reference(*function, f)
            }
            Instruction::FunctionEnv { destination } => {
                format_typed_destination(*destination, f)?;
                write!(f, [space(), token("="), space(), token("function.env")])
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

            Instruction::RawDrop { value } => write!(f, [token("raw.drop"), space(), value]),

            Instruction::StackDrop { value } => write!(f, [token("stack.drop"), space(), value]),

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
                        token("field.addr"),
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
                        index
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
                        token("element.addr"),
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
                        index,
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
                    format_value_bracket_list(args, f)?;
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
                format_u32_bracket_list(dimensions, f)
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
                format_u32_bracket_list(permutation, f)
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
                format_named_value_list("offsets", offsets, f)?;
                write!(f, [token(","), space()])?;
                format_named_value_list("sizes", sizes, f)?;
                write!(f, [token(","), space()])?;
                format_named_value_list("strides", strides, f)
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
                format_named_value_list("offsets", offsets, f)?;
                write!(f, [token(","), space()])?;
                format_named_value_list("sizes", sizes, f)?;
                write!(f, [token(","), space()])?;
                format_named_value_list("strides", strides, f)
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
                        space(),
                        token("value"),
                        token("="),
                        value,
                        token(","),
                        space()
                    ]
                )?;
                let args = f.context().tree.get_arguments(*arguments);
                let (low, high, interior) =
                    split_tensor_padding(args, *low_count, *high_count, *interior_count);
                format_named_value_list("low", low, f)?;
                write!(f, [token(","), space()])?;
                format_named_value_list("high", high, f)?;
                write!(f, [token(","), space()])?;
                format_named_value_list("interior", interior, f)
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
                format_value_bracket_list(args, f)?;
                write!(
                    f,
                    [
                        token(","),
                        space(),
                        token("axis"),
                        token("="),
                        text(&axis.to_string())
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
                format_named_u32_list("axes", axes, f)
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
                write!(
                    f,
                    [
                        token(","),
                        space(),
                        token("feature_group"),
                        token("="),
                        text(&feature_group_count.to_string()),
                        token(","),
                        space(),
                        token("batch_group"),
                        token("="),
                        text(&batch_group_count.to_string())
                    ]
                )
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
                format_named_u32_list("slice_sizes", slice_sizes, f)
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
                        token("="),
                        token(mode.to_str())
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
                arguments,
                signature,
                ..
            } => {
                if let Some(dst) = destination {
                    format_typed_destination(*dst, f)?;
                    write!(f, [space(), token("="), space()])?;
                }
                write!(f, [token("call"), space()])?;
                format_function_reference(*function, f)?;
                let args = f.context().tree.get_arguments(*arguments);
                format_value_list(args, f)?;
                write!(f, [space(), token("->"), space(), signature])
            }

            Instruction::CallVirtual {
                destination,
                receiver,
                arguments,
                declaring_type,
                slot_id,
                declared_target,
                signature,
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
                        text(&slot_id.to_string())
                    ]
                )?;
                if let Some(target) = declared_target {
                    write!(f, [token(","), space()])?;
                    format_function_reference(*target, f)?;
                }
                let args = f.context().tree.get_arguments(*arguments);
                format_value_list(args, f)?;
                write!(f, [space(), token("->"), space(), signature])
            }

            Instruction::CallInterface {
                destination,
                receiver,
                arguments,
                declaring_type,
                slot_id,
                declared_target,
                signature,
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
                        text(&slot_id.to_string())
                    ]
                )?;
                if let Some(target) = declared_target {
                    write!(f, [token(","), space()])?;
                    format_function_reference(*target, f)?;
                }
                let args = f.context().tree.get_arguments(*arguments);
                format_value_list(args, f)?;
                write!(f, [space(), token("->"), space(), signature])
            }

            Instruction::CallIndirect {
                destination,
                callee,
                env,
                arguments,
                signature,
                ..
            } => {
                if let Some(dst) = destination {
                    format_typed_destination(*dst, f)?;
                    write!(f, [space(), token("="), space()])?;
                }
                write!(f, [token("call.indirect"), space(), callee])?;
                let args = f.context().tree.get_arguments(*arguments);
                format_value_list_with_env(args, *env, f)?;
                write!(f, [space(), token("->"), space(), signature])
            }

            Instruction::ManagedAlloc {
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
                        token("managed.alloc"),
                        space(),
                        layout
                    ]
                )
            }

            Instruction::ManagedAllocArray {
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
                        token("managed.alloc_array"),
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

            Instruction::Intrinsic {
                destination,
                intrinsic,
                arguments,
                ordering,
                scope,
                memory_scope,
                semantics,
            } => {
                if let Some(dst) = destination {
                    format_typed_destination(*dst, f)?;
                    write!(f, [space(), token("="), space()])?;
                }
                write!(f, [token("intrinsic."), token(intrinsic.to_str())])?;
                let args = f.context().tree.get_arguments(*arguments);
                format_intrinsic_args(args, *ordering, *scope, *memory_scope, *semantics, f)
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
        .unwrap_or_else(|| panic!("missing value type for instruction destination"));
    write!(f, [destination, token(":"), space(), ty])
}

/// Format a function reference.
fn format_function_reference<'a>(
    function_id: LocalNodeId<Function>,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    // resolve the function name before formatting
    let name = f.context().function_name(function_id).to_string();
    write!(f, [token("@"), text(&name)])
}

/// Format a global reference.
fn format_global_reference<'a>(
    global_id: LocalNodeId<Global>,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    // resolve the global name before formatting
    let name = f.context().global_name(global_id).to_string();
    write!(f, [token("@"), text(&name)])
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

/// Format a parenthesized list of values with an optional env argument.
fn format_value_list_with_env<'a>(
    values: &[Value],
    env: Option<Value>,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token("(")])?;

    let mut needs_comma = false;
    for value in values {
        if needs_comma {
            write!(f, [token(","), space()])?;
        }
        write!(f, [value])?;
        needs_comma = true;
    }

    if let Some(env) = env {
        if needs_comma {
            write!(f, [token(","), space()])?;
        }
        write!(f, [token("env"), token("="), env])?;
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
        write!(f, [text(&val.to_string())])?;
    }
    write!(f, [token("]")])
}

/// Format a bracketed, comma-separated list of u64 values.
fn format_u64_bracket_list<'a>(values: &[u64], f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    write!(f, [token("[")])?;
    for (i, val) in values.iter().enumerate() {
        if i > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [text(&val.to_string())])?;
    }
    write!(f, [token("]")])
}

/// Format a bracketed, comma-separated list of bool values.
fn format_bool_bracket_list<'a>(values: &[bool], f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    write!(f, [token("[")])?;
    for (i, val) in values.iter().enumerate() {
        if i > 0 {
            write!(f, [token(","), space()])?;
        }
        let text = if *val { "true" } else { "false" };
        write!(f, [token(text)])?;
    }
    write!(f, [token("]")])
}

/// Format a named value list like `name=[v0, v1]`.
fn format_named_value_list<'a>(
    name: &str,
    values: &[Value],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [text(name), token("=")])?;
    format_value_bracket_list(values, f)
}

/// Format a named u32 list like `name=[0, 1]`.
fn format_named_u32_list<'a>(
    name: &str,
    values: &[u32],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [text(name), token("=")])?;
    format_u32_bracket_list(values, f)
}

/// Format tensor dot dimension numbers.
fn format_tensor_dot_dimensions<'a>(
    dimensions: &TensorDotDimensionNumbers,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token("dims"), token("(")])?;
    format_named_u32_list("lhs_batch", &dimensions.lhs_batch, f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_list("rhs_batch", &dimensions.rhs_batch, f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_list("lhs_contract", &dimensions.lhs_contracting, f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_list("rhs_contract", &dimensions.rhs_contracting, f)?;
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
            token("input_batch"),
            token("="),
            text(&dimensions.input_batch.to_string()),
            token(","),
            space(),
            token("input_feature"),
            token("="),
            text(&dimensions.input_feature.to_string()),
            token(","),
            space()
        ]
    )?;
    format_named_u32_list("input_spatial", &dimensions.input_spatial, f)?;
    write!(
        f,
        [
            token(","),
            space(),
            token("kernel_input_feature"),
            token("="),
            text(&dimensions.kernel_input_feature.to_string()),
            token(","),
            space(),
            token("kernel_output_feature"),
            token("="),
            text(&dimensions.kernel_output_feature.to_string()),
            token(","),
            space()
        ]
    )?;
    format_named_u32_list("kernel_spatial", &dimensions.kernel_spatial, f)?;
    write!(
        f,
        [
            token(","),
            space(),
            token("output_batch"),
            token("="),
            text(&dimensions.output_batch.to_string()),
            token(","),
            space(),
            token("output_feature"),
            token("="),
            text(&dimensions.output_feature.to_string()),
            token(","),
            space()
        ]
    )?;
    format_named_u32_list("output_spatial", &dimensions.output_spatial, f)?;
    write!(f, [token(")")])
}

/// Format tensor convolution window parameters.
fn format_tensor_convolution_window<'a>(
    window: &TensorConvolutionWindow,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token(","), space()])?;
    write!(f, [token("strides"), token("=")])?;
    format_u64_bracket_list(&window.strides, f)?;
    write!(f, [token(","), space()])?;
    write!(f, [token("padding_low"), token("=")])?;
    format_u64_bracket_list(&window.padding_low, f)?;
    write!(f, [token(","), space()])?;
    write!(f, [token("padding_high"), token("=")])?;
    format_u64_bracket_list(&window.padding_high, f)?;
    write!(f, [token(","), space()])?;
    write!(f, [token("lhs_dilation"), token("=")])?;
    format_u64_bracket_list(&window.lhs_dilation, f)?;
    write!(f, [token(","), space()])?;
    write!(f, [token("rhs_dilation"), token("=")])?;
    format_u64_bracket_list(&window.rhs_dilation, f)?;
    write!(f, [token(","), space()])?;
    write!(f, [token("window_reversal"), token("=")])?;
    format_bool_bracket_list(&window.window_reversal, f)
}

/// Format tensor gather dimension numbers.
fn format_tensor_gather_dimensions<'a>(
    dimensions: &TensorGatherDimensionNumbers,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token("dims"), token("(")])?;
    format_named_u32_list("offset_dims", &dimensions.offset_dims, f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_list("collapsed_slice_dims", &dimensions.collapsed_slice_dims, f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_list("start_index_map", &dimensions.start_index_map, f)?;
    write!(
        f,
        [
            token(","),
            space(),
            token("index_vector_dim"),
            token("="),
            text(&dimensions.index_vector_dim.to_string())
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
    format_named_u32_list("update_window_dims", &dimensions.update_window_dims, f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_list("inserted_window_dims", &dimensions.inserted_window_dims, f)?;
    write!(f, [token(","), space()])?;
    format_named_u32_list(
        "scatter_dims_to_operand_dims",
        &dimensions.scatter_dims_to_operand_dims,
        f,
    )?;
    write!(
        f,
        [
            token(","),
            space(),
            token("index_vector_dim"),
            token("="),
            text(&dimensions.index_vector_dim.to_string())
        ]
    )?;
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
fn format_intrinsic_args<'a>(
    values: &[Value],
    ordering: Option<MemoryOrdering>,
    scope: Option<AtomicScope>,
    memory_scope: Option<MemoryScope>,
    semantics: Option<MemorySemantics>,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token("(")])?;
    let mut has_arg = false;
    for (i, val) in values.iter().enumerate() {
        if i > 0 || has_arg {
            write!(f, [token(","), space()])?;
        }
        write!(f, [val])?;
        has_arg = true;
    }
    if let Some(ord) = ordering {
        if has_arg {
            write!(f, [token(","), space()])?;
        }
        write!(f, [token("ordering"), token("="), token(ord.to_str())])?;
        has_arg = true;
    }
    if let Some(scope) = scope {
        if has_arg {
            write!(f, [token(","), space()])?;
        }
        write!(f, [token("scope"), token("="), token(scope.to_str())])?;
        has_arg = true;
    }
    if let Some(memory_scope) = memory_scope {
        if has_arg {
            write!(f, [token(","), space()])?;
        }
        write!(
            f,
            [
                token("memory_scope"),
                token("="),
                token(memory_scope.to_str())
            ]
        )?;
        has_arg = true;
    }
    if let Some(semantics) = semantics {
        if has_arg {
            write!(f, [token(","), space()])?;
        }
        write!(f, [token("semantics"), token("=")])?;
        format_memory_semantics(semantics, f)?;
    }
    write!(f, [token(")")])
}

/// Format memory semantics for atomics and barriers.
fn format_memory_semantics<'a>(
    semantics: MemorySemantics,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    // collect the formatted semantics names
    let names = collect_memory_semantics_names(semantics);

    // render the list or a single token
    if names.len() == 1 {
        write!(f, [token(names[0])])
    } else {
        let joined = names.join(", ");
        write!(f, [token("["), text(&joined), token("]")])
    }
}

/// Collect memory semantics names in formatting order.
fn collect_memory_semantics_names(semantics: MemorySemantics) -> Vec<&'static str> {
    // collect location names first
    let mut names = collect_memory_location_names(semantics.locations);

    // append semantics flags
    if semantics.is_volatile {
        names.push("volatile");
    }
    if semantics.is_make_available {
        names.push("make_available");
    }
    if semantics.is_make_visible {
        names.push("make_visible");
    }

    names
}

/// Collect named memory locations in formatting order.
fn collect_memory_location_names(locations: MemoryLocationSet) -> Vec<&'static str> {
    // special cases for named sets
    if locations == MemoryLocationSet::NONE {
        return vec!["none"];
    }
    if locations == MemoryLocationSet::ANY {
        return vec!["any"];
    }

    // collect named locations in canonical order
    let mut names = Vec::new();
    let ordered = [
        ("arguments", MemoryLocationSet::ARGUMENTS),
        ("heap", MemoryLocationSet::HEAP),
        ("stack", MemoryLocationSet::STACK),
        ("global", MemoryLocationSet::GLOBAL),
        ("shared", MemoryLocationSet::SHARED),
        ("local", MemoryLocationSet::LOCAL),
        ("constant", MemoryLocationSet::CONSTANT),
        ("inaccessible", MemoryLocationSet::INACCESSIBLE),
        ("io", MemoryLocationSet::IO),
    ];
    for (name, set) in ordered {
        if locations.contains(set) {
            names.push(name);
        }
    }

    names
}
