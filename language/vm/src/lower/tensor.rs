use destack_mir as mir;

use crate::{Error, Result};
use destack_program::vm::{
    AddressSpace, Instruction, Op, Projection, ScalarLayout, TensorAddress, TensorBinary,
    TensorBroadcast, TensorConcat, TensorContiguousBinary, TensorConvert, TensorConvolution,
    TensorCopy, TensorDot, TensorExtract, TensorFill, TensorGather, TensorIndexReduce,
    TensorLayout, TensorLayoutId, TensorLoad, TensorPad, TensorReduce, TensorReshape,
    TensorScatter, TensorSelect, TensorSlice, TensorStore, TensorTranspose, TensorView, U32RangeId,
    cell_layout_from_type, column_major_strides, row_major_strides, scalar_layout_from_type,
    static_shape, tensor_element_count, tensor_element_span_len,
};

use super::arithmetic::{element_binary_kernel, same_contiguous_tensor_order};
use super::frame::{cell_offset, value_offset};
use super::lower::BlockLowerer;
use super::pool::Pool;
use super::projection::{
    tensor_element_projection as build_tensor_element_projection, tensor_element_type,
    tensor_view_address_space,
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
                let destination = *destination;
                let value = *value;
                let tensor_type = self.value_type_for_value(destination)?;
                let tensor_layout = self.tensor_layout(pool, tensor_type)?;

                Instruction::new(
                    Op::TensorSplat,
                    value_offset(self, destination)?,
                    cell_offset(self, value)?,
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
                let indices = self.cell_offset_range(pool, self.tree.get_values(*indices))?;
                let (address_space, element) = self.tensor_element_projection(view_type)?;
                let address = TensorAddress::from_address_space(address_space)?;
                let element = pool.projection(element);

                pool.instruction_with_side(
                    Op::TensorLoad,
                    TensorLoad {
                        dest_offset: cell_offset(self, destination)?,
                        view_offset: value_offset(self, view)?,
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
                let indices = self.cell_offset_range(pool, self.tree.get_values(*indices))?;

                pool.instruction_with_side(
                    Op::TensorExtract,
                    TensorExtract {
                        dest_offset: cell_offset(self, destination)?,
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
                let view = *view;
                let value = *value;
                let view_type = self.value_type_for_value(view)?;
                let view_layout = self.tensor_layout(pool, view_type)?;
                let indices = self.cell_offset_range(pool, self.tree.get_values(*indices))?;
                let (address_space, element) = self.tensor_element_projection(view_type)?;
                let address = TensorAddress::from_address_space(address_space)?;
                let element = pool.projection(element);

                pool.instruction_with_side(
                    Op::TensorStore,
                    TensorStore {
                        view_offset: value_offset(self, view)?,
                        indices,
                        value_offset: cell_offset(self, value)?,
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
                let (address_space, element) = self.tensor_element_projection(view_type)?;
                let address = TensorAddress::from_address_space(address_space)?;
                let element = pool.projection(element);

                pool.instruction_with_side(
                    Op::TensorFill,
                    TensorFill {
                        view_offset: value_offset(self, view)?,
                        value_offset: cell_offset(self, value)?,
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
                let (target_class, target_element) = self.tensor_element_projection(target_type)?;
                let (source_class, source_element) = self.tensor_element_projection(source_type)?;
                let target_address = TensorAddress::from_address_space(target_class)?;
                let source_address = TensorAddress::from_address_space(source_class)?;
                let target_element = pool.projection(target_element);
                let source_element = pool.projection(source_element);

                pool.instruction_with_side(
                    Op::TensorCopy,
                    TensorCopy {
                        target_offset: value_offset(self, target)?,
                        source_offset: value_offset(self, source)?,
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
                let shape = self.cell_offset_range(pool, self.tree.get_values(*shape))?;

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
                let destination = *destination;
                let tensor = *tensor;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let dimensions = pool.u32_range(self.indices(*dimensions));

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
                let destination = *destination;
                let tensor = *tensor;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let permutation = pool.u32_range(self.indices(*permutation));

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
                let destination = *destination;
                let tensor = *tensor;
                let arguments = self.cell_offset_range(pool, self.tree.get_values(*arguments))?;
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
                let destination = *destination;
                let tensor = *tensor;
                let value = *value;
                let arguments = self.cell_offset_range(pool, self.tree.get_values(*arguments))?;
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
                        value_offset: cell_offset(self, value)?,
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
                let tensor_value = self.tree.get_values(*tensors);
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
                let destination = *destination;
                let tensor = *tensor;
                let initial = *initial;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let axes = pool.u32_range(self.indices(*axes));

                pool.instruction_with_side(
                    Op::TensorReduce,
                    TensorReduce {
                        dest_offset: value_offset(self, destination)?,
                        tensor_offset: value_offset(self, tensor)?,
                        initial_offset: cell_offset(self, initial)?,
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
                        dest_offset: value_offset(self, destination)?,
                        tensor_offset: value_offset(self, tensor)?,
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
                let element = tensor_element_type(self.tree, dest_type)
                    .ok_or_else(|| Error::invalid_program("tensor dot element"))?;
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
                let element = tensor_element_type(self.tree, dest_type)
                    .ok_or_else(|| Error::invalid_program("tensor convolution element"))?;
                let element_layout = tensor_scalar_layout(self.tree, element)?;

                pool.instruction_with_side(
                    Op::TensorConvolution,
                    TensorConvolution {
                        dest_offset: value_offset(self, destination)?,
                        input_offset: value_offset(self, input)?,
                        kernel_offset: value_offset(self, kernel)?,
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
                let element = tensor_element_type(self.tree, dest_type)
                    .ok_or_else(|| Error::invalid_program("tensor scatter element"))?;
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
                let destination = *destination;
                let left = *left;
                let right = *right;
                let dest_type = self.value_type_for_value(destination)?;
                let left_type = self.value_type_for_value(left)?;
                let right_type = self.value_type_for_value(right)?;
                let dest_layout = self.tensor_layout_from_type(dest_type)?;
                let left_layout = self.tensor_layout_from_type(left_type)?;
                let right_layout = self.tensor_layout_from_type(right_type)?;
                let element = tensor_element_type(self.tree, left_type)
                    .ok_or_else(|| Error::invalid_program("tensor compare element"))?;
                let element_layout = self
                    .value_shape_for_type(element)
                    .ok_or(Error::invalid_instruction())?;
                let kernel = element_binary_kernel(*operator, element_layout)
                    .ok_or(Error::invalid_instruction())?;
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
                let destination = *destination;
                let tensor = *tensor;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let source_element = tensor_element_type(self.tree, source_type)
                    .ok_or_else(|| Error::invalid_program("tensor convert source element"))?;
                let dest_element = tensor_element_type(self.tree, dest_type)
                    .ok_or_else(|| Error::invalid_program("tensor convert destination element"))?;

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
                let destination = *destination;
                let tensor = *tensor;

                Instruction::new(
                    Op::TensorCast,
                    value_offset(self, destination)?,
                    value_offset(self, tensor)?,
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
                let arguments = self.cell_offset_range(pool, self.tree.get_values(*arguments))?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(view)?;
                let source_layout = self.tensor_layout(pool, source_type)?;
                let dest_layout = self.tensor_layout(pool, dest_type)?;
                let (address_space, element) = self.tensor_element_projection(source_type)?;
                let address = TensorAddress::from_address_space(address_space)?;
                let element = pool.projection(element);

                pool.instruction_with_side(
                    Op::TensorView,
                    TensorView {
                        dest_offset: value_offset(self, destination)?,
                        view_offset: value_offset(self, view)?,
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
            _ => return Err(Error::invalid_instruction()),
        })
    }

    /// Return the backing address space and element projection for one tensor view.
    fn tensor_element_projection(
        &self,
        view_type: mir::LocalNodeId<mir::Type>,
    ) -> Result<(AddressSpace, Projection)> {
        let element_type =
            tensor_element_type(self.tree, view_type).ok_or(Error::invalid_instruction())?;
        let address_space =
            tensor_view_address_space(self.tree, view_type).ok_or(Error::invalid_instruction())?;
        let projection = build_tensor_element_projection(self.tree, self.layouts(), element_type)
            .ok_or(Error::invalid_instruction())?;

        Ok((address_space, projection))
    }

    /// Return the compiled tensor layout for one tensor type.
    pub(super) fn tensor_layout(
        &self,
        pool: &mut Pool<'_, '_>,
        tensor_type: mir::LocalNodeId<mir::Type>,
    ) -> Result<TensorLayoutId> {
        let layout = self.tensor_layout_from_type(tensor_type)?;

        Ok(pool.tensor_layout(layout))
    }

    /// Compile one tensor layout from one MIR tensor type.
    pub(super) fn tensor_layout_from_type(
        &self,
        tensor_type: mir::LocalNodeId<mir::Type>,
    ) -> Result<TensorLayout> {
        let (shape, element) = match self.tree.get(tensor_type) {
            mir::Type::Tensor { shape, element, .. } => (shape, element),
            mir::Type::TensorView { shape, element, .. } => (shape, element),
            _ => {
                return Err(Error::type_mismatch(
                    "tensor type",
                    format!("{tensor_type:?}"),
                ));
            }
        };
        let element = *element;

        let shape = static_shape(shape)?;
        let strides = self.static_tensor_strides(tensor_type, &shape)?;
        let element_count = tensor_element_count(&shape);
        let element_span_len = tensor_element_span_len(&shape, &strides)?;
        let is_contiguous = element_count == element_span_len;

        let element_layout = self
            .layouts()
            .get(&element)
            .ok_or(Error::invalid_instruction())?;
        let element = Projection::indexed(
            element,
            element_span_len as u64,
            element_layout.stride(),
            element_layout.byte_len,
            cell_layout_from_type(self.tree, element),
        );
        let element_layout = scalar_layout_from_type(self.tree, element.value_type)
            .ok_or_else(|| Error::type_mismatch("tensor scalar element", format!("{element:?}")))?;
        let byte_len = element_span_len
            .checked_mul(element.byte_stride)
            .ok_or_else(|| Error::internal("tensor payload byte length overflow"))?;

        Ok(TensorLayout {
            byte_len,
            shape: shape.into_boxed_slice(),
            strides: strides.into_boxed_slice(),
            element_count,
            element_span_len,
            is_contiguous,
            element_layout,
            element,
        })
    }

    /// Compile one static tensor stride list.
    fn static_tensor_strides(
        &self,
        tensor_type: mir::LocalNodeId<mir::Type>,
        shape: &[u64],
    ) -> Result<Vec<u64>> {
        match self.tree.get(tensor_type) {
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
            _ => Err(Error::invalid_instruction()),
        }
    }

    /// Return one side-table range of cell frame offsets.
    fn cell_offset_range(
        &self,
        pool: &mut Pool<'_, '_>,
        values: &[mir::Value],
    ) -> Result<U32RangeId> {
        let mut offsets = Vec::with_capacity(values.len());
        for value in values {
            offsets.push(cell_offset(self, *value)?);
        }

        Ok(pool.u32_range(&offsets))
    }

    /// Return one side-table range of value frame offsets.
    fn value_offset_range(
        &self,
        pool: &mut Pool<'_, '_>,
        values: &[mir::Value],
    ) -> Result<U32RangeId> {
        let mut offsets = Vec::with_capacity(values.len());
        for value in values {
            offsets.push(value_offset(self, *value)?);
        }

        Ok(pool.u32_range(&offsets))
    }

    /// Return VM tensor dot dimensions from one MIR immediate.
    fn tensor_dot(
        &self,
        immediate: mir::TensorImmediateId,
    ) -> Result<mir::TensorDotDimensionNumbers> {
        let mir::TensorImmediate::Dot {
            lhs_batch,
            rhs_batch,
            lhs_contracting,
            rhs_contracting,
        } = self.tree.get_tensor_immediate(immediate)
        else {
            return Err(Error::invalid_instruction());
        };

        Ok(mir::TensorDotDimensionNumbers {
            lhs_batch: self.indices(*lhs_batch).to_vec(),
            rhs_batch: self.indices(*rhs_batch).to_vec(),
            lhs_contracting: self.indices(*lhs_contracting).to_vec(),
            rhs_contracting: self.indices(*rhs_contracting).to_vec(),
        })
    }

    /// Return VM tensor convolution descriptors from one MIR immediate.
    fn tensor_convolution(
        &self,
        immediate: mir::TensorImmediateId,
    ) -> Result<(
        mir::TensorConvolutionDimensionNumbers,
        mir::TensorConvolutionWindow,
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
        } = self.tree.get_tensor_immediate(immediate)
        else {
            return Err(Error::invalid_instruction());
        };

        let dimensions = mir::TensorConvolutionDimensionNumbers {
            input_batch: *input_batch,
            input_feature: *input_feature,
            input_spatial: self.indices(*input_spatial).to_vec(),
            kernel_input_feature: *kernel_input_feature,
            kernel_output_feature: *kernel_output_feature,
            kernel_spatial: self.indices(*kernel_spatial).to_vec(),
            output_batch: *output_batch,
            output_feature: *output_feature,
            output_spatial: self.indices(*output_spatial).to_vec(),
        };
        let window = mir::TensorConvolutionWindow {
            strides: self.tree.get_extents(*strides).to_vec(),
            padding_low: self.tree.get_extents(*padding_low).to_vec(),
            padding_high: self.tree.get_extents(*padding_high).to_vec(),
            lhs_dilation: self.tree.get_extents(*lhs_dilation).to_vec(),
            rhs_dilation: self.tree.get_extents(*rhs_dilation).to_vec(),
            window_reversal: self
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
    ) -> Result<(mir::TensorGatherDimensionNumbers, Vec<u32>)> {
        let mir::TensorImmediate::Gather {
            offset_dims,
            collapsed_slice_dims,
            start_index_map,
            index_vector_dim,
            slice_sizes,
        } = self.tree.get_tensor_immediate(immediate)
        else {
            return Err(Error::invalid_instruction());
        };

        Ok((
            mir::TensorGatherDimensionNumbers {
                offset_dims: self.indices(*offset_dims).to_vec(),
                collapsed_slice_dims: self.indices(*collapsed_slice_dims).to_vec(),
                start_index_map: self.indices(*start_index_map).to_vec(),
                index_vector_dim: *index_vector_dim,
            },
            self.indices(*slice_sizes).to_vec(),
        ))
    }

    /// Return VM tensor scatter dimensions from one MIR immediate.
    fn tensor_scatter(
        &self,
        immediate: mir::TensorImmediateId,
    ) -> Result<mir::TensorScatterDimensionNumbers> {
        let mir::TensorImmediate::Scatter {
            update_window_dims,
            inserted_window_dims,
            scatter_dims_to_operand_dims,
            index_vector_dim,
        } = self.tree.get_tensor_immediate(immediate)
        else {
            return Err(Error::invalid_instruction());
        };

        Ok(mir::TensorScatterDimensionNumbers {
            update_window_dims: self.indices(*update_window_dims).to_vec(),
            inserted_window_dims: self.indices(*inserted_window_dims).to_vec(),
            scatter_dims_to_operand_dims: self.indices(*scatter_dims_to_operand_dims).to_vec(),
            index_vector_dim: *index_vector_dim,
        })
    }
}

/// Return the scalar layout for one tensor element type.
pub(super) fn tensor_scalar_layout(
    tree: &mir::Tree,
    element: mir::LocalNodeId<mir::Type>,
) -> Result<ScalarLayout> {
    scalar_layout_from_type(tree, element)
        .ok_or_else(|| Error::type_mismatch("tensor scalar element", format!("{element:?}")))
}
