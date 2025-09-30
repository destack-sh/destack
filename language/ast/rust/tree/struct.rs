use dyst_source::StringId;

use crate::{Expression, Node, NodeId, NodeType, Parameter, Visibility};

/// The style of a struct.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum StructStyle {
    /// A tuple struct with explicit representation.
    Tuple,
    /// A struct with explicit representation.
    Struct,
}

/// A Struct is struct definition node.
/// The ',' separator is optional if newline-delimited.
/// Structs may `use` other structs to include them (just like traits).
/// Structs may also have super structs as semantic sugar for `use`-ing other structs.
///
/// Examples:
/// ```
/// struct {} // empty anonymous struct
///
/// struct _ {} // explicit anonymous struct (for disambiguation)
///
/// struct A() // unit struct (no fields)
///
/// struct Number(int32) // tuple struct (1 field)
///
/// struct Number(int32, isAwesome: boolean) { // tuple struct (2 fields)
///     ...
/// }
///
/// struct { a: int32, b: boolean }
///
/// struct { // anonymous struct (for use as a value)
///     myField: int32 // colon optional
///     myOtherField: boolean
/// }
///
/// struct(uint64) Bar { // 64-bit representation
///     myField: int32
///     myOtherField: boolean
/// }
///
/// struct Foo<T>: Baz { // Foo has a Baz
///     myField: int32
///     myOtherField: T
///
///     let x: int32 = 7 // constant
///
///     use Bar // Foo has a Bar
///
///     function myFunc() { // nested declaration
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    /// The name of the struct.
    pub name: Option<StringId>,
    /// The visibility of the struct.
    pub visibility: Option<Visibility>,
    /// The style of the struct.
    pub style: StructStyle,
    /// The super types of the struct (desugars to `use`-ing other types).
    pub super_types: Option<Vec<NodeId<Expression>>>,
    /// The representation type of the union.
    pub representation_type: Option<NodeId<Expression>>,
    /// The static parameters of the struct.
    pub static_parameters: Option<Vec<NodeId<Parameter>>>,
    /// The fields of the struct.
    pub fields: Vec<NodeId<StructField>>,
    /// The body of the type.
    pub expressions: Vec<NodeId<Expression>>,
}

impl Node for Struct {
    const KIND: NodeType = NodeType::Struct;
}

/// A StructField is a (struct) field declaration.
///
/// Examples:
/// ```
/// bar: int32
/// baz: T
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    /// The name of the field.
    pub name: Option<StringId>,
    /// The type of the field.
    pub r#type: NodeId<Expression>,
    /// The default value of the field.
    pub default: Option<NodeId<Expression>>,
}

// TODO! #Incomplete: getter/setter functions for Struct/Union/...Fields?
//  (how does this interact with traits and unions?)
//  (how does this relate with Entities?)
//  (how does this relate to $ virtualness?)

impl Node for StructField {
    const KIND: NodeType = NodeType::StructField;
}
