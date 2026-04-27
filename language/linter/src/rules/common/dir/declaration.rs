use destack_dir as dir;

/// Return true when one declaration has any `extends` heritage entries.
pub fn declaration_has_extends_heritage(declaration: &dir::Declaration) -> bool {
    match declaration {
        dir::Declaration::Class(declaration) => declaration.extends_expression.is_some(),
        dir::Declaration::Interface(declaration) => !declaration.extends.is_empty(),
        _ => false,
    }
}

/// Return true when one declarataion has any `implements`heritage entries.
pub fn declaration_has_implements_heritage(declaration: &dir::Declaration) -> bool {
    match declaration {
        dir::Declaration::Class(declaration) => !declaration.implements_types.is_empty(),
        dir::Declaration::Struct(declaration) => !declaration.implements_types.is_empty(),
        dir::Declaration::Extension(declaration) => !declaration.implements_types.is_empty(),
        _ => false,
    }
}

/// Return true when one declaration has any embedded type entries.
pub fn declaration_has_embedded_types(declaration: &dir::Declaration) -> bool {
    match declaration {
        dir::Declaration::Struct(declaration) => !declaration.embedded_types.is_empty(),
        _ => false,
    }
}

/// Return true when every member in one list is a field member.
pub fn members_are_all_fields(
    tree: &dir::NodeTree,
    member_ids: &[dir::LocalNodeId<dir::Member>],
) -> bool {
    member_ids
        .iter()
        .all(|member_id| matches!(tree.get(*member_id), dir::Member::Field { .. }))
}
