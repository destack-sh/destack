use destack_mir as mir;

use crate::program::{
    ElementAccess, Instruction, Opcode, TensorBroadcast, TensorCast, TensorCompare, TensorConcat,
    TensorConvert, TensorConvolution, TensorCopy, TensorDot, TensorExtract, TensorFill,
    TensorGather, TensorLoad, TensorPad, TensorReduce, TensorReshape, TensorScatter, TensorSelect,
    TensorSlice, TensorSplat, TensorStore, TensorTranspose, TensorView,
};
use crate::{Error, Result};

use super::access::{tensor_element_access, tensor_element_type, tensor_view_pointer_class};
use super::lower::BlockLowerer;
use super::pool::Pool;

impl<'a> BlockLowerer<'a> {
    /// Lower one tensor instruction.
    pub(super) fn lower_tensor(
        &self,
        inst: &mir::Instruction,
        pool: &mut Pool<'_>,
    ) -> Result<Instruction> {
        Ok(match inst {
            mir::Instruction::TensorSplat { destination, value } => {
                let destination = tensor_value(*destination, "tensor splat destination")?;
                let value = tensor_value(*value, "tensor splat value")?;
                let tensor_type = self.value_type_for_value(destination)?;

                Instruction::new(
                    Opcode::TensorSplat,
                    TensorSplat {
                        dest: destination,
                        value,
                        tensor_type,
                    },
                )
            }
            mir::Instruction::TensorLoad {
                destination,
                view,
                indices,
            } => {
                let destination = tensor_value(*destination, "tensor load destination")?;
                let view = tensor_value(*view, "tensor load view")?;
                let view_type = self.value_type_for_value(view)?;
                let indices = pool.argument_reference_range(
                    self.tree.get_arguments(*indices),
                    "tensor load index",
                )?;
                let element = self
                    .tensor_element_access(view_type)
                    .map(|element| pool.element_access(element));

                Instruction::new(
                    Opcode::TensorLoad,
                    TensorLoad {
                        dest: destination,
                        view,
                        indices,
                        view_type,
                        element,
                    },
                )
            }
            mir::Instruction::TensorExtract {
                destination,
                tensor,
                indices,
            } => {
                let destination = tensor_value(*destination, "tensor extract destination")?;
                let tensor = tensor_value(*tensor, "tensor extract source")?;
                let tensor_type = self.value_type_for_value(tensor)?;
                let indices = pool.argument_reference_range(
                    self.tree.get_arguments(*indices),
                    "tensor extract index",
                )?;

                Instruction::new(
                    Opcode::TensorExtract,
                    TensorExtract {
                        dest: destination,
                        tensor,
                        indices,
                        tensor_type,
                    },
                )
            }
            mir::Instruction::TensorStore {
                view,
                indices,
                value,
            } => {
                let view = tensor_value(*view, "tensor store view")?;
                let value = tensor_value(*value, "tensor store value")?;
                let view_type = self.value_type_for_value(view)?;
                let indices = pool.argument_reference_range(
                    self.tree.get_arguments(*indices),
                    "tensor store index",
                )?;
                let element = self
                    .tensor_element_access(view_type)
                    .map(|element| pool.element_access(element));

                Instruction::new(
                    Opcode::TensorStore,
                    TensorStore {
                        view,
                        indices,
                        value,
                        view_type,
                        element,
                    },
                )
            }
            mir::Instruction::TensorFill { view, value } => {
                let view = tensor_value(*view, "tensor fill view")?;
                let value = tensor_value(*value, "tensor fill value")?;
                let view_type = self.value_type_for_value(view)?;
                let element = self
                    .tensor_element_access(view_type)
                    .map(|element| pool.element_access(element));

                Instruction::new(
                    Opcode::TensorFill,
                    TensorFill {
                        view,
                        value,
                        view_type,
                        element,
                    },
                )
            }
            mir::Instruction::TensorCopy { target, source } => {
                let target = tensor_value(*target, "tensor move target")?;
                let source = tensor_value(*source, "tensor move source")?;
                let target_type = self.value_type_for_value(target)?;
                let source_type = self.value_type_for_value(source)?;
                let target_element = self
                    .tensor_element_access(target_type)
                    .map(|element| pool.element_access(element));
                let source_element = self
                    .tensor_element_access(source_type)
                    .map(|element| pool.element_access(element));

                Instruction::new(
                    Opcode::TensorCopy,
                    TensorCopy {
                        target,
                        source,
                        target_type,
                        source_type,
                        target_element,
                        source_element,
                    },
                )
            }
            mir::Instruction::TensorReshape {
                destination,
                tensor,
                shape,
            } => {
                let destination = tensor_value(*destination, "tensor reshape destination")?;
                let tensor = tensor_value(*tensor, "tensor reshape source")?;
                let dest_type = self.value_type_for_value(destination)?;
                let shape = pool.argument_reference_range(
                    self.tree.get_arguments(*shape),
                    "tensor reshape shape",
                )?;

                Instruction::new(
                    Opcode::TensorReshape,
                    TensorReshape {
                        dest: destination,
                        tensor,
                        shape,
                        dest_type,
                    },
                )
            }
            mir::Instruction::TensorBroadcast {
                destination,
                tensor,
                dimensions,
            } => {
                let destination = tensor_value(*destination, "tensor broadcast destination")?;
                let tensor = tensor_value(*tensor, "tensor broadcast source")?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;

                Instruction::new(
                    Opcode::TensorBroadcast,
                    TensorBroadcast {
                        dest: destination,
                        tensor,
                        dimensions: pool.u32_range(dimensions),
                        source_type,
                        dest_type,
                    },
                )
            }
            mir::Instruction::TensorTranspose {
                destination,
                tensor,
                permutation,
            } => {
                let destination = tensor_value(*destination, "tensor transpose destination")?;
                let tensor = tensor_value(*tensor, "tensor transpose source")?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;

                Instruction::new(
                    Opcode::TensorTranspose,
                    TensorTranspose {
                        dest: destination,
                        tensor,
                        permutation: pool.u32_range(permutation),
                        source_type,
                        dest_type,
                    },
                )
            }
            mir::Instruction::TensorSlice {
                destination,
                tensor,
                arguments,
                offsets_count,
                sizes_count,
                strides_count,
            } => {
                let destination = tensor_value(*destination, "tensor slice destination")?;
                let tensor = tensor_value(*tensor, "tensor slice source")?;
                let arguments = pool.argument_reference_range(
                    self.tree.get_arguments(*arguments),
                    "tensor slice argument",
                )?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;

                Instruction::new(
                    Opcode::TensorSlice,
                    TensorSlice {
                        dest: destination,
                        tensor,
                        arguments,
                        offsets_count: *offsets_count,
                        sizes_count: *sizes_count,
                        strides_count: *strides_count,
                        source_type,
                        dest_type,
                    },
                )
            }
            mir::Instruction::TensorPad {
                destination,
                tensor,
                arguments,
                low_count,
                high_count,
                interior_count,
                value,
            } => {
                let destination = tensor_value(*destination, "tensor pad destination")?;
                let tensor = tensor_value(*tensor, "tensor pad source")?;
                let value = tensor_value(*value, "tensor pad value")?;
                let arguments = pool.argument_reference_range(
                    self.tree.get_arguments(*arguments),
                    "tensor pad argument",
                )?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;

                Instruction::new(
                    Opcode::TensorPad,
                    TensorPad {
                        dest: destination,
                        tensor,
                        arguments,
                        low_count: *low_count,
                        high_count: *high_count,
                        interior_count: *interior_count,
                        value,
                        source_type,
                        dest_type,
                    },
                )
            }
            mir::Instruction::TensorConcat {
                destination,
                tensors,
                axis,
            } => {
                let destination = tensor_value(*destination, "tensor concat destination")?;
                let tensor_value = self.tree.get_arguments(*tensors);
                let tensors =
                    pool.argument_reference_range(tensor_value, "tensor concat operand")?;
                let mut tensor_types = Vec::with_capacity(tensor_value.len());
                for value in tensor_value {
                    let value = tensor_value_ref(*value, "tensor concat operand")?;
                    tensor_types.push(self.value_type_for_value(value)?);
                }
                let dest_type = self.value_type_for_value(destination)?;

                Instruction::new(
                    Opcode::TensorConcat,
                    TensorConcat {
                        dest: destination,
                        tensors,
                        tensor_types: pool.type_range(&tensor_types),
                        axis: *axis,
                        dest_type,
                    },
                )
            }
            mir::Instruction::TensorReduce {
                destination,
                operator,
                tensor,
                initial,
                axes,
            } => {
                let destination = tensor_value(*destination, "tensor reduce destination")?;
                let tensor = tensor_value(*tensor, "tensor reduce source")?;
                let initial = tensor_value(*initial, "tensor reduce initial")?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;

                Instruction::new(
                    Opcode::TensorReduce,
                    TensorReduce {
                        dest: destination,
                        operator: *operator,
                        tensor,
                        initial,
                        axes: pool.u32_range(axes),
                        source_type,
                        dest_type,
                    },
                )
            }
            mir::Instruction::TensorDot {
                destination,
                left,
                right,
                dimensions,
            } => {
                let destination = tensor_value(*destination, "tensor dot destination")?;
                let left = tensor_value(*left, "tensor dot left")?;
                let right = tensor_value(*right, "tensor dot right")?;
                let dest_type = self.value_type_for_value(destination)?;
                let left_type = self.value_type_for_value(left)?;
                let right_type = self.value_type_for_value(right)?;

                Instruction::new(
                    Opcode::TensorDot,
                    TensorDot {
                        dest: destination,
                        left,
                        right,
                        dimensions: pool.tensor_dot(dimensions.clone()),
                        left_type,
                        right_type,
                        dest_type,
                    },
                )
            }
            mir::Instruction::TensorConvolution {
                destination,
                input,
                kernel,
                dimensions,
                window,
                feature_group_count,
                batch_group_count,
            } => {
                let destination = tensor_value(*destination, "tensor convolution destination")?;
                let input = tensor_value(*input, "tensor convolution input")?;
                let kernel = tensor_value(*kernel, "tensor convolution kernel")?;
                let dest_type = self.value_type_for_value(destination)?;
                let input_type = self.value_type_for_value(input)?;
                let kernel_type = self.value_type_for_value(kernel)?;

                Instruction::new(
                    Opcode::TensorConvolution,
                    TensorConvolution {
                        dest: destination,
                        input,
                        kernel,
                        dimensions: pool.tensor_convolution(dimensions.clone()),
                        window: pool.tensor_window(window.clone()),
                        feature_group_count: *feature_group_count,
                        batch_group_count: *batch_group_count,
                        input_type,
                        kernel_type,
                        dest_type,
                    },
                )
            }
            mir::Instruction::TensorGather {
                destination,
                operand,
                indices,
                dimensions,
                slice_sizes,
            } => {
                let destination = tensor_value(*destination, "tensor gather destination")?;
                let operand = tensor_value(*operand, "tensor gather operand")?;
                let indices = tensor_value(*indices, "tensor gather indices")?;
                let dest_type = self.value_type_for_value(destination)?;
                let operand_type = self.value_type_for_value(operand)?;
                let indices_type = self.value_type_for_value(indices)?;

                Instruction::new(
                    Opcode::TensorGather,
                    TensorGather {
                        dest: destination,
                        operand,
                        indices,
                        dimensions: pool.tensor_gather(dimensions.clone()),
                        slice_sizes: pool.u32_range(slice_sizes),
                        operand_type,
                        indices_type,
                        dest_type,
                    },
                )
            }
            mir::Instruction::TensorScatter {
                destination,
                operand,
                indices,
                updates,
                dimensions,
                mode,
            } => {
                let destination = tensor_value(*destination, "tensor scatter destination")?;
                let operand = tensor_value(*operand, "tensor scatter operand")?;
                let indices = tensor_value(*indices, "tensor scatter indices")?;
                let updates = tensor_value(*updates, "tensor scatter updates")?;
                let dest_type = self.value_type_for_value(destination)?;
                let operand_type = self.value_type_for_value(operand)?;
                let indices_type = self.value_type_for_value(indices)?;
                let updates_type = self.value_type_for_value(updates)?;

                Instruction::new(
                    Opcode::TensorScatter,
                    TensorScatter {
                        dest: destination,
                        operand,
                        indices,
                        updates,
                        dimensions: pool.tensor_scatter(dimensions.clone()),
                        mode: *mode,
                        operand_type,
                        indices_type,
                        updates_type,
                        dest_type,
                    },
                )
            }
            mir::Instruction::TensorCompare {
                destination,
                operator,
                left,
                right,
            } => {
                let destination = tensor_value(*destination, "tensor compare destination")?;
                let left = tensor_value(*left, "tensor compare left")?;
                let right = tensor_value(*right, "tensor compare right")?;
                let dest_type = self.value_type_for_value(destination)?;
                let left_type = self.value_type_for_value(left)?;
                let right_type = self.value_type_for_value(right)?;

                Instruction::new(
                    Opcode::TensorCompare,
                    TensorCompare {
                        dest: destination,
                        operator: *operator,
                        left,
                        right,
                        left_type,
                        right_type,
                        dest_type,
                    },
                )
            }
            mir::Instruction::TensorSelect {
                destination,
                mask,
                then_value,
                else_value,
            } => {
                let destination = tensor_value(*destination, "tensor select destination")?;
                let mask = tensor_value(*mask, "tensor select mask")?;
                let then_value = tensor_value(*then_value, "tensor select then value")?;
                let else_value = tensor_value(*else_value, "tensor select else value")?;
                let dest_type = self.value_type_for_value(destination)?;

                Instruction::new(
                    Opcode::TensorSelect,
                    TensorSelect {
                        dest: destination,
                        mask,
                        then_value,
                        else_value,
                        dest_type,
                    },
                )
            }
            mir::Instruction::TensorConvert {
                destination,
                mode,
                tensor,
            } => {
                let destination = tensor_value(*destination, "tensor convert destination")?;
                let tensor = tensor_value(*tensor, "tensor convert source")?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;

                Instruction::new(
                    Opcode::TensorConvert,
                    TensorConvert {
                        dest: destination,
                        mode: *mode,
                        tensor,
                        source_type,
                        dest_type,
                    },
                )
            }
            mir::Instruction::TensorCast {
                destination,
                tensor,
            } => Instruction::new(
                Opcode::TensorCast,
                TensorCast {
                    dest: tensor_value(*destination, "tensor cast destination")?,
                    tensor: tensor_value(*tensor, "tensor cast source")?,
                },
            ),
            mir::Instruction::TensorView {
                destination,
                view,
                arguments,
                offsets_count,
                sizes_count,
                strides_count,
            } => {
                let destination = tensor_value(*destination, "tensor view destination")?;
                let view = tensor_value(*view, "tensor view source")?;
                let arguments = pool.argument_reference_range(
                    self.tree.get_arguments(*arguments),
                    "tensor view argument",
                )?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(view)?;
                let element = self
                    .tensor_element_access(source_type)
                    .map(|element| pool.element_access(element));

                Instruction::new(
                    Opcode::TensorView,
                    TensorView {
                        dest: destination,
                        view,
                        arguments,
                        offsets_count: *offsets_count,
                        sizes_count: *sizes_count,
                        strides_count: *strides_count,
                        source_type,
                        dest_type,
                        element,
                    },
                )
            }
            _ => return Err(Error::InvalidInstruction),
        })
    }

    /// Return the element access for one tensor view.
    fn tensor_element_access(
        &self,
        view_type: mir::LocalNodeId<mir::Type>,
    ) -> Option<ElementAccess> {
        tensor_element_type(self.tree, view_type).and_then(|element_type| {
            tensor_element_access(
                self.tree,
                self.layouts(),
                element_type,
                tensor_view_pointer_class(self.tree, view_type)?,
            )
        })
    }
}

/// Return one required tensor operand value.
fn tensor_value(reference: mir::ValueReference, context: &'static str) -> Result<mir::Value> {
    reference
        .value()
        .ok_or_else(|| Error::MissingRepresentation {
            context: context.into(),
        })
}

/// Return one required tensor operand from an argument slice.
fn tensor_value_ref(reference: mir::ValueReference, context: &'static str) -> Result<mir::Value> {
    tensor_value(reference, context)
}
