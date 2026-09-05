use crate::{Access, GenericArgument, Space, Static, Tree, Type, TypeDeclaration, TypeId};

impl TypeId {
    /// Return whether this type mentions a template parameter, through its declaration or its
    /// definition, identified children counting only through their declaration.
    pub fn mentions_parameter(self, tree: &Tree) -> bool {
        tree.type_declaration(self)
            .is_some_and(|declaration| tree.get(declaration).is_template())
            || definition_mentions_parameter(tree, self, true)
    }
}

impl GenericArgument {
    /// Return whether this argument mentions a template parameter.
    pub fn mentions_parameter(&self, tree: &Tree) -> bool {
        match self {
            GenericArgument::Type(ty) => definition_mentions_parameter(tree, *ty, false),
            GenericArgument::Space(space) => matches!(space, Space::Parameter(_)),
            GenericArgument::Access(access) => matches!(access, Access::Parameter(_)),
            GenericArgument::Value(value) => {
                matches!(tree.static_value(*value), Static::Parameter(_))
            }
        }
    }
}

impl TypeDeclaration {
    /// Return whether this declaration takes generic parameters.
    pub fn is_template(&self) -> bool {
        !self.generics.is_empty()
    }
}

/// Return whether one type mentions a parameter, walking identified children only at the root.
fn definition_mentions_parameter(tree: &Tree, ty: TypeId, root: bool) -> bool {
    // answer a parameter directly
    if matches!(tree.get(ty), Type::Parameter { .. }) {
        return true;
    }

    // answer an identified child through its declaration alone
    if !root && tree.type_symbol(ty).is_some() {
        return tree
            .type_declaration(ty)
            .is_some_and(|declaration| tree.get(declaration).is_template());
    }

    // check the storage, access, and length written on the type
    let definition = tree.get(ty);
    if mentions_memory_parameter(definition, tree) {
        return true;
    }

    match definition {
        // check only an application's arguments
        Type::Application { arguments, .. } => arguments
            .iter()
            .any(|argument| argument.mentions_parameter(tree)),
        Type::Struct { fields, .. } => fields
            .iter()
            .any(|field| definition_mentions_parameter(tree, tree.get(*field).ty, false)),
        other => {
            let mut found = false;
            other.clone().map_child_type_ids(&mut |child| {
                found = found || definition_mentions_parameter(tree, child, false);
                child
            });

            found
        }
    }
}

/// Return whether the storage, access, or length written on one type is a parameter.
fn mentions_memory_parameter(ty: &Type, tree: &Tree) -> bool {
    match ty {
        Type::Dynamic {
            storage, access, ..
        }
        | Type::Reference {
            storage, access, ..
        }
        | Type::Slice {
            storage, access, ..
        }
        | Type::Function {
            storage, access, ..
        } => {
            matches!(storage.space(), Space::Parameter(_)) || matches!(access, Access::Parameter(_))
        }
        Type::Pointer { access, .. } => matches!(access, Access::Parameter(_)),
        Type::FixedArray { length, .. } => {
            matches!(tree.static_value(*length), Static::Parameter(_))
        }
        _ => false,
    }
}
