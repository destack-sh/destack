use dyst_source::StringId;

use crate::{Expression, Node, NodeId, NodeType, Parameter, Visibility, With};

/// An Enum is an enumeration definition node.
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
///
/// enum Machine<T: int32 = 3, IsSomething: boolean = true> {
///     A = 1
///     B = T
///     @if(IsSomething)
///     C = 3
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
    /// The name of the enum.
    pub name: Option<StringId>,
    /// The visibility of the enum.
    pub visibility: Option<Visibility>,
    /// The type of the enum (if explicitly specified).
    pub tag_type: Option<NodeId<Expression>>,
    /// The static parameters of the enum.
    pub static_parameters: Option<Vec<NodeId<Parameter>>>,
    /// The super types of the enum (desugars to `use`-ing other types).
    pub super_types: Option<Vec<NodeId<Expression>>>,
    /// The with declaration of the enum.
    pub with: Option<NodeId<With>>,
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
