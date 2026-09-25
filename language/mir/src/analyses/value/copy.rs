use crate::{
    GenericParameter, GenericParameterDomain, LanguageItem, Multiplicity, Substitution, Tree, Type,
    TypeId,
};

/// Return whether one type copies under the bounds the generics in scope declare.
pub fn is_copy(tree: &Tree, ty: TypeId, generics: &[GenericParameter]) -> bool {
    let mut active = Vec::new();

    decide_copy(tree, ty, generics, &mut active)
}

/// Decide copying with the types under decision tracked, a recursive occurrence deciding nothing.
fn decide_copy(
    tree: &Tree,
    ty: TypeId,
    generics: &[GenericParameter],
    active: &mut Vec<TypeId>,
) -> bool {
    if active.contains(&ty) {
        return true;
    }
    active.push(ty);
    let copies = match tree.get(ty) {
        Type::Error
        | Type::Never
        | Type::Void
        | Type::Null
        | Type::Boolean
        | Type::Character
        | Type::Int { .. }
        | Type::Isize
        | Type::Usize
        | Type::Float(_)
        | Type::TypeId
        | Type::Pointer { .. }
        | Type::FunctionSignature { .. }
        | Type::FunctionPointer { .. } => true,
        // an open projection moves until its witness answers for it
        Type::Witness { .. } => false,
        // a parameter copies under a Copy bound
        Type::Parameter { index, .. } => {
            generics
                .get(*index as usize)
                .is_some_and(|parameter| match &parameter.domain {
                    GenericParameterDomain::Type { bounds } => bounds
                        .iter()
                        .any(|bound| LanguageItem::Copy.is_bound_by(tree, *bound)),
                    _ => false,
                })
        }
        // an exclusive or open access moves with its reference, an aliasable one copies
        Type::Dynamic { kind, access, .. }
        | Type::Reference { kind, access, .. }
        | Type::Slice { kind, access, .. } => kind.copies() && access.copies(),
        Type::Function {
            kind,
            access,
            multiplicity,
            ..
        } => kind.copies() && access.copies() && *multiplicity == Multiplicity::Repeatable,
        Type::Uninit { .. } => false,
        Type::ManuallyDrop { value } => decide_copy(tree, *value, generics, active),
        Type::FixedArray { element, .. } | Type::Vector { element, .. } => {
            decide_copy(tree, *element, generics, active)
        }
        Type::Tuple { elements } => elements
            .iter()
            .all(|element| decide_copy(tree, *element, generics, active)),
        Type::Struct { fields } => fields
            .iter()
            .all(|field| decide_copy(tree, tree.get(*field).ty, generics, active)),
        Type::Newtype { value } => decide_copy(tree, *value, generics, active),
        Type::Variant { cases, .. } => cases
            .iter()
            .all(|case| decide_copy(tree, case.ty, generics, active)),
        // a declaration copies when it derives Copy and its stored values copy
        Type::Declaration { declaration } => {
            let declaration = tree.get(*declaration);
            declaration.derives_copy
                && declaration
                    .definition
                    .is_some_and(|definition| decide_copy(tree, definition, generics, active))
        }
        Type::Application { base, .. } => {
            let resolved = Substitution::resolve(ty, tree);
            let derives = tree
                .type_declaration(*base)
                .is_some_and(|declaration| tree.get(declaration).derives_copy);

            derives && resolved != ty && decide_copy(tree, resolved, generics, active)
        }
    };
    active.pop();

    copies
}
