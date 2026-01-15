use destack_dir::AnchoredGlobalNodeId;
use destack_source::ModuleId;
use {destack_dir as dir, destack_mir as mir};

use super::{
    FieldInput, LayoutPolicy, TypeLowerer, compute_struct_layout, static_key_to_field_name,
};
use crate::{LowerError, LowerResult};

/// Compute aggregate copyability from element types.
fn compute_aggregate_copyability(
    element_types: &[mir::LocalNodeId<mir::Type>],
    tree: &mir::NodeTree,
) -> mir::Copyability {
    let mut copyability = mir::Copyability::Trivial;
    for &element_type_id in element_types {
        let element_type = tree.get(element_type_id);
        copyability = copyability.combine(element_type.copyability());
    }
    copyability
}

impl TypeLowerer {
    /// Lower a DIR object type to a MIR struct type.
    ///
    /// This computes the layout for the struct fields and creates the MIR type.
    pub(crate) fn lower_object_type(
        &mut self,
        types: &dir::TypeTable,
        fields: &[dir::TypeField],
        module_id: ModuleId,
        node: AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        let pointer_bytes = self.pointer_bytes();
        let mut field_inputs = Vec::with_capacity(fields.len());

        for (source_index, field) in fields.iter().enumerate() {
            // convert the field key to a name (handles symbols with synthetic names)
            let name = static_key_to_field_name(&field.key, builder);

            // lower the field's type and compute size/alignment
            let field_mir_type = self.lower_type(types, field.ty, module_id, node, builder)?;
            let field_type = builder.tree().get(field_mir_type);
            let (size, alignment) =
                super::size_and_align_of_type(field_type, builder.tree(), pointer_bytes);

            field_inputs.push(FieldInput {
                name,
                ty: field_mir_type,
                size,
                alignment,
                source_index: source_index as u32,
            });
        }

        // compute the layout and create the MIR struct type
        let layout = compute_struct_layout(field_inputs, LayoutPolicy::default());
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
        node: AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // reject optional and rest elements for now
        for element in elements {
            if element.is_optional {
                return Err(LowerError::UnsupportedType {
                    node,
                    ty: element.ty.into_global(module_id),
                    message: "optional tuple elements are not yet supported".to_string(),
                });
            }
            if element.is_rest {
                return Err(LowerError::UnsupportedType {
                    node,
                    ty: element.ty.into_global(module_id),
                    message: "rest tuple elements are not yet supported".to_string(),
                });
            }
        }

        // lower each element type
        let mut mir_elements = Vec::with_capacity(elements.len());
        for element in elements {
            let mir_type = self.lower_type(types, element.ty, module_id, node, builder)?;
            mir_elements.push(mir_type);
        }

        // compute copyability from element types
        let copyability = compute_aggregate_copyability(&mir_elements, builder.tree());

        Ok(builder.type_tuple(mir_elements, copyability))
    }

    /// Lower a DIR sized array type to a MIR array type.
    pub(crate) fn lower_array_sized_type(
        &mut self,
        types: &dir::TypeTable,
        element: dir::LocalTypeId,
        count: dir::LocalNodeId<dir::Expression>,
        module_id: ModuleId,
        node: AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // lower the element type
        let mir_element = self.lower_type(types, element, module_id, node, builder)?;

        // get the inferred type of the count expression
        let count_node = count.into_global_any(module_id);
        let length = types
            .get_declared_or_inferred_type_id(count_node)
            .and_then(|type_id| {
                let ty = types.get_type(type_id);
                match ty {
                    dir::Type::TypeLiteral {
                        value: dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::Integer(n)),
                    } => Some(*n as u64),
                    _ => None,
                }
            })
            .ok_or_else(|| LowerError::UnsupportedType {
                node,
                ty: element.into_global(module_id),
                message: "array size must be a constant integer".to_string(),
            })?;

        // array inherits copyability from element type
        let element_type = builder.tree().get(mir_element);
        let copyability = element_type.copyability();

        Ok(builder.type_array(mir_element, length, copyability))
    }
}
