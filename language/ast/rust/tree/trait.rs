use dyst_source::StringId;

use crate::{Expression, Node, NodeId, NodeType, Parameter, Visibility, With};

/// A Trait is trait definition node defining behavior and constants.
/// Traits can `use` other traits to include them (just like structs / unions).
/// Traits can also have super traits as semantic sugar for `use`-ing other traits.
///
/// Examples:
/// ```
/// trait { // anonymous trait
///     ...
/// }
///
/// trait _ {} // explicit anonymous trait (for disambiguation)
///
/// trait Foo: Baz { // Foo is a super
///     ..Bar
///     ..Boz
///     
///     let x: int32 // constant
///     function foo() => int32
///
///     function myFunc() { // nested declaration, default implementation
///     }
/// }
///
/// trait Baz<T> {
///     use Bar
///
///     function baz() => T // semicolon optional
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Trait {
    /// The name of the trait.
    pub name: Option<StringId>,
    /// The visibility of the trait.
    pub visibility: Option<Visibility>,
    /// The super types of the trait.
    pub super_types: Option<Vec<NodeId<Expression>>>,
    /// The static parameters to the trait.
    pub static_parameters: Option<Vec<NodeId<Parameter>>>,
    /// The with declarations of the trait.
    pub withs: Vec<NodeId<With>>,
    /// The body of the trait.
    pub expressions: Vec<NodeId<Expression>>,
}

impl Node for Trait {
    const KIND: NodeType = NodeType::Trait;
}
