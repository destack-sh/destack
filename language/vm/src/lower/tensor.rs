use destack_mir as mir;

use crate::program::{
    Instruction, Op, PointerClass, Projection, ScalarLayout, TensorBinary, TensorBroadcast,
    TensorConcat, TensorContiguousBinary, TensorConvert, TensorConvolution, TensorCopy, TensorDot,
    TensorExtract, TensorFill, TensorGather, TensorLayout, TensorLayoutId, TensorLoad, TensorPad,
    TensorReduce, TensorReshape, TensorScatter, TensorSelect, TensorSlice, TensorStore,
    TensorTranspose, TensorView, U32RangeId, scalar_layout_from_type, value_layout_from_type,
};
use crate::{Error, Result};

use super::arithmetic::{element_binary_kernel, same_contiguous_tensor_order};
use super::frame::{value_offset, word_offset};
use super::lower::BlockLowerer;
use super::pool::Pool;
use super::projection::{
    tensor_element_projection as build_tensor_element_projection, tensor_element_type,
    tensor_view_pointer_class,
};

impl<'a> BlockLowerer<'a> {
    /// Lower one tensor instruction.
    pub(super) fn lower_tensor(
        &self,
        inst: &mir::Instruction,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        Ok(match inst {
            // tensor.splat
            mir::Instruction::TensorSplat { destination, value } => {
                let destination = tensor_value(*destination, "tensor splat destination")?;
                let value = tensor_value(*value, "tensor splat value")?;
                let tensor_type = self.value_type_for_value(destination)?;
                let tensor_layout = self.tensor_layout(pool, tensor_type)?;

                Instruction::new(
                    Op::TensorSplat,
                    value_offset(self, destination)?,
                    word_offset(self, value)?,
                    tensor_layout.0,
                    0,
                )
            }
            // tensor.load
            mir::Instruction::TensorLoad {
                destination,
                view,
                indices,
            } => {
                let destination = tensor_value(*destination, "tensor load destination")?;
                let view = tensor_value(*view, "tensor load view")?;
                let view_type = self.value_type_for_value(view)?;
                let view_layout = self.tensor_layout(pool, view_type)?;
                let indices = self.word_offset_reference_range(
                    pool,
                    self.tree.get_arguments(*indices),
                    "tensor load index",
                )?;
                let (pointer_class, element) = self.tensor_element_projection(view_type)?;
                let op = tensor_load_op(pointer_class)?;
                let element = pool.projection(element);

                pool.instruction_with_side(
                    op,
                    TensorLoad {
                        dest_offset: word_offset(self, destination)?,
                        view_offset: word_offset(self, view)?,
                        indices,
                        view_layout,
                        element,
                    },
                )
            }
            // tensor.extract
            mir::Instruction::TensorExtract {
                destination,
                tensor,
                indices,
            } => {
                let destination = tensor_value(*destination, "tensor extract destination")?;
                let tensor = tensor_value(*tensor, "tensor extract source")?;
                let tensor_type = self.value_type_for_value(tensor)?;
                let tensor_layout = self.tensor_layout(pool, tensor_type)?;
                let indices = self.word_offset_reference_range(
                    pool,
                    self.tree.get_arguments(*indices),
                    "tensor extract index",
                )?;

                pool.instruction_with_side(
                    Op::TensorExtract,
                    TensorExtract {
                        dest_offset: word_offset(self, destination)?,
                        tensor_offset: value_offset(self, tensor)?,
                        indices,
                        tensor_layout,
                    },
                )
            }
            // tensor.store
            mir::Instruction::TensorStore {
                view,
                indices,
                value,
            } => {
                let view = tensor_value(*view, "tensor store view")?;
                let value = tensor_value(*value, "tensor store value")?;
                let view_type = self.value_type_for_value(view)?;
                let view_layout = self.tensor_layout(pool, view_type)?;
                let indices = self.word_offset_reference_range(
                    pool,
                    self.tree.get_arguments(*indices),
                    "tensor store index",
                )?;
                let (pointer_class, element) = self.tensor_element_projection(view_type)?;
                let op = tensor_store_op(pointer_class)?;
                let element = pool.projection(element);

                pool.instruction_with_side(
                    op,
                    TensorStore {
                        view_offset: word_offset(self, view)?,
                        indices,
                        value_offset: word_offset(self, value)?,
                        view_layout,
                        element,
                    },
                )
            }
            // tensor.fill
            mir::Instruction::TensorFill { view, value } => {
                let view = tensor_value(*view, "tensor fill view")?;
                let value = tensor_value(*value, "tensor fill value")?;
                let view_type = self.value_type_for_value(view)?;
                let view_layout = self.tensor_layout(pool, view_type)?;
                let (pointer_class, element) = self.tensor_element_projection(view_type)?;
                let op = tensor_fill_op(pointer_class)?;
                let element = pool.projection(element);

                pool.instruction_with_side(
                    op,
                    TensorFill {
                        view_offset: word_offset(self, view)?,
                        value_offset: word_offset(self, value)?,
                        view_layout,
                        element,
                    },
                )
            }
            // tensor.copy
            mir::Instruction::TensorCopy { target, source } => {
                let target = tensor_value(*target, "tensor copy target")?;
                let source = tensor_value(*source, "tensor copy source")?;
                let target_type = self.value_type_for_value(target)?;
                let source_type = self.value_type_for_value(source)?;
                let target_layout = self.tensor_layout(pool, target_type)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let (target_class, target_element) = self.tensor_element_projection(target_type)?;
                let (source_class, source_element) = self.tensor_element_projection(source_type)?;
                let op = tensor_copy_op(target_class, source_class)?;
                let target_element = pool.projection(target_element);
                let source_element = pool.projection(source_element);

                pool.instruction_with_side(
                    op,
                    TensorCopy {
                        target_offset: word_offset(self, target)?,
                        source_offset: word_offset(self, source)?,
                        target_layout,
                        source_layout,
                        target_element,
                        source_element,
                    },
                )
            }
            // tensor.reshape
            mir::Instruction::TensorReshape {
                destination,
                tensor,
                shape,
            } => {
                let destination = tensor_value(*destination, "tensor reshape destination")?;
                let tensor = tensor_value(*tensor, "tensor reshape source")?;
                let source_type = self.value_type_for_value(tensor)?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let shape = self.word_offset_reference_range(
                    pool,
                    self.tree.get_arguments(*shape),
                    "tensor reshape shape",
                )?;

                pool.instruction_with_side(
                    Op::TensorReshape,
                    TensorReshape {
                        dest_offset: value_offset(self, destination)?,
                        tensor_offset: value_offset(self, tensor)?,
                        shape,
                        source_layout,
                        dest_layout,
                    },
                )
            }
            // tensor.broadcast
            mir::Instruction::TensorBroadcast {
                destination,
                tensor,
                dimensions,
            } => {
                let destination = tensor_value(*destination, "tensor broadcast destination")?;
                let tensor = tensor_value(*tensor, "tensor broadcast source")?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let dimensions = pool.u32_range(dimensions);

                pool.instruction_with_side(
                    Op::TensorBroadcast,
                    TensorBroadcast {
                        dest_offset: value_offset(self, destination)?,
                        tensor_offset: value_offset(self, tensor)?,
                        dimensions,
                        source_layout,
                        dest_layout,
                    },
                )
            }
            // tensor.transpose
            mir::Instruction::TensorTranspose {
                destination,
                tensor,
                permutation,
            } => {
                let destination = tensor_value(*destination, "tensor transpose destination")?;
                let tensor = tensor_value(*tensor, "tensor transpose source")?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let permutation = pool.u32_range(permutation);

                pool.instruction_with_side(
                    Op::TensorTranspose,
                    TensorTranspose {
                        dest_offset: value_offset(self, destination)?,
                        tensor_offset: value_offset(self, tensor)?,
                        permutation,
                        source_layout,
                        dest_layout,
                    },
                )
            }
            // tensor.slice
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
                let arguments = self.word_offset_reference_range(
                    pool,
                    self.tree.get_arguments(*arguments),
                    "tensor slice argument",
                )?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;

                pool.instruction_with_side(
                    Op::TensorSlice,
                    TensorSlice {
                        dest_offset: value_offset(self, destination)?,
                        tensor_offset: value_offset(self, tensor)?,
                        arguments,
                        offsets_count: *offsets_count,
                        sizes_count: *sizes_count,
                        strides_count: *strides_count,
                        source_layout,
                        dest_layout,
                    },
                )
            }
            // tensor.pad
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
                let arguments = self.word_offset_reference_range(
                    pool,
                    self.tree.get_arguments(*arguments),
                    "tensor pad argument",
                )?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;

                pool.instruction_with_side(
                    Op::TensorPad,
                    TensorPad {
                        dest_offset: value_offset(self, destination)?,
                        tensor_offset: value_offset(self, tensor)?,
                        arguments,
                        low_count: *low_count,
                        high_count: *high_count,
                        interior_count: *interior_count,
                        value_offset: word_offset(self, value)?,
                        source_layout,
                        dest_layout,
                    },
                )
            }
            // tensor.concat
            mir::Instruction::TensorConcat {
                destination,
                tensors,
                axis,
            } => {
                let destination = tensor_value(*destination, "tensor concat destination")?;
                let tensor_value = self.tree.get_arguments(*tensors);
                let tensors =
                    self.value_offset_reference_range(pool, tensor_value, "tensor concat input")?;
                let mut tensor_layouts = Vec::with_capacity(tensor_value.len());
                for value in tensor_value {
                    let value = tensor_value_ref(*value, "tensor concat input")?;
                    let value_type = self.value_type_for_value(value)?;
                    tensor_layouts.push(self.tensor_layout(pool, value_type)?.0);
                }
                let dest_type = self.value_type_for_value(destination)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let tensor_layouts = pool.u32_range(&tensor_layouts);

                pool.instruction_with_side(
                    Op::TensorConcat,
                    TensorConcat {
                        dest_offset: value_offset(self, destination)?,
                        tensors,
                        tensor_layouts,
                        axis: *axis,
                        dest_layout,
                    },
                )
            }
            // tensor.reduce
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
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let axes = pool.u32_range(axes);

                pool.instruction_with_side(
                    Op::TensorReduce,
                    TensorReduce {
                        dest_offset: value_offset(self, destination)?,
                        tensor_offset: value_offset(self, tensor)?,
                        initial_offset: word_offset(self, initial)?,
                        axes,
                        source_layout,
                        dest_layout,
                        kernel: *operator,
                    },
                )
            }
            // tensor.dot
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
                let dest_layout = TensorLayout::from_type(self.tree, self.layouts(), dest_type)?;
                let left_layout = TensorLayout::from_type(self.tree, self.layouts(), left_type)?;
                let right_layout = TensorLayout::from_type(self.tree, self.layouts(), right_type)?;
                let dimensions = pool.tensor_dot(dimensions.clone());
                let element = tensor_element_type(self.tree, dest_type).ok_or_else(|| {
                    Error::MissingRepresentation {
                        context: "tensor dot element".to_string(),
                    }
                })?;
                let element_layout = tensor_scalar_layout(self.tree, element)?;

                let dest_layout = pool.tensor_layout(dest_layout);
                let left_layout = pool.tensor_layout(left_layout);
                let right_layout = pool.tensor_layout(right_layout);

                pool.instruction_with_side(
                    Op::TensorDot,
                    TensorDot {
                        dest_offset: value_offset(self, destination)?,
                        left_offset: value_offset(self, left)?,
                        right_offset: value_offset(self, right)?,
                        dimensions,
                        left_layout,
                        right_layout,
                        dest_layout,
                        element_layout,
                    },
                )
            }
            // tensor.convolution
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
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let input_layout = self.tensor_layout(pool, input_type)?;
                let kernel_layout = self.tensor_layout(pool, kernel_type)?;
                let dimensions = pool.tensor_convolution(dimensions.clone());
                let window = pool.tensor_window(window.clone());
                let element = tensor_element_type(self.tree, dest_type).ok_or_else(|| {
                    Error::MissingRepresentation {
                        context: "tensor convolution element".to_string(),
                    }
                })?;
                let element_layout = tensor_scalar_layout(self.tree, element)?;

                pool.instruction_with_side(
                    Op::TensorConvolution,
                    TensorConvolution {
                        dest_offset: value_offset(self, destination)?,
                        input_offset: value_offset(self, input)?,
                        kernel_offset: value_offset(self, kernel)?,
                        dimensions,
                        window,
                        feature_group_count: *feature_group_count,
                        batch_group_count: *batch_group_count,
                        input_layout,
                        kernel_layout,
                        dest_layout,
                        element_layout,
                    },
                )
            }
            // tensor.gather
            mir::Instruction::TensorGather {
                destination,
                operand: source,
                indices,
                dimensions,
                slice_sizes,
            } => {
                let destination = tensor_value(*destination, "tensor gather destination")?;
                let source = tensor_value(*source, "tensor gather source")?;
                let indices = tensor_value(*indices, "tensor gather indices")?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(source)?;
                let indices_type = self.value_type_for_value(indices)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let indices_layout = self.tensor_layout(pool, indices_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let dimensions = pool.tensor_gather(dimensions.clone());
                let slice_sizes = pool.u32_range(slice_sizes);

                pool.instruction_with_side(
                    Op::TensorGather,
                    TensorGather {
                        dest_offset: value_offset(self, destination)?,
                        source_offset: value_offset(self, source)?,
                        indices_offset: value_offset(self, indices)?,
                        dimensions,
                        slice_sizes,
                        source_layout,
                        indices_layout,
                        dest_layout,
                    },
                )
            }
            // tensor.scatter
            mir::Instruction::TensorScatter {
                destination,
                operand: source,
                indices,
                updates,
                dimensions,
                mode,
            } => {
                let destination = tensor_value(*destination, "tensor scatter destination")?;
                let source = tensor_value(*source, "tensor scatter source")?;
                let indices = tensor_value(*indices, "tensor scatter indices")?;
                let updates = tensor_value(*updates, "tensor scatter updates")?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(source)?;
                let indices_type = self.value_type_for_value(indices)?;
                let updates_type = self.value_type_for_value(updates)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let indices_layout = self.tensor_layout(pool, indices_type)?;
                let updates_layout = self.tensor_layout(pool, updates_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let dimensions = pool.tensor_scatter(dimensions.clone());
                let element = tensor_element_type(self.tree, dest_type).ok_or_else(|| {
                    Error::MissingRepresentation {
                        context: "tensor scatter element".to_string(),
                    }
                })?;
                let element_layout = tensor_scalar_layout(self.tree, element)?;

                pool.instruction_with_side(
                    Op::TensorScatter,
                    TensorScatter {
                        dest_offset: value_offset(self, destination)?,
                        source_offset: value_offset(self, source)?,
                        indices_offset: value_offset(self, indices)?,
                        updates_offset: value_offset(self, updates)?,
                        dimensions,
                        source_layout,
                        indices_layout,
                        updates_layout,
                        dest_layout,
                        element_layout,
                        mode: *mode,
                    },
                )
            }
            // tensor.compare
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
                let dest_layout = TensorLayout::from_type(self.tree, self.layouts(), dest_type)?;
                let left_layout = TensorLayout::from_type(self.tree, self.layouts(), left_type)?;
                let right_layout = TensorLayout::from_type(self.tree, self.layouts(), right_type)?;
                let element = tensor_element_type(self.tree, left_type).ok_or_else(|| {
                    Error::MissingRepresentation {
                        context: "tensor compare element".to_string(),
                    }
                })?;
                let element_layout = value_layout_from_type(self.tree, element);
                let kernel = element_binary_kernel(*operator, element_layout)
                    .ok_or(Error::InvalidInstruction)?;
                if same_contiguous_tensor_order(&dest_layout, &left_layout, &right_layout) {
                    let dest_layout = pool.tensor_layout(dest_layout);

                    return Ok(pool.instruction_with_side(
                        Op::TensorContiguousBinary,
                        TensorContiguousBinary {
                            dest_offset: value_offset(self, destination)?,
                            left_offset: value_offset(self, left)?,
                            right_offset: value_offset(self, right)?,
                            dest_layout,
                            element_layout: tensor_scalar_layout(self.tree, element)?,
                            kernel,
                        },
                    ));
                }

                let left_layout = pool.tensor_layout(left_layout);
                let right_layout = pool.tensor_layout(right_layout);
                let dest_layout = pool.tensor_layout(dest_layout);

                pool.instruction_with_side(
                    Op::TensorBinary,
                    TensorBinary {
                        dest_offset: value_offset(self, destination)?,
                        left_offset: value_offset(self, left)?,
                        right_offset: value_offset(self, right)?,
                        left_layout,
                        right_layout,
                        dest_layout,
                        kernel,
                        element_layout: tensor_scalar_layout(self.tree, element)?,
                    },
                )
            }
            // tensor.select
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
                let mask_type = self.value_type_for_value(mask)?;
                let then_type = self.value_type_for_value(then_value)?;
                let else_type = self.value_type_for_value(else_value)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let mask_layout = self.tensor_layout(pool, mask_type)?;
                let then_layout = self.tensor_layout(pool, then_type)?;
                let else_layout = self.tensor_layout(pool, else_type)?;

                pool.instruction_with_side(
                    Op::TensorSelect,
                    TensorSelect {
                        dest_offset: value_offset(self, destination)?,
                        mask_offset: value_offset(self, mask)?,
                        then_offset: value_offset(self, then_value)?,
                        else_offset: value_offset(self, else_value)?,
                        mask_layout,
                        then_layout,
                        else_layout,
                        dest_layout,
                    },
                )
            }
            // tensor.convert
            mir::Instruction::TensorConvert {
                destination,
                mode,
                tensor,
            } => {
                let destination = tensor_value(*destination, "tensor convert destination")?;
                let tensor = tensor_value(*tensor, "tensor convert source")?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let source_element =
                    tensor_element_type(self.tree, source_type).ok_or_else(|| {
                        Error::MissingRepresentation {
                            context: "tensor convert source element".to_string(),
                        }
                    })?;
                let dest_element = tensor_element_type(self.tree, dest_type).ok_or_else(|| {
                    Error::MissingRepresentation {
                        context: "tensor convert destination element".to_string(),
                    }
                })?;

                pool.instruction_with_side(
                    Op::TensorConvert,
                    TensorConvert {
                        dest_offset: value_offset(self, destination)?,
                        tensor_offset: value_offset(self, tensor)?,
                        source_layout,
                        dest_layout,
                        source_scalar: tensor_scalar_layout(self.tree, source_element)?,
                        dest_scalar: tensor_scalar_layout(self.tree, dest_element)?,
                        mode: *mode,
                    },
                )
            }
            // tensor.cast
            mir::Instruction::TensorCast {
                destination,
                tensor,
            } => {
                let destination = tensor_value(*destination, "tensor cast destination")?;
                let tensor = tensor_value(*tensor, "tensor cast source")?;
                let destination_type = self.value_type_for_value(destination)?;
                let byte_len = self.layout_for_type(destination_type)?.byte_len as u64;

                Instruction::new(
                    Op::TensorCast,
                    value_offset(self, destination)?,
                    value_offset(self, tensor)?,
                    byte_len as u32,
                    (byte_len >> 32) as u32,
                )
            }
            // tensor.view
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
                let arguments = self.word_offset_reference_range(
                    pool,
                    self.tree.get_arguments(*arguments),
                    "tensor view argument",
                )?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(view)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let (pointer_class, element) = self.tensor_element_projection(source_type)?;
                let op = tensor_view_op(pointer_class)?;
                let element = pool.projection(element);

                pool.instruction_with_side(
                    op,
                    TensorView {
                        dest_offset: word_offset(self, destination)?,
                        view_offset: word_offset(self, view)?,
                        arguments,
                        offsets_count: *offsets_count,
                        sizes_count: *sizes_count,
                        strides_count: *strides_count,
                        source_layout,
                        dest_layout,
                        element,
                    },
                )
            }
            _ => return Err(Error::InvalidInstruction),
        })
    }

    /// Return the backing pointer class and element projection for one tensor view.
    fn tensor_element_projection(
        &self,
        view_type: mir::LocalNodeId<mir::Type>,
    ) -> Result<(PointerClass, Projection)> {
        let element_type =
            tensor_element_type(self.tree, view_type).ok_or(Error::InvalidInstruction)?;
        let pointer_class =
            tensor_view_pointer_class(self.tree, view_type).ok_or(Error::InvalidInstruction)?;
        let projection = build_tensor_element_projection(self.tree, self.layouts(), element_type)
            .ok_or(Error::InvalidInstruction)?;

        Ok((pointer_class, projection))
    }

    /// Return the compiled tensor layout for one tensor type.
    fn tensor_layout(
        &self,
        pool: &mut Pool<'_, '_>,
        tensor_type: mir::LocalNodeId<mir::Type>,
    ) -> Result<TensorLayoutId> {
        let layout = TensorLayout::from_type(self.tree, self.layouts(), tensor_type)?;

        Ok(pool.tensor_layout(layout))
    }

    /// Return one side-table range of word frame offsets.
    fn word_offset_reference_range(
        &self,
        pool: &mut Pool<'_, '_>,
        values: &[mir::ValueReference],
        context: &'static str,
    ) -> Result<U32RangeId> {
        let mut offsets = Vec::with_capacity(values.len());
        for value in values {
            let value = tensor_value(*value, context)?;
            offsets.push(word_offset(self, value)?);
        }

        Ok(pool.u32_range(&offsets))
    }

    /// Return one side-table range of value frame offsets.
    fn value_offset_reference_range(
        &self,
        pool: &mut Pool<'_, '_>,
        values: &[mir::ValueReference],
        context: &'static str,
    ) -> Result<U32RangeId> {
        let mut offsets = Vec::with_capacity(values.len());
        for value in values {
            let value = tensor_value(*value, context)?;
            offsets.push(value_offset(self, value)?);
        }

        Ok(pool.u32_range(&offsets))
    }
}

/// Select the tensor load op for one view pointer class.
fn tensor_load_op(pointer_class: PointerClass) -> Result<Op> {
    match pointer_class {
        PointerClass::Heap | PointerClass::HeapAddress => Ok(Op::TensorLoadHeap),
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => Ok(Op::TensorLoadSharedHeap),
        PointerClass::Raw => Ok(Op::TensorLoadRaw),
        PointerClass::SharedRaw => Ok(Op::TensorLoadSharedRaw),
        PointerClass::Stack => Ok(Op::TensorLoadStack),
        PointerClass::Frame => Ok(Op::TensorLoadFrame),
        PointerClass::Static => Ok(Op::TensorLoadStatic),
        PointerClass::Unknown => Err(Error::InvalidInstruction),
    }
}

/// Select the tensor store op for one view pointer class.
fn tensor_store_op(pointer_class: PointerClass) -> Result<Op> {
    match pointer_class {
        PointerClass::Heap | PointerClass::HeapAddress => Ok(Op::TensorStoreHeap),
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => Ok(Op::TensorStoreSharedHeap),
        PointerClass::Raw => Ok(Op::TensorStoreRaw),
        PointerClass::SharedRaw => Ok(Op::TensorStoreSharedRaw),
        PointerClass::Stack => Ok(Op::TensorStoreStack),
        PointerClass::Frame => Ok(Op::TensorStoreFrame),
        PointerClass::Static => Ok(Op::TensorStoreStatic),
        PointerClass::Unknown => Err(Error::InvalidInstruction),
    }
}

/// Select the tensor fill op for one view pointer class.
fn tensor_fill_op(pointer_class: PointerClass) -> Result<Op> {
    match pointer_class {
        PointerClass::Heap | PointerClass::HeapAddress => Ok(Op::TensorFillHeap),
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => Ok(Op::TensorFillSharedHeap),
        PointerClass::Raw => Ok(Op::TensorFillRaw),
        PointerClass::SharedRaw => Ok(Op::TensorFillSharedRaw),
        PointerClass::Stack => Ok(Op::TensorFillStack),
        PointerClass::Frame => Ok(Op::TensorFillFrame),
        PointerClass::Static => Ok(Op::TensorFillStatic),
        PointerClass::Unknown => Err(Error::InvalidInstruction),
    }
}

/// Select the tensor copy op for one target and source pointer class.
fn tensor_copy_op(target: PointerClass, source: PointerClass) -> Result<Op> {
    match (target, source) {
        (
            PointerClass::Heap | PointerClass::HeapAddress,
            PointerClass::Heap | PointerClass::HeapAddress,
        ) => Ok(Op::TensorCopyHeapFromHeap),
        (
            PointerClass::Heap | PointerClass::HeapAddress,
            PointerClass::SharedHeap | PointerClass::SharedHeapAddress,
        ) => Ok(Op::TensorCopyHeapFromSharedHeap),
        (PointerClass::Heap | PointerClass::HeapAddress, PointerClass::Raw) => {
            Ok(Op::TensorCopyHeapFromRaw)
        }
        (PointerClass::Heap | PointerClass::HeapAddress, PointerClass::SharedRaw) => {
            Ok(Op::TensorCopyHeapFromSharedRaw)
        }
        (PointerClass::Heap | PointerClass::HeapAddress, PointerClass::Stack) => {
            Ok(Op::TensorCopyHeapFromStack)
        }
        (PointerClass::Heap | PointerClass::HeapAddress, PointerClass::Frame) => {
            Ok(Op::TensorCopyHeapFromFrame)
        }
        (PointerClass::Heap | PointerClass::HeapAddress, PointerClass::Static) => {
            Ok(Op::TensorCopyHeapFromStatic)
        }
        (
            PointerClass::SharedHeap | PointerClass::SharedHeapAddress,
            PointerClass::Heap | PointerClass::HeapAddress,
        ) => Ok(Op::TensorCopySharedHeapFromHeap),
        (
            PointerClass::SharedHeap | PointerClass::SharedHeapAddress,
            PointerClass::SharedHeap | PointerClass::SharedHeapAddress,
        ) => Ok(Op::TensorCopySharedHeapFromSharedHeap),
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, PointerClass::Raw) => {
            Ok(Op::TensorCopySharedHeapFromRaw)
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, PointerClass::SharedRaw) => {
            Ok(Op::TensorCopySharedHeapFromSharedRaw)
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, PointerClass::Stack) => {
            Ok(Op::TensorCopySharedHeapFromStack)
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, PointerClass::Frame) => {
            Ok(Op::TensorCopySharedHeapFromFrame)
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, PointerClass::Static) => {
            Ok(Op::TensorCopySharedHeapFromStatic)
        }
        (PointerClass::Raw, PointerClass::Heap | PointerClass::HeapAddress) => {
            Ok(Op::TensorCopyRawFromHeap)
        }
        (PointerClass::Raw, PointerClass::SharedHeap | PointerClass::SharedHeapAddress) => {
            Ok(Op::TensorCopyRawFromSharedHeap)
        }
        (PointerClass::Raw, PointerClass::Raw) => Ok(Op::TensorCopyRawFromRaw),
        (PointerClass::Raw, PointerClass::SharedRaw) => Ok(Op::TensorCopyRawFromSharedRaw),
        (PointerClass::Raw, PointerClass::Stack) => Ok(Op::TensorCopyRawFromStack),
        (PointerClass::Raw, PointerClass::Frame) => Ok(Op::TensorCopyRawFromFrame),
        (PointerClass::Raw, PointerClass::Static) => Ok(Op::TensorCopyRawFromStatic),
        (PointerClass::SharedRaw, PointerClass::Heap | PointerClass::HeapAddress) => {
            Ok(Op::TensorCopySharedRawFromHeap)
        }
        (PointerClass::SharedRaw, PointerClass::SharedHeap | PointerClass::SharedHeapAddress) => {
            Ok(Op::TensorCopySharedRawFromSharedHeap)
        }
        (PointerClass::SharedRaw, PointerClass::Raw) => Ok(Op::TensorCopySharedRawFromRaw),
        (PointerClass::SharedRaw, PointerClass::SharedRaw) => {
            Ok(Op::TensorCopySharedRawFromSharedRaw)
        }
        (PointerClass::SharedRaw, PointerClass::Stack) => Ok(Op::TensorCopySharedRawFromStack),
        (PointerClass::SharedRaw, PointerClass::Frame) => Ok(Op::TensorCopySharedRawFromFrame),
        (PointerClass::SharedRaw, PointerClass::Static) => Ok(Op::TensorCopySharedRawFromStatic),
        (PointerClass::Stack, PointerClass::Heap | PointerClass::HeapAddress) => {
            Ok(Op::TensorCopyStackFromHeap)
        }
        (PointerClass::Stack, PointerClass::SharedHeap | PointerClass::SharedHeapAddress) => {
            Ok(Op::TensorCopyStackFromSharedHeap)
        }
        (PointerClass::Stack, PointerClass::Raw) => Ok(Op::TensorCopyStackFromRaw),
        (PointerClass::Stack, PointerClass::SharedRaw) => Ok(Op::TensorCopyStackFromSharedRaw),
        (PointerClass::Stack, PointerClass::Stack) => Ok(Op::TensorCopyStackFromStack),
        (PointerClass::Stack, PointerClass::Frame) => Ok(Op::TensorCopyStackFromFrame),
        (PointerClass::Stack, PointerClass::Static) => Ok(Op::TensorCopyStackFromStatic),
        (PointerClass::Frame, PointerClass::Heap | PointerClass::HeapAddress) => {
            Ok(Op::TensorCopyFrameFromHeap)
        }
        (PointerClass::Frame, PointerClass::SharedHeap | PointerClass::SharedHeapAddress) => {
            Ok(Op::TensorCopyFrameFromSharedHeap)
        }
        (PointerClass::Frame, PointerClass::Raw) => Ok(Op::TensorCopyFrameFromRaw),
        (PointerClass::Frame, PointerClass::SharedRaw) => Ok(Op::TensorCopyFrameFromSharedRaw),
        (PointerClass::Frame, PointerClass::Stack) => Ok(Op::TensorCopyFrameFromStack),
        (PointerClass::Frame, PointerClass::Frame) => Ok(Op::TensorCopyFrameFromFrame),
        (PointerClass::Frame, PointerClass::Static) => Ok(Op::TensorCopyFrameFromStatic),
        (PointerClass::Static, PointerClass::Heap | PointerClass::HeapAddress) => {
            Ok(Op::TensorCopyStaticFromHeap)
        }
        (PointerClass::Static, PointerClass::SharedHeap | PointerClass::SharedHeapAddress) => {
            Ok(Op::TensorCopyStaticFromSharedHeap)
        }
        (PointerClass::Static, PointerClass::Raw) => Ok(Op::TensorCopyStaticFromRaw),
        (PointerClass::Static, PointerClass::SharedRaw) => Ok(Op::TensorCopyStaticFromSharedRaw),
        (PointerClass::Static, PointerClass::Stack) => Ok(Op::TensorCopyStaticFromStack),
        (PointerClass::Static, PointerClass::Frame) => Ok(Op::TensorCopyStaticFromFrame),
        (PointerClass::Static, PointerClass::Static) => Ok(Op::TensorCopyStaticFromStatic),
        (PointerClass::Unknown, _) | (_, PointerClass::Unknown) => Err(Error::InvalidInstruction),
    }
}

/// Select the tensor view op for one view pointer class.
fn tensor_view_op(pointer_class: PointerClass) -> Result<Op> {
    match pointer_class {
        PointerClass::Heap | PointerClass::HeapAddress => Ok(Op::TensorViewHeap),
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => Ok(Op::TensorViewSharedHeap),
        PointerClass::Raw => Ok(Op::TensorViewRaw),
        PointerClass::SharedRaw => Ok(Op::TensorViewSharedRaw),
        PointerClass::Stack => Ok(Op::TensorViewStack),
        PointerClass::Frame => Ok(Op::TensorViewFrame),
        PointerClass::Static => Ok(Op::TensorViewStatic),
        PointerClass::Unknown => Err(Error::InvalidInstruction),
    }
}

/// Return the scalar layout for one tensor element type.
pub(super) fn tensor_scalar_layout(
    tree: &mir::Tree,
    element: mir::LocalNodeId<mir::Type>,
) -> Result<ScalarLayout> {
    scalar_layout_from_type(tree, element).ok_or_else(|| Error::TypeMismatch {
        expected: "tensor scalar element".to_string(),
        actual: format!("{element:?}"),
    })
}

/// Return one required tensor value.
fn tensor_value(reference: mir::ValueReference, context: &'static str) -> Result<mir::Value> {
    reference
        .value()
        .ok_or_else(|| Error::MissingRepresentation {
            context: context.into(),
        })
}

/// Return one required tensor value from an argument slice.
fn tensor_value_ref(reference: mir::ValueReference, context: &'static str) -> Result<mir::Value> {
    tensor_value(reference, context)
}
