use destack_source::ModuleId;
use {destack_dir as dir, destack_mir as mir};

use super::{FieldInput, FieldLayoutKind, LayoutPolicy, TypeLowerer};
use crate::lower::static_key_to_field_name;
use crate::{LowerError, LowerResult};

/// Maximum unwrap depth when resolving array count literals.
const MAX_ARRAY_COUNT_UNWRAP_STEPS: usize = 16;

/// Compute aggregate copy from element types.
fn compute_aggregate_copyability(
    element_types: &[mir::LocalNodeId<mir::Type>],
    tree: &mir::Tree,
) -> mir::Copy {
    let mut copy = mir::Copy::Yes;
    for &element_type_id in element_types {
        let element_type = tree.get(element_type_id);
        copy = copy.combine(element_type.copy());
    }
    copy
}

impl TypeLowerer<'_> {
    /// Resolve an integer literal length from one DIR type id.
    fn array_sized_length_from_type(
        &self,
        types: &dir::TypeTable,
        mut type_id: dir::LocalTypeId,
    ) -> Option<u64> {
        for _ in 0..MAX_ARRAY_COUNT_UNWRAP_STEPS {
            match types.get_type(type_id) {
                dir::Type::Literal(dir::LiteralType::ScalarLiteral(
                    dir::ScalarLiteral::Integer(value),
                )) => return u64::try_from(*value).ok(),
                dir::Type::Value(value) => type_id = value.value,
                dir::Type::Reference(reference) => {
                    type_id = types.get_value_type_id(reference.symbol)?;
                }
                _ => return None,
            }
        }

        None
    }

    /// Lower a DIR object type to a MIR struct type.
    ///
    /// This computes the layout for the struct fields and creates the MIR type.
    pub(crate) fn lower_object_type(
        &mut self,
        types: &dir::TypeTable,
        fields: &[dir::TypeField],
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        let mut field_inputs = Vec::with_capacity(fields.len());

        for (source_index, field) in fields.iter().enumerate() {
            // convert the field key to a name (handles symbols with synthetic names)
            let name = static_key_to_field_name(&field.key, builder);

            // lower the field's type and compute size/alignment
            let field_mir_type = self.lower_type(types, field.ty, module_id, node, builder)?;
            // skip void fields that lower to no storage
            if field_mir_type == self.ty_void {
                continue;
            }
            let field_type = builder.tree().get(field_mir_type);
            let (size, alignment) = self
                .size_and_align_of_type(field_type, builder.tree())
                .ok_or_else(|| LowerError::UnsupportedType {
                    anchor: self.diagnostic_anchor(node),
                    ty: field.ty.into_global(module_id),
                    message: "aggregate layout requires concrete nested types".to_string(),
                })?;

            field_inputs.push(FieldInput {
                name,
                ty: field_mir_type,
                size,
                alignment,
                source_index: Some(source_index as u32),
                kind: FieldLayoutKind::Source,
            });
        }

        // compute the layout and create the MIR struct type
        let layout = Self::compute_struct_layout(field_inputs, LayoutPolicy::default());
        let mir_type = self.create_struct_type(&layout, builder);
        self.layout_cache.insert(mir_type, layout);

        Ok(mir_type)
    }

    /// Lower a DIR tuple type to a MIR tuple type.
    pub(crate) fn lower_tuple_type(
        &mut self,
        types: &dir::TypeTable,
        elements: &[dir::TypeElement],
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // lower each element type
        let mut mir_elements = Vec::with_capacity(elements.len());
        for element in elements {
            let mir_type = self.lower_type(types, element.ty, module_id, node, builder)?;
            // reject void elements in tuples
            if mir_type == self.ty_void {
                return Err(LowerError::UnsupportedType {
                    anchor: self.diagnostic_anchor(node),
                    ty: element.ty.into_global(module_id),
                    message: "void is not allowed in tuples".to_string(),
                }
                .into());
            }
            mir_elements.push(mir_type);
        }

        // compute copy from element types
        let copy = compute_aggregate_copyability(&mir_elements, builder.tree());

        Ok(builder.type_tuple(mir_elements, copy))
    }

    /// Lower a DIR sized array type to a MIR array type.
    pub(crate) fn lower_array_sized_type(
        &mut self,
        types: &dir::TypeTable,
        element: dir::LocalTypeId,
        count: dir::LocalTypeId,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // lower the element type
        let mir_element = self.lower_type(types, element, module_id, node, builder)?;
        // reject void elements in arrays
        if mir_element == self.ty_void {
            return Err(LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: element.into_global(module_id),
                message: "void is not allowed in arrays".to_string(),
            }
            .into());
        }

        // get the integer literal length from the count type
        let count_type_id = types.unwrap_value_type_id(count);
        let length = self
            .array_sized_length_from_type(types, count_type_id)
            .ok_or_else(|| LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: element.into_global(module_id),
                message: "array size must be a constant integer".to_string(),
            })?;

        // array inherits copy from element type
        let element_type = builder.tree().get(mir_element);
        let copy = element_type.copy();

        Ok(builder.type_array(mir_element, length, copy))
    }
}
