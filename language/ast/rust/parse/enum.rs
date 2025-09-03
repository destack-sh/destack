//! Parse unions and enums (which are just sugar for unions).

use crate::{Enum, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat an enum declaration.
    ///
    /// Examples:
    /// ```
    /// // anonymous enum (for use as a value)
    /// enum { Success, Failure }
    ///
    /// enum Foo {
    ///     A // semicolon optional
    ///     B
    ///     C
    /// }
    ///
    /// enum(u8) Foo {
    ///     Baz = 1
    ///     Qux = 2
    /// }
    /// ```
    pub fn eat_enum(&mut self) -> ParseResult<NodeId<Enum>> {
        todo!()
    }

    /// Eat an enum body (without the header or `{` and `}`)
    pub fn eat_enum_body(&mut self) -> ParseResult<NodeId<Enum>> {
        todo!()
    }
}
