use dyst_source::StringId;

use crate::{Expression, Node, NodeId, NodeType, Type, Visibility};

/// An Enum is an enumeration definition node in the AST.
/// Like with structs, the ',' separator is optional if newline-delimited.
/// Like other types, enums can have super types - since "super" types are just
///  sugar for `use`-ing other types and not implicit subtypes, this is fine and useful.
///
/// Examples:
/// ```
/// // anonymous enum (for use as a value)
/// enum { Success, Failure }
///
/// enum _ {} // explicit anonymous enum (for disambiguation)
///
/// enum Foo {
///     A // colon optional
///     B
///     C
///
///     function myFunc() { // nested declaration
///     }
/// }
///
/// enum(u8) Foo {
///     Baz = 1
///     Qux = 2
/// }
///
/// enum ExtendedDay: Day { // ExtendedDay has Day as super
///     Surfday = 8
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
    /// The name of the enum.
    pub name: Option<StringId>,
    /// The visibility of the enum.
    pub visibility: Option<Visibility>,
    /// The type of the enum (if explicitly specified).
    pub r#type: Option<NodeId<Type>>,
    /// The super types of the enum (desugars to `use`-ing other types).
    pub super_types: Option<Vec<NodeId<Type>>>,
    /// The fields of the enum.
    pub fields: Vec<NodeId<EnumField>>,
    /// The body of the enum.
    pub expressions: Vec<NodeId<Expression>>,
}

impl Node for Enum {
    const KIND: NodeType = NodeType::Enum;
}

/// A EnumField is a enum field declaration.
///
/// Examples:
/// ```
/// A
/// B = 4
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EnumField {
    /// The name of the enum field.
    pub name: StringId,
    /// The default value of the enum field.
    pub value: Option<NodeId<Expression>>,
}

impl Node for EnumField {
    const KIND: NodeType = NodeType::EnumField;
}
