use crate::{Keyword, ParseResult, Parser, Using};

impl<'a> Parser<'a> {
    // /// Eat a using declaration (including keyword and semicolon).
    // pub fn eat_using(&mut self) -> ParseResult<'a, Using> {
    //     self.eat_keyword(Keyword::Using)?;
    //     let using = self.eat_using_content()?;
    //     self.eat_semicolon()?;
    //     Ok(using)
    // }

    // /// Eat the content of a using declaration (without the `using` keyword).
    // pub fn eat_using_content(&mut self) -> ParseResult<'a, Using> {
    //     let path = self.eat_path()?;
    //     let using: Using;
    //     if self.peek_keyword(Keyword::As).is_ok() {
    //         self.bump();
    //         let alias = self.eat_identifier()?;
    //         using.alias = Some(alias);
    //     }
    //     Ok(using)
    // }

    // /// Eat a using item (like `geometry` or `geometry as geom`).
    // pub fn eat_using_item(&mut self) -> ParseResult<'a, UsingItem> {
    //     todo!()
    // }
}

#[cfg(test)]
mod tests {
    use destack_language_lexer::tokenize_semantic;

    use crate::Parser;

    #[test]
    fn test_using() {
        let input = r##"
using destack;
using destack.geometry;
using destack as ds;
using ds.geometry as geom;
using ds.geometry.{Vector2, Vector3 as V3};
"##;
        let tokens = tokenize_semantic(input);
        println!("{tokens:?}");
        let mut parser = Parser::new(&tokens);
    }
}
