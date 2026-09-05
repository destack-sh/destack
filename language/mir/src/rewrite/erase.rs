use crate::{Tree, Type, TypeId};

/// Return one type with its top-level lifetimes erased.
pub fn erase_lifetime(tree: &mut Tree, ty: TypeId) -> TypeId {
    // erase through an application, keeping its arguments
    if let Type::Application {
        base, arguments, ..
    } = tree.get(ty)
    {
        let (base, arguments) = (*base, arguments.clone());
        let base = erase_lifetime(tree, base);
        if arguments.is_empty() {
            return base;
        }

        return tree.intern_type(Type::Application {
            base,
            arguments,
            lifetimes: Vec::new(),
        });
    }

    // erase the lifetime written on the type itself
    let erased = tree.get(ty).erased_lifetime();
    if erased == *tree.get(ty) {
        return ty;
    }

    tree.intern_type(erased)
}
