/// A ModuleDefinition is a module definition (either nested or as a whole file).
/// 
/// Example:
/// ```
/// mod foo {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDefinition {
    pub name: String,
}

/// A ModuleDeclaration is a module declaration.
/// 
/// Example:
/// ```
/// mod foo;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDeclaration {
    pub name: String,
}

/// A StructDeclaration is an anonymous struct definition.
/// 
/// Example:
/// ```
/// struct {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StructDeclaration {}

/// A StructDefinition is a named struct definition.
/// 
/// Example:
/// ```
/// struct Foo {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StructDefinition {
    pub name: String,
}

/// An ImplDefinition is an impl definition.
/// 
/// Example:
/// ```
/// impl Foo for Bar {
///     ...
/// }
/// impl Bar<i32> for Baz {
///     ...
/// }
/// impl<T> Bar<T> for Baz {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ImplDefinition {
}

/// A FunctionDeclaration is a function definition without a body.
/// 
/// Example: 
/// ```
/// fn foo();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDeclaration {
	pub name: String,
}

/// A FunctionDefinition is a function definition with a body.
/// 
/// Example:
/// ```
/// fn foo() {
///     println!("Hello, world!");
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDefinition {
	pub name: String,
}

/// A UseDeclaration is a use declaration.
/// 
/// Example:
/// ```
/// use foo::*;
/// use foo::bar;
/// use foo::bar::*;
/// use foo::{bar, baz};
/// use foo as baz;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UseDeclaration {
	pub name: String,
	pub is_glob: bool,
}

/// A FieldDeclaration is a struct field declaration.
/// 
/// Example:
/// ```
/// bar: i32;
/// baz: T;
/// baz: #some_macro(T);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDeclaration {
	pub name: String,
}