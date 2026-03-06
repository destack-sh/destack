use destack_dir as dir;

/// Return true when one declaration has any `extends` heritage entries.
pub fn declaration_has_extends_types(declaration: &dir::Declaration) -> bool {
    let Some(heritage) = declaration.heritage() else {
        return false;
    };

    heritage
        .extends_types
        .as_ref()
        .is_some_and(|types| !types.is_empty())
}

/// Return true when one declaration has any embedded type entries.
pub fn declaration_has_embedded_types(declaration: &dir::Declaration) -> bool {
    let Some(heritage) = declaration.heritage() else {
        return false;
    };

    heritage
        .embedded_types
        .as_ref()
        .is_some_and(|types| !types.is_empty())
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
