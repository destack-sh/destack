use crate::build::{BuildError, BuildResult, FunctionBuilder};
use crate::{
    BinaryOperator, ConvertMode, Copy, DispatchSlot, Instruction, LocalNodeId, Place, Substitution,
    Tree, Type, TypeId, Value, VectorReduceOperator,
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
    pub fn field_get(&mut self, aggregate: Value, field: u32, copy: Copy) -> Value {
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
            copy,
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

    /// Read the discriminant of a stored variant.
    pub fn variant_tag_load(&mut self, place: Place, variant_type: LocalNodeId<Type>) -> Value {
        let destination = self.allocate_value();
        let tag_type = self.variant_discriminant_type(variant_type);
        let tag_type = self.expect_build(tag_type);
        self.insert_instruction(Instruction::VariantTagLoad { destination, place });
        self.define_value(destination, tag_type);

        destination
    }

    /// Extract the payload of one statically selected variant case.
    pub fn variant_payload(&mut self, variant: Value, case: u32, copy: Copy) -> Value {
        let destination = self.allocate_value();
        let variant_type = self.expect_value_type(variant, "variant.payload variant");
        let payload_type = self.variant_case_type(variant_type, case);
        let payload_type = self.expect_build(payload_type);
        self.insert_instruction(Instruction::VariantPayload {
            copy,
            destination,
            variant,
            case,
        });
        self.define_value(destination, payload_type);

        destination
    }

    /// Extract one statically selected fixed-array element.
    pub fn element_get(&mut self, aggregate: Value, index: u32, copy: Copy) -> Value {
        let destination = self.allocate_value();
        let aggregate_type = self.expect_value_type(aggregate, "element.get aggregate");
        let element_type = self.fixed_array_element_type(aggregate_type, index);
        let element_type = self.expect_build(element_type);
        self.insert_instruction(Instruction::ElementGet {
            copy,
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

    /// Read the runtime length from a slice descriptor.
    pub fn slice_length(&mut self, slice: Value) -> Value {
        let destination = self.allocate_value();
        let usize_type = self.tree.intern_type(Type::Usize);
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
        let type_id = self.tree.intern_type(Type::TypeId);
        self.insert_instruction(Instruction::DynamicType {
            destination,
            dynamic,
        });
        self.define_value(destination, type_id);
        destination
    }

    /// Read one slot entry through a dynamic value's concrete table.
    pub fn dynamic_read(
        &mut self,
        dynamic: Value,
        slot: DispatchSlot,
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
        mode: ConvertMode,
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
        // substitute the aggregate before selecting its component
        let owner = Substitution::resolve(owner, self.tree);

        project(self.tree, self.tree.type_definition(owner))
    }

    /// Resolve one statically selected fixed-array element type.
    fn fixed_array_element_type(
        &mut self,
        array_type: LocalNodeId<Type>,
        index: u32,
    ) -> BuildResult<LocalNodeId<Type>> {
        self.projected_type(array_type, |tree, array| match array {
            Type::FixedArray {
                element, length, ..
            } if tree
                .static_value(*length)
                .length()
                .is_some_and(|length| u64::from(index) < length) =>
            {
                Ok(*element)
            }
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
}
