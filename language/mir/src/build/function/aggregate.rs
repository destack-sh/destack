use crate::build::{BuildError, BuildResult, FunctionBuilder};
use crate::{
    BinaryOperator, Instruction, LocalNodeId, TensorConvertMode, TensorConvolutionDimensionNumbers,
    TensorConvolutionWindow, TensorDotDimensionNumbers, TensorGatherDimensionNumbers,
    TensorIndexReduceOperator, TensorIndexTieBreak, TensorReduceOperator,
    TensorScatterDimensionNumbers, TensorScatterMode, Tree, Type, TypeId, Value, VectorConvertMode,
    VectorReduceOperator,
};

#[allow(clippy::too_many_arguments)]
impl<'a> FunctionBuilder<'a> {
    /// Construct an aggregate from values in logical slot order.
    pub fn aggregate(&mut self, ty: LocalNodeId<Type>, values: Vec<Value>) -> Value {
        let destination = self.allocate_value();
        let values = self.tree.add_values(&values);
        self.insert_instruction(Instruction::Aggregate {
            destination,
            values,
        });
        self.define_value(destination, ty);

        destination
    }

    /// Extract one structural field from an aggregate.
    pub fn field_get(&mut self, aggregate: Value, field: u32) -> Value {
        let destination = self.allocate_value();
        let aggregate_type = self.expect_value_type(aggregate, "field.get aggregate");
        let field_type = self.projected_type(aggregate_type, |tree, aggregate| {
            aggregate
                .field_type(field, tree)
                .ok_or(BuildError::InvalidFieldIndex {
                    aggregate: aggregate_type,
                    index: field,
                })
        });
        let field_type = self.expect_build(field_type);
        self.insert_instruction(Instruction::FieldGet {
            destination,
            aggregate,
            field,
        });
        self.define_value(destination, field_type);

        destination
    }

    /// Replace one structural field in an aggregate.
    pub fn field_set(&mut self, aggregate: Value, field: u32, value: Value) -> Value {
        let destination = self.allocate_value();
        let aggregate_type = self.expect_value_type(aggregate, "field.set aggregate");
        self.insert_instruction(Instruction::FieldSet {
            destination,
            aggregate,
            field,
            value,
        });
        self.define_value(destination, aggregate_type);

        destination
    }

    /// Construct a variant value from one case payload.
    pub fn variant_new(
        &mut self,
        ty: LocalNodeId<Type>,
        case: u32,
        payload: Option<Value>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::VariantNew {
            destination,
            case,
            payload,
            result_type: ty,
        });
        self.define_value(destination, ty);

        destination
    }

    /// Read the discriminant of a variant value.
    pub fn variant_tag(&mut self, variant: Value) -> Value {
        let destination = self.allocate_value();
        let variant_type = self.expect_value_type(variant, "variant.tag variant");
        let tag_type = self.variant_discriminant_type(variant_type);
        let tag_type = self.expect_build(tag_type);
        self.insert_instruction(Instruction::VariantTag {
            destination,
            variant,
        });
        self.define_value(destination, tag_type);

        destination
    }

    /// Extract the payload of one statically selected variant case.
    pub fn variant_payload(&mut self, variant: Value, case: u32) -> Value {
        let destination = self.allocate_value();
        let variant_type = self.expect_value_type(variant, "variant.payload variant");
        let payload_type = self.variant_case_type(variant_type, case);
        let payload_type = self.expect_build(payload_type);
        self.insert_instruction(Instruction::VariantPayload {
            destination,
            variant,
            case,
        });
        self.define_value(destination, payload_type);

        destination
    }

    /// Get the address of one structural field in an aggregate.
    pub fn field_addr(
        &mut self,
        aggregate: Value,
        field: u32,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::FieldAddr {
            destination,
            aggregate,
            field,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Extract one statically selected fixed-array element.
    pub fn element_get(&mut self, aggregate: Value, index: u32) -> Value {
        let destination = self.allocate_value();
        let aggregate_type = self.expect_value_type(aggregate, "element.get aggregate");
        let element_type = self.fixed_array_element_type(aggregate_type, index);
        let element_type = self.expect_build(element_type);
        self.insert_instruction(Instruction::ElementGet {
            destination,
            aggregate,
            index,
        });
        self.define_value(destination, element_type);

        destination
    }

    /// Insert one value into a statically selected fixed-array element.
    pub fn element_set(&mut self, aggregate: Value, index: u32, value: Value) -> Value {
        let destination = self.allocate_value();
        let aggregate_type = self.expect_value_type(aggregate, "element.set aggregate");
        let element_type = self.fixed_array_element_type(aggregate_type, index);
        self.expect_build(element_type);
        self.insert_instruction(Instruction::ElementSet {
            destination,
            aggregate,
            index,
            value,
        });
        self.define_value(destination, aggregate_type);

        destination
    }

    /// Get the address of an element from an array.
    pub fn element_addr(
        &mut self,
        base: Value,
        index: Value,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::ElementAddr {
            destination,
            base,
            index,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Form a non-owning slice view over a contiguous source region.
    pub fn slice_view(
        &mut self,
        source: Value,
        start: Value,
        length: Value,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::SliceView {
            destination,
            source,
            start,
            length,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Read the runtime length from a slice descriptor.
    pub fn slice_length(&mut self, slice: Value) -> Value {
        let destination = self.allocate_value();
        let usize_type = self.ensure_usize_type();
        self.insert_instruction(Instruction::SliceLength { destination, slice });
        self.define_value(destination, usize_type);
        destination
    }

    /// Bind a typed reference payload to its concrete runtime type.
    pub fn dynamic_bind(
        &mut self,
        dynamic_type: LocalNodeId<Type>,
        payload: Value,
        concrete: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::DynamicBind {
            destination,
            payload,
            concrete: TypeId::from(concrete),
        });
        self.define_value(destination, dynamic_type);

        destination
    }

    /// Read the erased payload from a dynamic value.
    pub fn dynamic_payload(&mut self, dynamic: Value, result_type: LocalNodeId<Type>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::DynamicPayload {
            destination,
            dynamic,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Read the concrete type id from a dynamic value.
    pub fn dynamic_type(&mut self, dynamic: Value) -> Value {
        let destination = self.allocate_value();
        let type_id = self.tree.type_id_type();
        self.insert_instruction(Instruction::DynamicType {
            destination,
            dynamic,
        });
        self.define_value(destination, type_id);
        destination
    }

    /// Read one dispatch slot through a dynamic value's concrete table.
    pub fn dynamic_read(
        &mut self,
        dynamic: Value,
        slot: u32,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::DynamicRead {
            destination,
            dynamic,
            slot,
            result_type: TypeId::from(result_type),
        });
        self.define_value(destination, result_type);

        destination
    }

    /// Find one named entry through a dynamic value's concrete table.
    pub fn dynamic_find(
        &mut self,
        dynamic: Value,
        key: Value,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::DynamicFind {
            destination,
            dynamic,
            key,
            result_type: TypeId::from(result_type),
        });
        self.define_value(destination, result_type);

        destination
    }

    // instruction builders: vector operations

    /// Broadcast a scalar to all vector lanes.
    pub fn vector_splat(&mut self, vector_type: LocalNodeId<Type>, value: Value) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::VectorSplat { destination, value });
        self.define_value(destination, vector_type);
        destination
    }

    /// Extract a lane from a vector.
    pub fn vector_extract(&mut self, vector: Value, index: Value) -> Value {
        let destination = self.allocate_value();
        let vector_type = self.expect_value_type(vector, "vector.extract vector");
        let element_type = self.vector_element_type(vector_type);
        let element_type = self.expect_build(element_type);
        self.insert_instruction(Instruction::VectorExtract {
            destination,
            vector,
            index,
        });
        self.define_value(destination, element_type);
        destination
    }

    /// Insert a lane into a vector.
    pub fn vector_insert(&mut self, vector: Value, index: Value, value: Value) -> Value {
        let destination = self.allocate_value();
        let vector_type = self.expect_value_type(vector, "vector.insert vector");
        self.insert_instruction(Instruction::VectorInsert {
            destination,
            vector,
            index,
            value,
        });
        self.define_value(destination, vector_type);
        destination
    }

    /// Shuffle vector lanes with a constant mask.
    pub fn vector_shuffle(
        &mut self,
        vector_type: LocalNodeId<Type>,
        left: Value,
        right: Value,
        mask: Vec<u32>,
    ) -> Value {
        let destination = self.allocate_value();
        let mask = self.tree.add_indices(&mask);
        self.insert_instruction(Instruction::VectorShuffle {
            destination,
            left,
            right,
            mask,
        });
        self.define_value(destination, vector_type);
        destination
    }

    /// Select vector lanes based on a boolean mask.
    pub fn vector_select(&mut self, mask: Value, then_value: Value, else_value: Value) -> Value {
        let destination = self.allocate_value();
        let vector_type = self.expect_value_type(then_value, "vector.select then_value");
        self.insert_instruction(Instruction::VectorSelect {
            destination,
            mask,
            then_value,
            else_value,
        });
        self.define_value(destination, vector_type);
        destination
    }

    /// Reduce a vector to a scalar.
    pub fn vector_reduce(&mut self, operator: VectorReduceOperator, vector: Value) -> Value {
        let destination = self.allocate_value();
        let vector_type = self.expect_value_type(vector, "vector.reduce vector");
        let element_type = self.vector_element_type(vector_type);
        let element_type = self.expect_build(element_type);
        self.insert_instruction(Instruction::VectorReduce {
            destination,
            operator,
            vector,
        });
        self.define_value(destination, element_type);
        destination
    }

    /// Compare two vectors elementwise.
    pub fn vector_compare(
        &mut self,
        result_type: LocalNodeId<Type>,
        operator: BinaryOperator,
        left: Value,
        right: Value,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::VectorCompare {
            destination,
            operator,
            left,
            right,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Convert vector element types with an explicit mode.
    pub fn vector_convert(
        &mut self,
        result_type: LocalNodeId<Type>,
        mode: VectorConvertMode,
        vector: Value,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::VectorConvert {
            destination,
            mode,
            vector,
        });
        self.define_value(destination, result_type);
        destination
    }

    // instruction builders: tensor operations

    /// Broadcast a scalar to all tensor elements.
    pub fn tensor_splat(&mut self, tensor_type: LocalNodeId<Type>, value: Value) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::TensorSplat { destination, value });
        self.define_value(destination, tensor_type);
        destination
    }

    /// Select tensor elements based on a boolean mask.
    pub fn tensor_select(&mut self, mask: Value, then_value: Value, else_value: Value) -> Value {
        let destination = self.allocate_value();
        let tensor_type = self.expect_value_type(then_value, "tensor.select then_value");
        self.insert_instruction(Instruction::TensorSelect {
            destination,
            mask,
            then_value,
            else_value,
        });
        self.define_value(destination, tensor_type);
        destination
    }

    /// Load a tensor element from a tensor reference.
    pub fn tensor_load(&mut self, view: Value, indices: Vec<Value>) -> Value {
        let destination = self.allocate_value();
        let view_type = self.expect_value_type(view, "tensor.load view");
        let element_type = self.tensor_view_element_type(view_type);
        let element_type = self.expect_build(element_type);
        let indices = indices.into_iter().collect::<Vec<_>>();
        let indices = self.tree.add_values(&indices);
        self.insert_instruction(Instruction::TensorLoad {
            destination,
            view,
            indices,
        });
        self.define_value(destination, element_type);
        destination
    }

    /// Extract a tensor element from a tensor value.
    pub fn tensor_extract(&mut self, tensor: Value, indices: Vec<Value>) -> Value {
        let destination = self.allocate_value();
        let tensor_type = self.expect_value_type(tensor, "tensor.extract tensor");
        let element_type = self.tensor_element_type(tensor_type);
        let element_type = self.expect_build(element_type);
        let indices = indices.into_iter().collect::<Vec<_>>();
        let indices = self.tree.add_values(&indices);
        self.insert_instruction(Instruction::TensorExtract {
            destination,
            tensor,
            indices,
        });
        self.define_value(destination, element_type);
        destination
    }

    /// Store a tensor element into a tensor reference.
    pub fn tensor_store(&mut self, view: Value, indices: Vec<Value>, value: Value) {
        let indices = indices.into_iter().collect::<Vec<_>>();
        let indices = self.tree.add_values(&indices);
        self.insert_instruction(Instruction::TensorStore {
            view,
            indices,
            value,
        });
    }

    /// Fill a tensor reference with a scalar value.
    pub fn tensor_fill(&mut self, view: Value, value: Value) {
        self.insert_instruction(Instruction::TensorFill { view, value });
    }

    /// Copy elements from a source tensor reference into a destination tensor reference.
    pub fn tensor_copy(&mut self, target: Value, source: Value) {
        self.insert_instruction(Instruction::TensorCopy { target, source });
    }

    /// Reshape a tensor value into a new shape.
    pub fn tensor_reshape(
        &mut self,
        result_type: LocalNodeId<Type>,
        tensor: Value,
        shape_values: Vec<Value>,
    ) -> Value {
        let destination = self.allocate_value();
        let shape = shape_values.into_iter().collect::<Vec<_>>();
        let shape = self.tree.add_values(&shape);
        self.insert_instruction(Instruction::TensorReshape {
            destination,
            tensor,
            shape,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Broadcast a tensor into a larger shape.
    pub fn tensor_broadcast(
        &mut self,
        result_type: LocalNodeId<Type>,
        tensor: Value,
        dimensions: Vec<u32>,
    ) -> Value {
        let destination = self.allocate_value();
        let dimensions = self.tree.add_indices(&dimensions);
        self.insert_instruction(Instruction::TensorBroadcast {
            destination,
            tensor,
            dimensions,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Permute tensor dimensions.
    pub fn tensor_transpose(
        &mut self,
        result_type: LocalNodeId<Type>,
        tensor: Value,
        permutation: Vec<u32>,
    ) -> Value {
        let destination = self.allocate_value();
        let permutation = self.tree.add_indices(&permutation);
        self.insert_instruction(Instruction::TensorTranspose {
            destination,
            tensor,
            permutation,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Refine a tensor type without changing its contents.
    pub fn tensor_cast(&mut self, result_type: LocalNodeId<Type>, tensor: Value) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::TensorCast {
            destination,
            tensor,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Create a view into a tensor reference.
    pub fn tensor_view(
        &mut self,
        result_type: LocalNodeId<Type>,
        view: Value,
        offsets: Vec<Value>,
        sizes: Vec<Value>,
        strides: Vec<Value>,
    ) -> Value {
        let destination = self.allocate_value();
        let offsets_count = self.expect_build(instruction_table_count(offsets.len(), "offsets"));
        let sizes_count = self.expect_build(instruction_table_count(sizes.len(), "sizes"));
        let strides_count = self.expect_build(instruction_table_count(strides.len(), "strides"));
        let mut values = Vec::with_capacity(offsets.len() + sizes.len() + strides.len());
        values.extend_from_slice(&offsets);
        values.extend_from_slice(&sizes);
        values.extend_from_slice(&strides);
        let arguments = values.into_iter().collect::<Vec<_>>();
        let arguments = self.tree.add_values(&arguments);
        self.insert_instruction(Instruction::TensorView {
            destination,
            view,
            arguments,
            offsets_count,
            sizes_count,
            strides_count,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Slice a tensor by offsets, sizes, and strides.
    pub fn tensor_slice(
        &mut self,
        result_type: LocalNodeId<Type>,
        tensor: Value,
        offsets: Vec<Value>,
        sizes: Vec<Value>,
        strides: Vec<Value>,
    ) -> Value {
        let destination = self.allocate_value();
        let offsets_count = self.expect_build(instruction_table_count(offsets.len(), "offsets"));
        let sizes_count = self.expect_build(instruction_table_count(sizes.len(), "sizes"));
        let strides_count = self.expect_build(instruction_table_count(strides.len(), "strides"));
        let mut values = Vec::with_capacity(offsets.len() + sizes.len() + strides.len());
        values.extend_from_slice(&offsets);
        values.extend_from_slice(&sizes);
        values.extend_from_slice(&strides);
        let arguments = values.into_iter().collect::<Vec<_>>();
        let arguments = self.tree.add_values(&arguments);
        self.insert_instruction(Instruction::TensorSlice {
            destination,
            tensor,
            arguments,
            offsets_count,
            sizes_count,
            strides_count,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Pad a tensor with low, high, and interior padding.
    pub fn tensor_pad(
        &mut self,
        result_type: LocalNodeId<Type>,
        tensor: Value,
        value: Value,
        low: Vec<Value>,
        high: Vec<Value>,
        interior: Vec<Value>,
    ) -> Value {
        let destination = self.allocate_value();
        let low_count = self.expect_build(instruction_table_count(low.len(), "low padding"));
        let high_count = self.expect_build(instruction_table_count(high.len(), "high padding"));
        let interior_count =
            self.expect_build(instruction_table_count(interior.len(), "interior padding"));
        let mut values = Vec::with_capacity(low.len() + high.len() + interior.len());
        values.extend_from_slice(&low);
        values.extend_from_slice(&high);
        values.extend_from_slice(&interior);
        let arguments = values.into_iter().collect::<Vec<_>>();
        let arguments = self.tree.add_values(&arguments);
        self.insert_instruction(Instruction::TensorPad {
            destination,
            tensor,
            arguments,
            low_count,
            high_count,
            interior_count,
            value,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Concatenate tensors along a dimension.
    pub fn tensor_concat(
        &mut self,
        result_type: LocalNodeId<Type>,
        tensors: Vec<Value>,
        axis: u32,
    ) -> Value {
        let destination = self.allocate_value();
        let tensors = tensors.into_iter().collect::<Vec<_>>();
        let tensors = self.tree.add_values(&tensors);
        self.insert_instruction(Instruction::TensorConcat {
            destination,
            tensors,
            axis,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Compare two tensors elementwise.
    pub fn tensor_compare(
        &mut self,
        result_type: LocalNodeId<Type>,
        operator: BinaryOperator,
        left: Value,
        right: Value,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::TensorCompare {
            destination,
            operator,
            left,
            right,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Reduce a tensor along axes with a fixed operator.
    pub fn tensor_reduce(
        &mut self,
        result_type: LocalNodeId<Type>,
        operator: TensorReduceOperator,
        tensor: Value,
        initial: Value,
        axes: Vec<u32>,
    ) -> Value {
        let destination = self.allocate_value();
        let axes = self.tree.add_indices(&axes);
        self.insert_instruction(Instruction::TensorReduce {
            destination,
            operator,
            tensor,
            initial,
            axes,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Reduce a tensor along one axis and return selected source indices.
    pub fn tensor_index_reduce(
        &mut self,
        result_type: LocalNodeId<Type>,
        operator: TensorIndexReduceOperator,
        tensor: Value,
        axis: u32,
        tie_break: TensorIndexTieBreak,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::TensorIndexReduce {
            destination,
            operator,
            tensor,
            axis,
            tie_break,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Compute a tensor dot product.
    pub fn tensor_dot(
        &mut self,
        result_type: LocalNodeId<Type>,
        left: Value,
        right: Value,
        dimensions: TensorDotDimensionNumbers,
    ) -> Value {
        let destination = self.allocate_value();
        let immediate = self.tree.add_tensor_dot_immediate(dimensions);
        self.insert_instruction(Instruction::TensorDot {
            destination,
            left,
            right,
            immediate,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Perform a tensor convolution.
    pub fn tensor_convolution(
        &mut self,
        result_type: LocalNodeId<Type>,
        input: Value,
        kernel: Value,
        dimensions: TensorConvolutionDimensionNumbers,
        window: TensorConvolutionWindow,
        feature_group_count: u32,
        batch_group_count: u32,
    ) -> Value {
        let destination = self.allocate_value();
        let immediate = self.tree.add_tensor_convolution_immediate(
            dimensions,
            window,
            feature_group_count,
            batch_group_count,
        );
        self.insert_instruction(Instruction::TensorConvolution {
            destination,
            input,
            kernel,
            immediate,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Gather slices from a tensor based on indices.
    pub fn tensor_gather(
        &mut self,
        result_type: LocalNodeId<Type>,
        operand: Value,
        indices: Value,
        dimensions: TensorGatherDimensionNumbers,
        slice_sizes: Vec<u32>,
    ) -> Value {
        let destination = self.allocate_value();
        let immediate = self
            .tree
            .add_tensor_gather_immediate(dimensions, &slice_sizes);
        self.insert_instruction(Instruction::TensorGather {
            destination,
            operand,
            indices,
            immediate,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Scatter updates into a tensor based on indices.
    pub fn tensor_scatter(
        &mut self,
        result_type: LocalNodeId<Type>,
        operand: Value,
        indices: Value,
        updates: Value,
        dimensions: TensorScatterDimensionNumbers,
        mode: TensorScatterMode,
    ) -> Value {
        let destination = self.allocate_value();
        let immediate = self.tree.add_tensor_scatter_immediate(dimensions);
        self.insert_instruction(Instruction::TensorScatter {
            destination,
            operand,
            indices,
            updates,
            immediate,
            mode,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Convert a tensor element type.
    pub fn tensor_convert(
        &mut self,
        result_type: LocalNodeId<Type>,
        mode: TensorConvertMode,
        tensor: Value,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::TensorConvert {
            destination,
            mode,
            tensor,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Resolve the discriminant type of a variant type.
    fn variant_discriminant_type(
        &mut self,
        variant_type: LocalNodeId<Type>,
    ) -> BuildResult<LocalNodeId<Type>> {
        self.projected_type(variant_type, |_, variant| match variant {
            Type::Variant { discriminant, .. } => Ok(*discriminant),
            _ => Err(BuildError::InvalidVariantOwner { ty: variant_type }),
        })
    }

    /// Resolve one statically selected variant case payload type.
    fn variant_case_type(
        &mut self,
        variant_type: LocalNodeId<Type>,
        case: u32,
    ) -> BuildResult<LocalNodeId<Type>> {
        self.projected_type(variant_type, |_, variant| match variant {
            Type::Variant { cases, .. } => match cases.get(case as usize) {
                Some(entry) => Ok(entry.ty),
                None => Err(BuildError::InvalidCaseIndex {
                    variant: variant_type,
                    case,
                }),
            },
            _ => Err(BuildError::InvalidVariantOwner { ty: variant_type }),
        })
    }

    /// Select and instantiate one type projected from an applied owner.
    fn projected_type(
        &mut self,
        owner: LocalNodeId<Type>,
        project: impl FnOnce(&Tree, &Type) -> BuildResult<LocalNodeId<Type>>,
    ) -> BuildResult<LocalNodeId<Type>> {
        let (base, arguments) = self.tree.split_lifetime_application(owner);
        let arguments = arguments.to_vec();
        let projected = project(self.tree, self.tree.get(base))?;

        Ok(self.tree.instantiate_type_lifetimes(projected, &arguments))
    }

    /// Resolve one statically selected fixed-array element type.
    fn fixed_array_element_type(
        &mut self,
        array_type: LocalNodeId<Type>,
        index: u32,
    ) -> BuildResult<LocalNodeId<Type>> {
        self.projected_type(array_type, |_, array| match array {
            Type::FixedArray {
                element, length, ..
            } if u64::from(index) < *length => Ok(*element),
            Type::FixedArray { .. } => Err(BuildError::InvalidElementIndex {
                array: array_type,
                index,
            }),
            _ => Err(BuildError::InvalidElementOwner { ty: array_type }),
        })
    }

    /// Resolve the element type of one vector type.
    fn vector_element_type(
        &mut self,
        vector_type: LocalNodeId<Type>,
    ) -> BuildResult<LocalNodeId<Type>> {
        self.projected_type(vector_type, |_, vector| match vector {
            Type::Vector { element, .. } => Ok(*element),
            _ => Err(BuildError::InvalidVectorOwner { ty: vector_type }),
        })
    }

    /// Resolve the element type of one tensor type.
    fn tensor_element_type(
        &self,
        tensor_type: LocalNodeId<Type>,
    ) -> BuildResult<LocalNodeId<Type>> {
        match self.tree.get(tensor_type) {
            Type::Tensor { element, .. } => Ok(*element),
            _ => Err(BuildError::InvalidTensorOwner { ty: tensor_type }),
        }
    }

    /// Resolve the element type of one tensor view type.
    fn tensor_view_element_type(
        &self,
        view_type: LocalNodeId<Type>,
    ) -> BuildResult<LocalNodeId<Type>> {
        match self.tree.get(view_type) {
            Type::TensorView { element, .. } => Ok(*element),
            _ => Err(BuildError::InvalidTensorViewOwner { ty: view_type }),
        }
    }
}

/// Convert one list length into a MIR instruction-table count.
fn instruction_table_count(count: usize, context: &str) -> BuildResult<u16> {
    u16::try_from(count).map_err(|_| BuildError::CountTooLarge {
        count,
        context: context.to_string(),
    })
}
