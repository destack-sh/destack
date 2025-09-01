//! Parse structs.

use crate::{ParseResult, Parser, Type};

impl<'a> Parser<'a> {
    /// Eat a struct.
    pub fn eat_struct(&mut self) -> ParseResult<Type> {
        todo!()
    }

    // Eat a struct body (without the header or `{` and `}`)
    pub fn eat_struct_body(&mut self) -> ParseResult<Type> {
        todo!()
    }
}
