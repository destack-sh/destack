//! Parse calls, static calls, dynamic calls, etc.

use crate::{DynamicCall, NodeId, ParseResult, Parser, StaticCall};

impl<'a> Parser<'a> {
    /// Eat a static call (e.g., `@foo(x: 1, y: 2)`).
    ///
    /// Examples:
    /// ```
    /// @foo()
    /// @foo(1, 2, 3)
    /// @foo<int32>(1, 2, 3)
    /// @foo<Validate: false>(1, 2, 3)
    /// @foo(Vector2 {x: 1, y: 2}, (true, 3))
    /// ```
    pub fn eat_static_call(&mut self) -> ParseResult<NodeId<StaticCall>> {
        todo!()
    }

    /// Eat a dynamic call (e.g., `foo(x: 1, y: 2)`).
    ///
    /// Examples:
    /// ```
    /// foo()
    /// foo(1, 2, 3)
    /// foo(foo.a {x: 1, y: 2}, (true, 3))
    /// foo<true>(1, 2, 3)
    pub fn eat_dynamic_call(&mut self) -> ParseResult<NodeId<DynamicCall>> {
        todo!()
    }
}
