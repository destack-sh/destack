use destack_mir as mir;

use crate::LinkResult;

use destack_program::CellLayout;
use destack_program::vm::{
    Instruction, Op, Projection, TensorAddress, TensorBinary, TensorBroadcast, TensorConcat,
    TensorContiguousBinary, TensorConvert, TensorConvolution, TensorConvolutionDimensionsBuilder,
    TensorConvolutionWindowBuilder, TensorCopy, TensorDot, TensorDotDimensionsBuilder,
    TensorExtract, TensorFill, TensorGather, TensorGatherDimensionsBuilder, TensorIndexReduce,
    TensorLayoutBuilder, TensorLayoutId, TensorLoad, TensorPad, TensorReduce, TensorReshape,
    TensorScatter, TensorScatterDimensionsBuilder, TensorSelect, TensorSlice, TensorStore,
    TensorTranspose, TensorView, U32RangeId, column_major_strides, row_major_strides,
    tensor_element_count, tensor_element_span_len,
};

use super::arithmetic::element_binary_kernel;
use super::lower::BlockLowerer;
use super::pool::Pool;

impl<'a> BlockLowerer<'a> {
    /// Lower one tensor instruction.
    pub(super) fn lower_tensor(
        &self,
        inst: &mir::Instruction,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        Ok(match inst {
            // tensor.splat
            mir::Instruction::TensorSplat { destination, value } => {
                let destination = *destination;
                let value = *value;
                let tensor_type = self.value_type_for_value(destination)?;
                let tensor_layout = self.tensor_layout(pool, tensor_type)?;

                Instruction::new(
                    Op::TensorSplat,
                    self.value_offset(destination)?,
                    self.cell_offset(value)?,
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
                let destination = *destination;
                let view = *view;
                let view_type = self.value_type_for_value(view)?;
                let view_layout = self.tensor_layout(pool, view_type)?;
                let indices =
                    self.cell_offset_range(pool, self.function.tree.get_values(*indices))?;
                let (pointer, element) = self.tensor_view_projection(view_type)?;
                let address = self.tensor_address(pointer)?;
                let element = pool.projection(element);

                pool.instruction_with_side(
                    Op::TensorLoad,
                    TensorLoad {
                        dest_offset: self.cell_offset(destination)?,
                        view_offset: self.value_offset(view)?,
                        indices,
                        view_layout,
                        element,
                        address,
                    },
                )
            }
            // tensor.extract
            mir::Instruction::TensorExtract {
                destination,
                tensor,
                indices,
            } => {
                let destination = *destination;
                let tensor = *tensor;
                let tensor_type = self.value_type_for_value(tensor)?;
                let tensor_layout = self.tensor_layout(pool, tensor_type)?;
                let indices =
                    self.cell_offset_range(pool, self.function.tree.get_values(*indices))?;

                pool.instruction_with_side(
                    Op::TensorExtract,
                    TensorExtract {
                        dest_offset: self.cell_offset(destination)?,
                        tensor_offset: self.value_offset(tensor)?,
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
                let view = *view;
                let value = *value;
                let view_type = self.value_type_for_value(view)?;
                let view_layout = self.tensor_layout(pool, view_type)?;
                let indices =
                    self.cell_offset_range(pool, self.function.tree.get_values(*indices))?;
                let (pointer, element) = self.tensor_view_projection(view_type)?;
                let address = self.tensor_address(pointer)?;
                let element = pool.projection(element);

                pool.instruction_with_side(
                    Op::TensorStore,
                    TensorStore {
                        view_offset: self.value_offset(view)?,
                        indices,
                        value_offset: self.cell_offset(value)?,
                        view_layout,
                        element,
                        address,
                    },
                )
            }
            // tensor.fill
            mir::Instruction::TensorFill { view, value } => {
                let view = *view;
                let value = *value;
                let view_type = self.value_type_for_value(view)?;
                let view_layout = self.tensor_layout(pool, view_type)?;
                let (pointer, element) = self.tensor_view_projection(view_type)?;
                let address = self.tensor_address(pointer)?;
                let element = pool.projection(element);

                pool.instruction_with_side(
                    Op::TensorFill,
                    TensorFill {
                        view_offset: self.value_offset(view)?,
                        value_offset: self.cell_offset(value)?,
                        view_layout,
                        element,
                        address,
                    },
                )
            }
            // tensor.copy
            mir::Instruction::TensorCopy { target, source } => {
                let target = *target;
                let source = *source;
                let target_type = self.value_type_for_value(target)?;
                let source_type = self.value_type_for_value(source)?;
                let target_layout = self.tensor_layout(pool, target_type)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let (target_class, target_element) = self.tensor_view_projection(target_type)?;
                let (source_class, source_element) = self.tensor_view_projection(source_type)?;
                let target_address = self.tensor_address(target_class)?;
                let source_address = self.tensor_address(source_class)?;
                let target_element = pool.projection(target_element);
                let source_element = pool.projection(source_element);

                pool.instruction_with_side(
                    Op::TensorCopy,
                    TensorCopy {
                        target_offset: self.value_offset(target)?,
                        source_offset: self.value_offset(source)?,
                        target_layout,
                        source_layout,
                        target_element,
                        source_element,
                        target_address,
                        source_address,
                    },
                )
            }
            // tensor.reshape
            mir::Instruction::TensorReshape {
                destination,
                tensor,
                shape,
            } => {
                let destination = *destination;
                let tensor = *tensor;
                let source_type = self.value_type_for_value(tensor)?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let shape = self.cell_offset_range(pool, self.function.tree.get_values(*shape))?;

                pool.instruction_with_side(
                    Op::TensorReshape,
                    TensorReshape {
                        dest_offset: self.value_offset(destination)?,
                        tensor_offset: self.value_offset(tensor)?,
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
                let destination = *destination;
                let tensor = *tensor;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let dimensions = pool.u32_range(self.function.indices(*dimensions));

                pool.instruction_with_side(
                    Op::TensorBroadcast,
                    TensorBroadcast {
                        dest_offset: self.value_offset(destination)?,
                        tensor_offset: self.value_offset(tensor)?,
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
                let destination = *destination;
                let tensor = *tensor;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let permutation = pool.u32_range(self.function.indices(*permutation));

                pool.instruction_with_side(
                    Op::TensorTranspose,
                    TensorTranspose {
                        dest_offset: self.value_offset(destination)?,
                        tensor_offset: self.value_offset(tensor)?,
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
                let destination = *destination;
                let tensor = *tensor;
                let arguments =
                    self.cell_offset_range(pool, self.function.tree.get_values(*arguments))?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;

                pool.instruction_with_side(
                    Op::TensorSlice,
                    TensorSlice {
                        dest_offset: self.value_offset(destination)?,
                        tensor_offset: self.value_offset(tensor)?,
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
                let destination = *destination;
                let tensor = *tensor;
                let value = *value;
                let arguments =
                    self.cell_offset_range(pool, self.function.tree.get_values(*arguments))?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;

                pool.instruction_with_side(
                    Op::TensorPad,
                    TensorPad {
                        dest_offset: self.value_offset(destination)?,
                        tensor_offset: self.value_offset(tensor)?,
                        arguments,
                        low_count: *low_count,
                        high_count: *high_count,
                        interior_count: *interior_count,
                        value_offset: self.cell_offset(value)?,
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
                let destination = *destination;
                let tensor_value = self.function.tree.get_values(*tensors);
                let tensors = self.value_offset_range(pool, tensor_value)?;
                let mut tensor_layouts = Vec::with_capacity(tensor_value.len());
                for value in tensor_value {
                    let value_type = self.value_type_for_value(*value)?;
                    tensor_layouts.push(self.tensor_layout(pool, value_type)?.0);
                }
                let dest_type = self.value_type_for_value(destination)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let tensor_layouts = pool.u32_range(&tensor_layouts);

                pool.instruction_with_side(
                    Op::TensorConcat,
                    TensorConcat {
                        dest_offset: self.value_offset(destination)?,
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
                let destination = *destination;
                let tensor = *tensor;
                let initial = *initial;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let axes = pool.u32_range(self.function.indices(*axes));

                pool.instruction_with_side(
                    Op::TensorReduce,
                    TensorReduce {
                        dest_offset: self.value_offset(destination)?,
                        tensor_offset: self.value_offset(tensor)?,
                        initial_offset: self.cell_offset(initial)?,
                        axes,
                        source_layout,
                        dest_layout,
                        kernel: *operator,
                    },
                )
            }
            // tensor.indexReduce
            mir::Instruction::TensorIndexReduce {
                destination,
                operator,
                tensor,
                axis,
                tie_break,
            } => {
                let destination = *destination;
                let tensor = *tensor;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;

                pool.instruction_with_side(
                    Op::TensorIndexReduce,
                    TensorIndexReduce {
                        dest_offset: self.value_offset(destination)?,
                        tensor_offset: self.value_offset(tensor)?,
                        axis: *axis,
                        source_layout,
                        dest_layout,
                        kernel: *operator,
                        tie_break: *tie_break,
                    },
                )
            }
            // tensor.dot
            mir::Instruction::TensorDot {
                destination,
                left,
                right,
                immediate,
            } => {
                let destination = *destination;
                let left = *left;
                let right = *right;
                let dest_type = self.value_type_for_value(destination)?;
                let left_type = self.value_type_for_value(left)?;
                let right_type = self.value_type_for_value(right)?;
                let dest_layout = self.tensor_layout_from_type(dest_type)?;
                let left_layout = self.tensor_layout_from_type(left_type)?;
                let right_layout = self.tensor_layout_from_type(right_type)?;
                let dimensions = pool.tensor_dot(self.tensor_dot(*immediate)?);
                let element = self
                    .tensor_element_type(dest_type)
                    .ok_or_else(|| self.invalid_instruction("tensor dot element"))?;
                let element_layout = self
                    .function
                    .require_scalar_format(element, "tensor scalar element")?;

                let dest_layout = pool.tensor_layout(dest_layout);
                let left_layout = pool.tensor_layout(left_layout);
                let right_layout = pool.tensor_layout(right_layout);

                pool.instruction_with_side(
                    Op::TensorDot,
                    TensorDot {
                        dest_offset: self.value_offset(destination)?,
                        left_offset: self.value_offset(left)?,
                        right_offset: self.value_offset(right)?,
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
                immediate,
            } => {
                let destination = *destination;
                let input = *input;
                let kernel = *kernel;
                let dest_type = self.value_type_for_value(destination)?;
                let input_type = self.value_type_for_value(input)?;
                let kernel_type = self.value_type_for_value(kernel)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let input_layout = self.tensor_layout(pool, input_type)?;
                let kernel_layout = self.tensor_layout(pool, kernel_type)?;
                let (dimensions, window, feature_group_count, batch_group_count) =
                    self.tensor_convolution(*immediate)?;
                let dimensions = pool.tensor_convolution(dimensions);
                let window = pool.tensor_window(window);
                let element = self
                    .tensor_element_type(dest_type)
                    .ok_or_else(|| self.invalid_instruction("tensor convolution element"))?;
                let element_layout = self
                    .function
                    .require_scalar_format(element, "tensor scalar element")?;

                pool.instruction_with_side(
                    Op::TensorConvolution,
                    TensorConvolution {
                        dest_offset: self.value_offset(destination)?,
                        input_offset: self.value_offset(input)?,
                        kernel_offset: self.value_offset(kernel)?,
                        dimensions,
                        window,
                        feature_group_count,
                        batch_group_count,
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
                immediate,
            } => {
                let destination = *destination;
                let source = *source;
                let indices = *indices;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(source)?;
                let indices_type = self.value_type_for_value(indices)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let indices_layout = self.tensor_layout(pool, indices_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let (dimensions, slice_sizes) = self.tensor_gather(*immediate)?;
                let dimensions = pool.tensor_gather(dimensions);
                let slice_sizes = pool.u32_range(&slice_sizes);

                pool.instruction_with_side(
                    Op::TensorGather,
                    TensorGather {
                        dest_offset: self.value_offset(destination)?,
                        source_offset: self.value_offset(source)?,
                        indices_offset: self.value_offset(indices)?,
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
                immediate,
                mode,
            } => {
                let destination = *destination;
                let source = *source;
                let indices = *indices;
                let updates = *updates;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(source)?;
                let indices_type = self.value_type_for_value(indices)?;
                let updates_type = self.value_type_for_value(updates)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let indices_layout = self.tensor_layout(pool, indices_type)?;
                let updates_layout = self.tensor_layout(pool, updates_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let dimensions = pool.tensor_scatter(self.tensor_scatter(*immediate)?);
                let element = self
                    .tensor_element_type(dest_type)
                    .ok_or_else(|| self.invalid_instruction("tensor scatter element"))?;
                let element_layout = self
                    .function
                    .require_scalar_format(element, "tensor scalar element")?;

                pool.instruction_with_side(
                    Op::TensorScatter,
                    TensorScatter {
                        dest_offset: self.value_offset(destination)?,
                        source_offset: self.value_offset(source)?,
                        indices_offset: self.value_offset(indices)?,
                        updates_offset: self.value_offset(updates)?,
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
                let destination = *destination;
                let left = *left;
                let right = *right;
                let dest_type = self.value_type_for_value(destination)?;
                let left_type = self.value_type_for_value(left)?;
                let right_type = self.value_type_for_value(right)?;
                let dest_layout = self.tensor_layout_from_type(dest_type)?;
                let left_layout = self.tensor_layout_from_type(left_type)?;
                let right_layout = self.tensor_layout_from_type(right_type)?;
                let element = self
                    .tensor_element_type(left_type)
                    .ok_or_else(|| self.invalid_instruction("tensor compare element"))?;
                let element_layout = self
                    .function
                    .operand_for_type(element)
                    .ok_or_else(|| self.invalid_instruction("tensor compare element layout"))?;
                let kernel = element_binary_kernel(*operator, element_layout)
                    .ok_or_else(|| self.invalid_instruction("tensor compare operator"))?;
                if dest_layout.has_same_contiguous_order(&left_layout)
                    && dest_layout.has_same_contiguous_order(&right_layout)
                {
                    let dest_layout = pool.tensor_layout(dest_layout);

                    return Ok(pool.instruction_with_side(
                        Op::TensorContiguousBinary,
                        TensorContiguousBinary {
                            dest_offset: self.value_offset(destination)?,
                            left_offset: self.value_offset(left)?,
                            right_offset: self.value_offset(right)?,
                            dest_layout,
                            element_layout: self
                                .function
                                .require_scalar_format(element, "tensor scalar element")?,
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
                        dest_offset: self.value_offset(destination)?,
                        left_offset: self.value_offset(left)?,
                        right_offset: self.value_offset(right)?,
                        left_layout,
                        right_layout,
                        dest_layout,
                        kernel,
                        element_layout: self
                            .function
                            .require_scalar_format(element, "tensor scalar element")?,
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
                let destination = *destination;
                let mask = *mask;
                let then_value = *then_value;
                let else_value = *else_value;
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
                        dest_offset: self.value_offset(destination)?,
                        mask_offset: self.value_offset(mask)?,
                        then_offset: self.value_offset(then_value)?,
                        else_offset: self.value_offset(else_value)?,
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
                let destination = *destination;
                let tensor = *tensor;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let source_element = self
                    .tensor_element_type(source_type)
                    .ok_or_else(|| self.invalid_instruction("tensor convert source element"))?;
                let dest_element = self.tensor_element_type(dest_type).ok_or_else(|| {
                    self.invalid_instruction("tensor convert destination element")
                })?;

                pool.instruction_with_side(
                    Op::TensorConvert,
                    TensorConvert {
                        dest_offset: self.value_offset(destination)?,
                        tensor_offset: self.value_offset(tensor)?,
                        source_layout,
                        dest_layout,
                        source_scalar: self
                            .function
                            .require_scalar_format(source_element, "tensor scalar element")?,
                        dest_scalar: self
                            .function
                            .require_scalar_format(dest_element, "tensor scalar element")?,
                        mode: *mode,
                    },
                )
            }
            // tensor.cast
            mir::Instruction::TensorCast {
                destination,
                tensor,
            } => {
                let destination = *destination;
                let tensor = *tensor;

                Instruction::new(
                    Op::TensorCast,
                    self.value_offset(destination)?,
                    self.value_offset(tensor)?,
                    0,
                    0,
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
                let destination = *destination;
                let view = *view;
                let arguments =
                    self.cell_offset_range(pool, self.function.tree.get_values(*arguments))?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(view)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let (pointer, element) = self.tensor_view_projection(source_type)?;
                let address = self.tensor_address(pointer)?;
                let element = pool.projection(element);

                pool.instruction_with_side(
                    Op::TensorView,
                    TensorView {
                        dest_offset: self.value_offset(destination)?,
                        view_offset: self.value_offset(view)?,
                        arguments,
                        offsets_count: *offsets_count,
                        sizes_count: *sizes_count,
                        strides_count: *strides_count,
                        source_layout,
                        dest_layout,
                        element,
                        address,
                    },
                )
            }
            _ => return Err(self.invalid_instruction("tensor instruction")),
        })
    }

    /// Return one tensor address class for one pointer layout.
    fn tensor_address(&self, pointer: CellLayout) -> LinkResult<TensorAddress> {
        TensorAddress::from_cell_layout(pointer)
            .map_err(|_| self.invalid_pointer_type(format!("{pointer:?}")))
    }

    /// Return the backing pointer layout and element projection for one tensor view.
    fn tensor_view_projection(
        &self,
        view_type: mir::LocalNodeId<mir::Type>,
    ) -> LinkResult<(CellLayout, Projection)> {
        let element_type = self
            .tensor_element_type(view_type)
            .ok_or_else(|| self.invalid_instruction("tensor view element"))?;
        let pointer = self
            .tensor_view_cell_layout(view_type)
            .ok_or_else(|| self.invalid_instruction("tensor view pointer layout"))?;
        let projection = self
            .tensor_element_projection(element_type)
            .ok_or_else(|| self.invalid_instruction("tensor view element projection"))?;

        Ok((pointer, projection))
    }

    /// Return the VM tensor layout for one tensor type.
    pub(super) fn tensor_layout(
        &self,
        pool: &mut Pool<'_, '_>,
        tensor_type: mir::LocalNodeId<mir::Type>,
    ) -> LinkResult<TensorLayoutId> {
        let layout = self.tensor_layout_from_type(tensor_type)?;

        Ok(pool.tensor_layout(layout))
    }

    /// Compile one tensor layout from one MIR tensor type.
    pub(super) fn tensor_layout_from_type(
        &self,
        tensor_type: mir::LocalNodeId<mir::Type>,
    ) -> LinkResult<TensorLayoutBuilder> {
        let (shape, element) = match self.function.tree.get(tensor_type) {
            mir::Type::Tensor { shape, element, .. } => (shape, element),
            mir::Type::TensorView { shape, element, .. } => (shape, element),
            _ => {
                return Err(self.type_mismatch("tensor type", format!("{tensor_type:?}")));
            }
        };
        let element_type = *element;

        let shape = self.static_shape(shape)?;
        let strides = self.static_tensor_strides(tensor_type, &shape)?;
        let element_count = tensor_element_count(&shape);
        let element_span_len = tensor_element_span_len(&shape, &strides)
            .map_err(|_| self.layout_overflow("tensor element span"))?;
        let is_contiguous = element_count == element_span_len;

        let element_layout = self
            .layouts()
            .get(&element_type)
            .ok_or_else(|| self.invalid_instruction("tensor element layout"))?;
        let element = Projection::indexed(
            self.function.program.type_id(element_type),
            element_span_len as u64,
            element_layout.stride(),
            element_layout.byte_len(),
            self.function.cell_layout_for_type(element_type),
        );
        let element_layout = self
            .function
            .scalar_layout_for_type(element_type)
            .ok_or_else(|| self.type_mismatch("tensor scalar element", format!("{element:?}")))?;
        let byte_len = element_span_len
            .checked_mul(element.byte_stride())
            .ok_or_else(|| self.layout_overflow("tensor payload byte length"))?;

        Ok(TensorLayoutBuilder {
            byte_len: byte_len as u64,
            shape,
            strides,
            element_count: element_count as u64,
            element_span_len: element_span_len as u64,
            is_contiguous,
            element_layout,
            element,
        })
    }

    /// Return a static tensor shape.
    fn static_shape(&self, shape: &[mir::TensorDimension]) -> LinkResult<Vec<u64>> {
        let mut dims = Vec::with_capacity(shape.len());
        for dim in shape {
            match dim {
                mir::TensorDimension::Static(value) => dims.push(*value),
                mir::TensorDimension::Dynamic => {
                    return Err(self.invalid_instruction("tensor dynamic shape"));
                }
                mir::TensorDimension::Symbol(name) => {
                    return Err(self.invalid_instruction(format!("tensor symbolic shape {name}")));
                }
            }
        }

        Ok(dims)
    }

    /// Compile one static tensor stride list.
    fn static_tensor_strides(
        &self,
        tensor_type: mir::LocalNodeId<mir::Type>,
        shape: &[u64],
    ) -> LinkResult<Vec<u64>> {
        match self.function.tree.get(tensor_type) {
            mir::Type::Tensor { format, .. } => match format {
                mir::TensorFormat::Dense {
                    order: mir::TensorDimensionOrder::RowMajor,
                } => Ok(row_major_strides(shape)),
                mir::TensorFormat::Dense {
                    order: mir::TensorDimensionOrder::ColumnMajor,
                } => Ok(column_major_strides(shape)),
            },
            mir::Type::TensorView { format, .. } => match format {
                mir::TensorViewFormat::Dense {
                    order: mir::TensorDimensionOrder::RowMajor,
                }
                | mir::TensorViewFormat::Strided => Ok(row_major_strides(shape)),
                mir::TensorViewFormat::Dense {
                    order: mir::TensorDimensionOrder::ColumnMajor,
                } => Ok(column_major_strides(shape)),
            },
            _ => Err(self.invalid_instruction("tensor strides")),
        }
    }

    /// Return one side-table range of cell frame offsets.
    fn cell_offset_range(
        &self,
        pool: &mut Pool<'_, '_>,
        values: &[mir::Value],
    ) -> LinkResult<U32RangeId> {
        let mut offsets = Vec::with_capacity(values.len());
        for value in values {
            offsets.push(self.cell_offset(*value)?);
        }

        Ok(pool.u32_range(&offsets))
    }

    /// Return one side-table range of value frame offsets.
    fn value_offset_range(
        &self,
        pool: &mut Pool<'_, '_>,
        values: &[mir::Value],
    ) -> LinkResult<U32RangeId> {
        let mut offsets = Vec::with_capacity(values.len());
        for value in values {
            offsets.push(self.value_offset(*value)?);
        }

        Ok(pool.u32_range(&offsets))
    }

    /// Return VM tensor dot dimensions from one MIR immediate.
    fn tensor_dot(
        &self,
        immediate: mir::TensorImmediateId,
    ) -> LinkResult<TensorDotDimensionsBuilder> {
        let mir::TensorImmediate::Dot {
            lhs_batch,
            rhs_batch,
            lhs_contracting,
            rhs_contracting,
        } = self.function.tree.get_tensor_immediate(immediate)
        else {
            return Err(self.invalid_instruction("tensor dot immediate"));
        };

        Ok(TensorDotDimensionsBuilder {
            lhs_batch: self.function.indices(*lhs_batch).to_vec(),
            rhs_batch: self.function.indices(*rhs_batch).to_vec(),
            lhs_contracting: self.function.indices(*lhs_contracting).to_vec(),
            rhs_contracting: self.function.indices(*rhs_contracting).to_vec(),
        })
    }

    /// Return VM tensor convolution descriptors from one MIR immediate.
    fn tensor_convolution(
        &self,
        immediate: mir::TensorImmediateId,
    ) -> LinkResult<(
        TensorConvolutionDimensionsBuilder,
        TensorConvolutionWindowBuilder,
        u32,
        u32,
    )> {
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
        } = self.function.tree.get_tensor_immediate(immediate)
        else {
            return Err(self.invalid_instruction("tensor convolution immediate"));
        };

        let dimensions = TensorConvolutionDimensionsBuilder {
            input_batch: *input_batch,
            input_feature: *input_feature,
            input_spatial: self.function.indices(*input_spatial).to_vec(),
            kernel_input_feature: *kernel_input_feature,
            kernel_output_feature: *kernel_output_feature,
            kernel_spatial: self.function.indices(*kernel_spatial).to_vec(),
            output_batch: *output_batch,
            output_feature: *output_feature,
            output_spatial: self.function.indices(*output_spatial).to_vec(),
        };
        let window = TensorConvolutionWindowBuilder {
            strides: self.function.tree.get_extents(*strides).to_vec(),
            padding_low: self.function.tree.get_extents(*padding_low).to_vec(),
            padding_high: self.function.tree.get_extents(*padding_high).to_vec(),
            lhs_dilation: self.function.tree.get_extents(*lhs_dilation).to_vec(),
            rhs_dilation: self.function.tree.get_extents(*rhs_dilation).to_vec(),
            window_reversal: self
                .function
                .tree
                .get_flags(*window_reversal)
                .iter()
                .map(|flag| *flag != 0)
                .collect(),
        };

        Ok((dimensions, window, *feature_group_count, *batch_group_count))
    }

    /// Return VM tensor gather descriptors from one MIR immediate.
    fn tensor_gather(
        &self,
        immediate: mir::TensorImmediateId,
    ) -> LinkResult<(TensorGatherDimensionsBuilder, Vec<u32>)> {
        let mir::TensorImmediate::Gather {
            offset_dims,
            collapsed_slice_dims,
            start_index_map,
            index_vector_dim,
            slice_sizes,
        } = self.function.tree.get_tensor_immediate(immediate)
        else {
            return Err(self.invalid_instruction("tensor gather immediate"));
        };

        Ok((
            TensorGatherDimensionsBuilder {
                offset_dims: self.function.indices(*offset_dims).to_vec(),
                collapsed_slice_dims: self.function.indices(*collapsed_slice_dims).to_vec(),
                start_index_map: self.function.indices(*start_index_map).to_vec(),
                index_vector_dim: *index_vector_dim,
            },
            self.function.indices(*slice_sizes).to_vec(),
        ))
    }

    /// Return VM tensor scatter dimensions from one MIR immediate.
    fn tensor_scatter(
        &self,
        immediate: mir::TensorImmediateId,
    ) -> LinkResult<TensorScatterDimensionsBuilder> {
        let mir::TensorImmediate::Scatter {
            update_window_dims,
            inserted_window_dims,
            scatter_dims_to_operand_dims,
            index_vector_dim,
        } = self.function.tree.get_tensor_immediate(immediate)
        else {
            return Err(self.invalid_instruction("tensor scatter immediate"));
        };

        Ok(TensorScatterDimensionsBuilder {
            update_window_dims: self.function.indices(*update_window_dims).to_vec(),
            inserted_window_dims: self.function.indices(*inserted_window_dims).to_vec(),
            scatter_dims_to_operand_dims: self
                .function
                .indices(*scatter_dims_to_operand_dims)
                .to_vec(),
            index_vector_dim: *index_vector_dim,
        })
    }
}
