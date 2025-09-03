//! Parse calls, static calls, dynamic calls, etc.

use crate::{Call, Index, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat an index (excluding the receiver).
    ///
    /// Examples:
    /// ```
    /// foo[1]
    /// foo[1..3]
    /// foo["bar"]
    /// foo().result[0][variable+1]
    /// ```
    pub fn eat_index(&mut self) -> ParseResult<NodeId<Index>> {
        todo!()
    }

    /// Eat a static call (e.g., `@foo(x: 1, y: 2)`, excluding the receiver).
    ///
    /// Examples:
    /// ```
    /// foo()
    /// @foo(1, 2, 3)
    /// foo<int32>(1, 2, 3)
    /// foo<Validate: false>(1, 2, 3)
    /// @foo(Vector2 {x: 1, y: 2}, (true, 3))
    /// ```
    pub fn eat_call(&mut self) -> ParseResult<NodeId<Call>> {
        todo!()
    }
}
