use destack_language_token::{SourceFile, TokenSpan, tokenize_semantic};

use crate::Parser;

#[derive(Debug)]
pub struct TestParse<'a> {
    pub tokens: Vec<TokenSpan>,
    pub source: SourceFile<'a>,
}

impl<'a> TestParse<'a> {
    pub fn new(input: &'a str) -> Self {
        let tokens = tokenize_semantic(input);
        let source = SourceFile::new(0, input, input.len() as u32);
        Self { tokens, source }
    }

    /// Get a Parser for this test.
    pub fn parser(&self) -> Parser<'_> {
        Parser::new(self.source.clone(), &self.tokens)
    }
}
