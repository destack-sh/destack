use crate::{NodeId, ParseResult, Parser, Trait};

impl<'a> Parser<'a> {
    /// Eat a Trait.
    ///
    /// Examples:
    /// ```
    /// trait Foo {
    ///     let x: T
    ///     function foo() => T
    /// }
    /// ```
    pub fn eat_trait(&mut self) -> ParseResult<NodeId<Trait>> {
        todo!()
    }
}
