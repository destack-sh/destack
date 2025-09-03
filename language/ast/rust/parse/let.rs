use crate::{Let, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat a let or var binding (incl. `let` or `var` keyword).
    ///
    /// Examples:
    /// ```
    /// let x = 1
    /// let x: i32 = 1
    /// if let Some(x) = someFunction() {
    ///     ...
    /// }
    /// var x = 1
    /// var x: i32 = 1
    /// var x: int32 // implicitly uninitialized, must be set before use
    /// var x: [float64; 3] = --- // explicitly uninitialized, can do whatever
    /// if var Some(x) = someFunction() {
    ///     ...
    /// }
    pub fn eat_let(&mut self) -> ParseResult<NodeId<Let>> {
        todo!()
    }
}
