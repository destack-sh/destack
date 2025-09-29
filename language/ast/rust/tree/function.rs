use dyst_source::StringId;

use crate::{
    Block, Node, NodeId, NodeType, Parameter, Runtime, ScopedMutability, Type, Visibility, With,
};

/// A FunctionStyle is the style of a function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionStyle {
    /// A normal function.
    Function,
    /// A lambda function.
    Lambda,
}

/// A Function is function or "lambda" definition or declaration node in the AST.
/// If no body is provided, it is a declaration for a function defined elsewhere.
///
/// Examples:
/// ```
/// // function style
///
/// function () // anonymous function with empty signature
///
/// function foo() // just declaration, no body, no opening `{`
///
/// function foo<T, U>(x: T) => (int32, boolean) with (
///    T: Copy
///    U: Numeric
/// ) {
///    print("Hello, world!")
/// }
///
/// function baz(a: int32, b: boolean) => (
///    MyStruct,
///    boolean
/// ) with Disk, Time { // with can be on next line
///    ...
/// }
///
/// function @comptime() {
///    ...
/// }
///
/// // optional , if newline-delimited
/// function longBar<Validate: boolean>(
///   /// doc comment for `a`
///   a: int32
///   /// doc comment for `b`
///   b: boolean
///   // regular comment
///   c: Vector2
/// ) => (
///    int32,
///    isGood: boolean
/// ) with (
///   Time
/// ) {
///    ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// The name of the function (excluding the `@` prefix if static).
    pub name: Option<StringId>,
    /// The visibility of the union.
    pub visibility: Option<Visibility>,
    /// The runtime of the function (static or dynamic).
    pub runtime: Runtime,
    /// The style of the function (function or lambda).
    pub style: FunctionStyle,
    /// The static parameters to the function.
    pub static_parameters: Option<Vec<NodeId<Parameter>>>,
    /// The self parameter to the function.
    pub self_parameter: Option<SelfParameter>,
    /// The dynamic parameters to the function.
    pub dynamic_parameters: Vec<NodeId<Parameter>>,
    /// The return type of the function.
    pub return_type: Option<NodeId<Type>>,
    /// The with declaration for the function (can't have a body).
    pub with: Option<NodeId<With>>,
    /// The body of the function.
    pub body: Option<NodeId<Block>>,
}

impl Node for Function {
    const KIND: NodeType = NodeType::Function;
}

/// The "self" parameter for a function (also accepts `this` and `&`).
///
/// Examples:
/// ```
/// self
/// var self
/// &self
/// &var self
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SelfParameter {
    /// Whether the self parameter is mutable.
    pub mutability: ScopedMutability,
    /// Whether the self parameter is a pointer.
    pub is_pointer: bool,
}
