use crate::{If, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat an if / else statement.
    ///
    /// Examples:
    /// ```
    /// if x > 0 {
    ///     print("positive")
    /// }
    ///
    /// if x > 0 {
    ///     print("positive")
    /// } else if x == 0 {
    ///     print("zero")
    /// } else {
    ///     print("negative")
    /// }
    /// ```
    pub fn eat_if(&mut self) -> ParseResult<NodeId<If>> {
        todo!()
    }
}
