use dyst_source::StringId;

use crate::{Expression, Node, NodeId, NodeType, Parameter, Visibility, With};

/// A Union is a tagged sum type of structs.
/// Like with structs, the ',' separator is optional if newline-delimited.
///
/// Examples:
/// ```
/// union { // anonymous union (for use as a value)
///     myField: int32
///     myOtherField: boolean
/// }
///
/// union _ {} // explicit anonymous union (for disambiguation)
///
/// union(uint4, uint60) Foo<T> { // 4-bit tag with 60-bit content
///     A
///     B { x: int32, y: T } = 4
///     C(boolean)
///     D(boolean, count: int32) = 6
/// }
///
/// // unions can be tagged with enums and include other types with use (like structs)
/// union(TetrisShapeType) TetrisShape: Entity { // TetrisShape has Entity as super
///     ..TetrisGameObject
///
///     function myFunc() { // nested declaration
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Union {
    /// The name of the union.
    pub name: Option<StringId>,
    /// The visibility of the union.
    pub visibility: Option<Visibility>,
    /// The tag type of the union (if explicitly specified).
    pub tag_type: Option<NodeId<Expression>>,
    /// The representation type of the union (if explicitly specified).
    pub representation_type: Option<NodeId<Expression>>,
    /// The static parameters of the union.
    pub static_parameters: Option<Vec<NodeId<Parameter>>>,
    /// The super types of the union (desugars to `use`-ing other types).
    pub super_types: Option<Vec<NodeId<Expression>>>,
    /// The with declaration of the union.
    pub with: Option<NodeId<With>>,
    /// The fields of the union.
    pub fields: Vec<NodeId<UnionField>>,
    /// The body of the union.
    pub expressions: Vec<NodeId<Expression>>,
}

impl Node for Union {
    const KIND: NodeType = NodeType::Union;
}

/// A UnionField is a union field declaration.
///
/// Examples:
/// ```
/// A
/// A(int32)
/// B { x: int32, y: int32 } = 4
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UnionField {
    /// The name of the union field.
    pub name: StringId,
    /// The type of the union field.
    pub r#type: Option<NodeId<Expression>>,
    /// The default value of the union field.
    pub value: Option<NodeId<Expression>>,
}

impl Node for UnionField {
    const KIND: NodeType = NodeType::UnionField;
}
