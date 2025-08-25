use crate::{DynamicCall, ParseResult, Parser, StaticCall};

impl<'a> Parser<'a> {
    /// Eat a static call (e.g., `@foo.bar(x: 1, y: 2)`).
    pub fn eat_static_call(&mut self) -> ParseResult<StaticCall> {
        todo!()
    }

    /// Eat a dynamic call (e.g., `foo.bar(x: 1, y: 2)`).
    pub fn eat_dynamic_call(&mut self) -> ParseResult<DynamicCall> {
        todo!()
    }
}
