use destack_dir as dir;

use crate::check::CheckModuleState;

impl CheckModuleState {
    /// Return whether a source type is assignable to a target type.
    pub(in crate::check) fn is_type_assignable(
        &self,
        source: dir::LocalTypeId,
        target: dir::LocalTypeId,
    ) -> bool {
        let source = self.get_type(source);
        let target = self.get_type(target);

        is_assignable_type(&source, &target)
    }
}

/// Return whether a source type is assignable to a target type.
fn is_assignable_type(source: &dir::Type, target: &dir::Type) -> bool {
    if source == target {
        return true;
    }

    match (source, target) {
        (_, dir::Type::Any | dir::Type::Unknown) => true,
        (dir::Type::Never, _) => true,
        (dir::Type::Literal(source), dir::Type::Primitive(target)) => {
            literal_matches_primitive(source, *target)
        }
        _ => false,
    }
}

/// Return whether one literal belongs to a primitive family.
fn literal_matches_primitive(source: &dir::ScalarLiteral, target: dir::PrimitiveType) -> bool {
    matches!(
        (source, target),
        (dir::ScalarLiteral::Boolean(_), dir::PrimitiveType::Boolean)
            | (
                dir::ScalarLiteral::Character(_),
                dir::PrimitiveType::Character
            )
            | (dir::ScalarLiteral::String(_), dir::PrimitiveType::String)
            | (dir::ScalarLiteral::Bigint(_), dir::PrimitiveType::Bigint)
            | (
                dir::ScalarLiteral::Integer(_),
                dir::PrimitiveType::Integer(_)
            )
            | (dir::ScalarLiteral::Float(_), dir::PrimitiveType::Float(_))
    )
}
