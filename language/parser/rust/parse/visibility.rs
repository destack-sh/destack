use crate::{Keyword, Parser, ParserResult, Visibility};

impl<'a> Parser<'a> {
    /// Peek a visibility.
    #[inline]
    pub fn peek_visibility(&self) -> ParserResult<Option<Visibility>> {
        if self.peek_keyword(Keyword::Public).is_ok() {
            Ok(Some(Visibility::Public))
        } else if self.peek_keyword(Keyword::Protected).is_ok() {
            Ok(Some(Visibility::Protected))
        } else if self.peek_keyword(Keyword::Private).is_ok() {
            Ok(Some(Visibility::Private))
        } else {
            Ok(None)
        }
    }
}
