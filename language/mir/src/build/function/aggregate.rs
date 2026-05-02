use crate::build::FunctionBuilder;
use crate::{
    BinaryOperator, Instruction, LocalNodeId, TensorConvertMode, TensorConvolutionDimensionNumbers,
    TensorConvolutionWindow, TensorDotDimensionNumbers, TensorGatherDimensionNumbers,
    TensorReduceOperator, TensorScatterDimensionNumbers, TensorScatterMode, Type, TypeReference,
    Value, ValueReference, VectorConvertMode, VectorReduceOperator,
};

#[allow(clippy::too_many_arguments)]
impl<'a> FunctionBuilder<'a> {
    /// Extract a field from a struct or tuple.
    pub fn field_get(&mut self, aggregate: Value, index: u32) -> Value {
        let destination = self.allocate_value();
        let aggregate_type = self.value_type_or_panic(aggregate, "field.get aggregate");
        let field_type = self.field_type_for_aggregate(aggregate_type, index);
        self.insert_instruction(Instruction::FieldGet {
            destination: destination.into(),
            aggregate: aggregate.into(),
            index,
        });
        self.define_value(destination, field_type);
        destination
    }

    /// Get the address of a field from a struct or tuple.
    pub fn field_addr(
        &mut self,
        aggregate: Value,
        index: u32,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::FieldAddr {
            destination: destination.into(),
            aggregate: aggregate.into(),
            index,
            result_type: result_type.into(),
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Insert a value into a struct or tuple field.
    pub fn field_set(&mut self, aggregate: Value, index: u32, value: Value) -> Value {
        let destination = self.allocate_value();
        let aggregate_type = self.value_type_or_panic(aggregate, "field.set aggregate");
        self.insert_instruction(Instruction::FieldSet {
            destination: destination.into(),
            aggregate: aggregate.into(),
            index,
            value: value.into(),
        });
        self.define_value(destination, aggregate_type);
        destination
    }

    /// Extract an element from an array.
    pub fn element_get(&mut self, array: Value, index: u32) -> Value {
        let destination = self.allocate_value();
        let array_type = self.value_type_or_panic(array, "element.get array");
        let element_type = self.element_type_for_array(array_type);
        self.insert_instruction(Instruction::ElementGet {
            destination: destination.into(),
            array: array.into(),
            index,
        });
        self.define_value(destination, element_type);
        destination
    }

    /// Get the address of an element from an array.
    pub fn element_addr(
        &mut self,
        array: Value,
        index: Value,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::ElementAddr {
            destination: destination.into(),
            array: array.into(),
            index: index.into(),
            result_type: result_type.into(),
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Insert a value into an array element.
    pub fn element_set(&mut self, array: Value, index: u32, value: Value) -> Value {
        let destination = self.allocate_value();
        let array_type = self.value_type_or_panic(array, "element.set array");
        self.insert_instruction(Instruction::ElementSet {
            destination: destination.into(),
            array: array.into(),
            index,
            value: value.into(),
        });
        self.define_value(destination, array_type);
        destination
    }

    /// Construct a struct from field values.
    ///
    /// Fields must be provided in layout order.
    pub fn struct_(&mut self, ty: LocalNodeId<Type>, field_values: Vec<Value>) -> Value {
        let destination = self.allocate_value();
        let field_values = field_values
            .into_iter()
            .map(ValueReference::from)
            .collect::<Vec<_>>();
        let fields = self.tree.add_arguments(&field_values);
        self.insert_instruction(Instruction::Struct {
            destination: destination.into(),
            ty: TypeReference::Type(ty),
            fields,
        });
        self.define_value(destination, ty);
        destination
    }

    /// Construct a tuple from element values.
    ///
    /// Elements must be provided in order.
    pub fn tuple(&mut self, ty: LocalNodeId<Type>, element_values: Vec<Value>) -> Value {
        let destination = self.allocate_value();
        let element_values = element_values
            .into_iter()
            .map(ValueReference::from)
            .collect::<Vec<_>>();
        let elements = self.tree.add_arguments(&element_values);
        self.insert_instruction(Instruction::Tuple {
            destination: destination.into(),
            ty: TypeReference::Type(ty),
            elements,
        });
        self.define_value(destination, ty);
        destination
    }

    /// Construct an array from element values.
    ///
    /// Elements must be provided in index order.
    pub fn array(&mut self, ty: LocalNodeId<Type>, element_values: Vec<Value>) -> Value {
        let destination = self.allocate_value();
        let element_values = element_values
            .into_iter()
            .map(ValueReference::from)
            .collect::<Vec<_>>();
        let elements = self.tree.add_arguments(&element_values);
        self.insert_instruction(Instruction::Array {
            destination: destination.into(),
            ty: TypeReference::Type(ty),
            elements,
        });
        self.define_value(destination, ty);
        destination
    }

    // instruction builders: vector operations

    /// Broadcast a scalar to all vector lanes.
    pub fn vector_splat(&mut self, vector_type: LocalNodeId<Type>, value: Value) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::VectorSplat {
            destination: destination.into(),
            value: value.into(),
        });
        self.define_value(destination, vector_type);
        destination
    }

    /// Extract a lane from a vector.
    pub fn vector_extract(&mut self, vector: Value, index: Value) -> Value {
        let destination = self.allocate_value();
        let vector_type = self.value_type_or_panic(vector, "vector.extract vector");
        let element_type = self.element_type_for_vector(vector_type);
        self.insert_instruction(Instruction::VectorExtract {
            destination: destination.into(),
            vector: vector.into(),
            index: index.into(),
        });
        self.define_value(destination, element_type);
        destination
    }

    /// Insert a lane into a vector.
    pub fn vector_insert(&mut self, vector: Value, index: Value, value: Value) -> Value {
        let destination = self.allocate_value();
        let vector_type = self.value_type_or_panic(vector, "vector.insert vector");
        self.insert_instruction(Instruction::VectorInsert {
            destination: destination.into(),
            vector: vector.into(),
            index: index.into(),
            value: value.into(),
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
        self.insert_instruction(Instruction::VectorShuffle {
            destination: destination.into(),
            left: left.into(),
            right: right.into(),
            mask,
        });
        self.define_value(destination, vector_type);
        destination
    }

    /// Select vector lanes based on a boolean mask.
    pub fn vector_select(&mut self, mask: Value, then_value: Value, else_value: Value) -> Value {
        let destination = self.allocate_value();
        let vector_type = self.value_type_or_panic(then_value, "vector.select then_value");
        self.insert_instruction(Instruction::VectorSelect {
            destination: destination.into(),
            mask: mask.into(),
            then_value: then_value.into(),
            else_value: else_value.into(),
        });
        self.define_value(destination, vector_type);
        destination
    }

    /// Reduce a vector to a scalar.
    pub fn vector_reduce(&mut self, operator: VectorReduceOperator, vector: Value) -> Value {
        let destination = self.allocate_value();
        let vector_type = self.value_type_or_panic(vector, "vector.reduce vector");
        let element_type = self.element_type_for_vector(vector_type);
        self.insert_instruction(Instruction::VectorReduce {
            destination: destination.into(),
            operator,
            vector: vector.into(),
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
            destination: destination.into(),
            operator,
            left: left.into(),
            right: right.into(),
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
            destination: destination.into(),
            mode,
            vector: vector.into(),
        });
        self.define_value(destination, result_type);
        destination
    }

    // instruction builders: tensor operations

    /// Broadcast a scalar to all tensor elements.
    pub fn tensor_splat(&mut self, tensor_type: LocalNodeId<Type>, value: Value) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::TensorSplat {
            destination: destination.into(),
            value: value.into(),
        });
        self.define_value(destination, tensor_type);
        destination
    }

    /// Select tensor elements based on a boolean mask.
    pub fn tensor_select(&mut self, mask: Value, then_value: Value, else_value: Value) -> Value {
        let destination = self.allocate_value();
        let tensor_type = self.value_type_or_panic(then_value, "tensor.select then_value");
        self.insert_instruction(Instruction::TensorSelect {
            destination: destination.into(),
            mask: mask.into(),
            then_value: then_value.into(),
            else_value: else_value.into(),
        });
        self.define_value(destination, tensor_type);
        destination
    }

    /// Load a tensor element from a tensor reference.
    pub fn tensor_load(&mut self, view: Value, indices: Vec<Value>) -> Value {
        let destination = self.allocate_value();
        let view_type = self.value_type_or_panic(view, "tensor.load view");
        let element_type = self.element_type_for_tensor_view(view_type);
        let indices = indices
            .into_iter()
            .map(ValueReference::from)
            .collect::<Vec<_>>();
        let indices = self.tree.add_arguments(&indices);
        self.insert_instruction(Instruction::TensorLoad {
            destination: destination.into(),
            view: view.into(),
            indices,
        });
        self.define_value(destination, element_type);
        destination
    }

    /// Extract a tensor element from a tensor value.
    pub fn tensor_extract(&mut self, tensor: Value, indices: Vec<Value>) -> Value {
        let destination = self.allocate_value();
        let tensor_type = self.value_type_or_panic(tensor, "tensor.extract tensor");
        let element_type = self.element_type_for_tensor(tensor_type);
        let indices = indices
            .into_iter()
            .map(ValueReference::from)
            .collect::<Vec<_>>();
        let indices = self.tree.add_arguments(&indices);
        self.insert_instruction(Instruction::TensorExtract {
            destination: destination.into(),
            tensor: tensor.into(),
            indices,
        });
        self.define_value(destination, element_type);
        destination
    }

    /// Store a tensor element into a tensor reference.
    pub fn tensor_store(&mut self, view: Value, indices: Vec<Value>, value: Value) {
        let indices = indices
            .into_iter()
            .map(ValueReference::from)
            .collect::<Vec<_>>();
        let indices = self.tree.add_arguments(&indices);
        self.insert_instruction(Instruction::TensorStore {
            view: view.into(),
            indices,
            value: value.into(),
        });
    }

    /// Fill a tensor reference with a scalar value.
    pub fn tensor_fill(&mut self, view: Value, value: Value) {
        self.insert_instruction(Instruction::TensorFill {
            view: view.into(),
            value: value.into(),
        });
    }

    /// Copy elements from a source tensor reference into a destination tensor reference.
    pub fn tensor_copy(&mut self, target: Value, source: Value) {
        self.insert_instruction(Instruction::TensorCopy {
            target: target.into(),
            source: source.into(),
        });
    }

    /// Reshape a tensor value into a new shape.
    pub fn tensor_reshape(
        &mut self,
        result_type: LocalNodeId<Type>,
        tensor: Value,
        shape_values: Vec<Value>,
    ) -> Value {
        let destination = self.allocate_value();
        let shape = shape_values
            .into_iter()
            .map(ValueReference::from)
            .collect::<Vec<_>>();
        let shape = self.tree.add_arguments(&shape);
        self.insert_instruction(Instruction::TensorReshape {
            destination: destination.into(),
            tensor: tensor.into(),
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
        self.insert_instruction(Instruction::TensorBroadcast {
            destination: destination.into(),
            tensor: tensor.into(),
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
        self.insert_instruction(Instruction::TensorTranspose {
            destination: destination.into(),
            tensor: tensor.into(),
            permutation,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Refine a tensor type without changing its contents.
    pub fn tensor_cast(&mut self, result_type: LocalNodeId<Type>, tensor: Value) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::TensorCast {
            destination: destination.into(),
            tensor: tensor.into(),
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
        let offsets_count = self.to_u16_count(offsets.len(), "offsets count");
        let sizes_count = self.to_u16_count(sizes.len(), "sizes count");
        let strides_count = self.to_u16_count(strides.len(), "strides count");
        let mut values = Vec::with_capacity(offsets.len() + sizes.len() + strides.len());
        values.extend_from_slice(&offsets);
        values.extend_from_slice(&sizes);
        values.extend_from_slice(&strides);
        let arguments = values
            .into_iter()
            .map(ValueReference::from)
            .collect::<Vec<_>>();
        let arguments = self.tree.add_arguments(&arguments);
        self.insert_instruction(Instruction::TensorView {
            destination: destination.into(),
            view: view.into(),
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
        let offsets_count = self.to_u16_count(offsets.len(), "offsets count");
        let sizes_count = self.to_u16_count(sizes.len(), "sizes count");
        let strides_count = self.to_u16_count(strides.len(), "strides count");
        let mut values = Vec::with_capacity(offsets.len() + sizes.len() + strides.len());
        values.extend_from_slice(&offsets);
        values.extend_from_slice(&sizes);
        values.extend_from_slice(&strides);
        let arguments = values
            .into_iter()
            .map(ValueReference::from)
            .collect::<Vec<_>>();
        let arguments = self.tree.add_arguments(&arguments);
        self.insert_instruction(Instruction::TensorSlice {
            destination: destination.into(),
            tensor: tensor.into(),
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
        let low_count = self.to_u16_count(low.len(), "low padding count");
        let high_count = self.to_u16_count(high.len(), "high padding count");
        let interior_count = self.to_u16_count(interior.len(), "interior padding count");
        let mut values = Vec::with_capacity(low.len() + high.len() + interior.len());
        values.extend_from_slice(&low);
        values.extend_from_slice(&high);
        values.extend_from_slice(&interior);
        let arguments = values
            .into_iter()
            .map(ValueReference::from)
            .collect::<Vec<_>>();
        let arguments = self.tree.add_arguments(&arguments);
        self.insert_instruction(Instruction::TensorPad {
            destination: destination.into(),
            tensor: tensor.into(),
            arguments,
            low_count,
            high_count,
            interior_count,
            value: value.into(),
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
        let tensors = tensors
            .into_iter()
            .map(ValueReference::from)
            .collect::<Vec<_>>();
        let tensors = self.tree.add_arguments(&tensors);
        self.insert_instruction(Instruction::TensorConcat {
            destination: destination.into(),
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
            destination: destination.into(),
            operator,
            left: left.into(),
            right: right.into(),
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
        self.insert_instruction(Instruction::TensorReduce {
            destination: destination.into(),
            operator,
            tensor: tensor.into(),
            initial: initial.into(),
            axes,
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
        self.insert_instruction(Instruction::TensorDot {
            destination: destination.into(),
            left: left.into(),
            right: right.into(),
            dimensions,
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
        self.insert_instruction(Instruction::TensorConvolution {
            destination: destination.into(),
            input: input.into(),
            kernel: kernel.into(),
            dimensions,
            window,
            feature_group_count,
            batch_group_count,
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
        self.insert_instruction(Instruction::TensorGather {
            destination: destination.into(),
            operand: operand.into(),
            indices: indices.into(),
            dimensions,
            slice_sizes,
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
        self.insert_instruction(Instruction::TensorScatter {
            destination: destination.into(),
            operand: operand.into(),
            indices: indices.into(),
            updates: updates.into(),
            dimensions,
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
            destination: destination.into(),
            mode,
            tensor: tensor.into(),
        });
        self.define_value(destination, result_type);
        destination
    }
}
