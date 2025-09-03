use crate::{Implement, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat an implement (incl. `implement` keyword).
    ///
    /// Examples:
    /// ```
    /// implement Foo {
    ///     ...
    /// }
    /// ```
    pub fn eat_implement(&mut self) -> ParseResult<NodeId<Implement>> {
        todo!()
    }
}
