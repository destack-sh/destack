use crate::{
    Argument, AssignPattern, AssignPatternField, Block, Catch, Declaration, Declarator, Decorator,
    DependencyItem, EnumField, Expression, GenericArgument, GenericParameter, MatchArm, Member,
    Node, Parameter, Pattern, PatternField, Property, SwitchCase, Tree, TreeAttribute, TreeChild,
    TupleElement, TypeExpression, TypeMappedParameter, TypeMember, WhereClause,
};

/// Map node types to arenas.
pub trait TreeStore<T: Node> {
    /// Allocate a node into the relevant arena.
    fn allocate(tree: &mut Tree, node: T) -> u32;

    /// Get a node from the relevant arena.
    fn get(tree: &Tree, idx: u32) -> &T;

    /// Get a mutable node from the relevant arena.
    fn get_mut(tree: &mut Tree, idx: u32) -> &mut T;
}

macro_rules! impl_tree_store {
    ($ty:ty, $field:ident) => {
        impl TreeStore<$ty> for Tree {
            #[inline]
            fn allocate(tree: &mut Tree, node: $ty) -> u32 {
                tree.$field.allocate(node)
            }

            #[inline]
            fn get(tree: &Tree, idx: u32) -> &$ty {
                tree.$field.get(idx)
            }

            #[inline]
            fn get_mut(tree: &mut Tree, idx: u32) -> &mut $ty {
                tree.$field.get_mut(idx)
            }
        }
    };
}

macro_rules! impl_tree_stores {
    ( $( $ty:ty => $field:ident ),+ $(,)? ) => {
        $( impl_tree_store!($ty, $field); )*
    };
}

impl_tree_stores! {
    Expression => expressions,
    TypeExpression => type_expressions,
    Block => blocks,
    Catch => catches,
    Declaration => declarations,
    Declarator => declarators,
    Property => properties,
    TypeMember => type_members,
    TypeMappedParameter => type_mapped_parameters,
    Member => members,
    EnumField => enum_fields,
    WhereClause => where_clauses,
    DependencyItem => dependency_items,
    GenericParameter => generic_parameters,
    Parameter => parameters,
    GenericArgument => generic_arguments,
    TupleElement => tuple_elements,
    Argument => arguments,
    TreeAttribute => tree_attributes,
    TreeChild => tree_children,
    MatchArm => match_arms,
    Pattern => patterns,
    PatternField => pattern_fields,
    AssignPattern => assign_patterns,
    AssignPatternField => assign_pattern_fields,
    Decorator => decorators,
    SwitchCase => switch_cases,
}
