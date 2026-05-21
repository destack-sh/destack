use crate::Parser;
use destack_dir::{Keyword, Visibility};

impl Parser {
    /// Return the visibility keyword when present.
    #[inline]
    pub fn peek_visibility_is(&mut self) -> Option<Visibility> {
        if self.is_keyword(Keyword::Public) {
            Some(Visibility::Public)
        } else if self.is_keyword(Keyword::Protected) {
            Some(Visibility::Protected)
        } else if self.is_keyword(Keyword::Private) {
            Some(Visibility::Private)
        } else {
            None
        }
    }
}
