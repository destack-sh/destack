//! Parse calls, static calls, dynamic calls, etc.

use crate::{Expression, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat a static call (e.g., `@foo.bar(x: 1, y: 2)`).
    pub fn eat_static_call(&mut self) -> ParseResult<NodeId<Expression>> {
        todo!()
    }

    /// Eat a dynamic call (e.g., `foo.bar(x: 1, y: 2)`).
    pub fn eat_dynamic_call(&mut self) -> ParseResult<NodeId<Expression>> {
        todo!()
    }

    /// Eat a call body (e.g., `path(1, y: 2)`).
    pub fn eat_call_body(&mut self) -> ParseResult<NodeId<Expression>> {
        todo!()
    }
}
