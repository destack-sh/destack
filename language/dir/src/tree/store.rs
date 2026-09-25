use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{
    Argument, AssignPattern, AssignPatternField, Block, Catch, Declaration, Declarator, Decorator,
    DependencyItem, EnumField, Expression, GenericArgument, GenericParameter, LocalNodeId,
    LocalNodeIdAny, MatchArm, Member, Node, NodeFold, NodeType, Parameter, Pattern, PatternField,
    Property, SwitchCase, Tree, TreeAttribute, TreeChild, TupleElement, TypeExpression,
    TypeMappedParameter, TypeMember, View, WhereClause,
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

macro_rules! tree_nodes {
    ( $( $ty:ident => $field:ident ),+ $(,)? ) => {
        /// One owned DIR node value.
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
        pub enum NodeValue {
            $(
                #[doc = concat!("One `", stringify!($ty), "` node.")]
                $ty($ty),
            )*
        }

        impl View<'_> {
            /// Clone one visible node into an erased value.
            pub fn clone_node(&self, node: LocalNodeIdAny) -> Option<NodeValue> {
                if !self.is_visible(node) {
                    return None;
                }

                Some(match node.ty {
                    $(
                        NodeType::$ty => {
                            let node = LocalNodeId::<$ty>::new(node.id);

                            NodeValue::$ty(self.get(node).clone())
                        }
                    )*
                })
            }
        }

        $(
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
        )*
    };
}

tree_nodes! {
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
