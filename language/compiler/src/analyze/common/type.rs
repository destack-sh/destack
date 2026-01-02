use crate::Compiler;
use destack_dir::{LocalTypeId, Type, TypeTable};

impl Compiler {
    /// Build a union type from two type ids.
    pub(crate) fn union_type(
        &self,
        left_ty_id: LocalTypeId,
        right_ty_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // bail on identical types
        if left_ty_id == right_ty_id {
            return left_ty_id;
        }

        // collect union elements
        let mut elements = Vec::new();
        self.collect_union_elements(left_ty_id, &mut elements, types);
        self.collect_union_elements(right_ty_id, &mut elements, types);

        // materialize the union type
        if elements.len() == 1 {
            elements[0]
        } else {
            types.insert_type(Type::Union { elements })
        }
    }

    /// Build a union type from a list of type ids.
    pub(crate) fn union_type_from_list(
        &self,
        type_ids: Vec<LocalTypeId>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // collect union elements
        let mut elements = Vec::new();
        for ty_id in type_ids {
            self.collect_union_elements(ty_id, &mut elements, types);
        }

        // materialize the union type
        if elements.len() == 1 {
            elements[0]
        } else {
            types.insert_type(Type::Union { elements })
        }
    }

    /// Collect union elements for a type id into a list.
    fn collect_union_elements(
        &self,
        ty_id: LocalTypeId,
        elements: &mut Vec<LocalTypeId>,
        types: &TypeTable,
    ) {
        // expand union elements
        match types.get_type(ty_id) {
            Type::Union { elements: union } => {
                for element_id in union {
                    if !elements.contains(element_id) {
                        elements.push(*element_id);
                    }
                }
            }
            _ => {
                // collect non union elements
                if !elements.contains(&ty_id) {
                    elements.push(ty_id);
                }
            }
        }
    }
}
