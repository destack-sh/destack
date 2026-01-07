use crate::Compiler;
use destack_dir::{FunctionSignature, LocalTypeId, NodeTree, Type, TypeTable};
use destack_workspace::Module;

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
            types.insert_type_from_type(Type::Union { elements }, left_ty_id)
        }
    }

    /// Build a union type from a list of type ids.
    pub(crate) fn union_type_from_list(
        &self,
        type_ids: Vec<LocalTypeId>,
        source_type_id: LocalTypeId,
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
            types.insert_type_from_type(Type::Union { elements }, source_type_id)
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

    /// Build placeholder types for static parameters in a signature.
    pub(crate) fn static_parameter_placeholders_for_signature(
        &self,
        module: &Module,
        signature: &FunctionSignature,
        tree: &NodeTree,
        types: &mut TypeTable,
    ) -> Vec<LocalTypeId> {
        // extract static parameters from the signature
        let mut static_parameters = Vec::new();
        let Some(generics) = &signature.generics else {
            return static_parameters;
        };
        let Some(parameters) = &generics.static_parameters else {
            return static_parameters;
        };

        // register each static parameter as a reference placeholder
        for parameter_id in parameters {
            let parameter = tree.get(*parameter_id);
            let symbol = parameter.symbol().into_global(module.id);
            let ty = Type::Reference {
                symbol,
                static_arguments: None,
            };
            let ty_id = types.insert_type_from(ty, *parameter_id);
            static_parameters.push(ty_id);
        }

        static_parameters
    }
}
