use crate::{Expression, ParseResult, Parser, Using};

impl<'a> Parser<'a> {
    /// Eat a using declaration.
    pub fn eat_using(&mut self) -> ParseResult<'a, Using> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use destack_language_lexer::tokenize_semantic;

    #[test]
    fn test_using() {
        let input = r##"
using destack;
using destack.geometry;
using destack as ds;
using ds.geometry as geom;
using ds.geometry.{Vector2, Vector3};
"##;
        let tokens = tokenize_semantic(input);
        println!("{tokens:?}");
    }
}
