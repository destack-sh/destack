use cssparser::ToCss;
use destack_core::StringPool;

use crate::{Dimension, Number, Symbol, Token};

/// One internal CSS token renderer.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TokenRenderer<'a> {
    /// The pooled strings referenced by tokens.
    strings: &'a StringPool,
}

impl<'a> TokenRenderer<'a> {
    /// Create one token renderer over one string pool.
    pub(crate) fn new(strings: &'a StringPool) -> Self {
        Self { strings }
    }

    /// Render one identifier as canonical CSS source.
    pub(crate) fn render_identifier(&self, value: &str) -> String {
        Self::render_identifier_source(value)
    }

    /// Render one identifier as canonical CSS source without pooled context.
    pub(crate) fn render_identifier_source(value: &str) -> String {
        let mut source = String::new();

        Self::write_raw_cssparser_token(&mut source, cssparser::Token::Ident(value.into()));

        source
    }

    /// Render one number as canonical CSS source without pooled context.
    pub(crate) fn render_number_source(number: Number) -> String {
        let mut source = String::new();

        Self::write_raw_cssparser_token(
            &mut source,
            cssparser::Token::Number {
                has_sign: number.has_sign,
                value: number.value,
                int_value: number.integer_value,
            },
        );

        source
    }

    /// Render one percentage as canonical CSS source without pooled context.
    pub(crate) fn render_percentage_source(number: Number) -> String {
        let mut source = String::new();

        Self::write_raw_cssparser_token(
            &mut source,
            cssparser::Token::Percentage {
                has_sign: number.has_sign,
                unit_value: number.value,
                int_value: number.integer_value,
            },
        );

        source
    }

    /// Render one component token as canonical CSS source.
    pub(crate) fn render_token(&self, token: &Token) -> String {
        let mut source = String::new();

        self.write_token(&mut source, token);

        source
    }

    /// Write one component token into CSS source.
    pub(crate) fn write_token(&self, source: &mut String, token: &Token) {
        match token {
            Token::Ident(value) => self.write_cssparser_token(
                source,
                cssparser::Token::Ident(self.strings.get(*value).into()),
            ),
            Token::AtKeyword(value) => self.write_cssparser_token(
                source,
                cssparser::Token::AtKeyword(self.strings.get(*value).into()),
            ),
            Token::Hash {
                value,
                is_identifier,
            } => {
                if *is_identifier {
                    self.write_cssparser_token(
                        source,
                        cssparser::Token::IDHash(value.as_str().into()),
                    );
                } else {
                    self.write_cssparser_token(
                        source,
                        cssparser::Token::Hash(value.as_str().into()),
                    );
                }
            }
            Token::String(value) => {
                self.write_cssparser_token(
                    source,
                    cssparser::Token::QuotedString(value.as_str().into()),
                );
            }
            Token::UnquotedUrl { value, .. } => {
                self.write_cssparser_token(
                    source,
                    cssparser::Token::UnquotedUrl(value.as_str().into()),
                );
            }
            Token::Delimiter(value) => {
                self.write_cssparser_token(source, cssparser::Token::Delim(*value));
            }
            Token::Number(number) => self.write_number(source, *number),
            Token::Percentage(number) => self.write_percentage(source, *number),
            Token::Dimension(dimension) => self.write_dimension(source, dimension),
            Token::WhiteSpace(value) => source.push_str(value),
            Token::Comment(value) => {
                source.push_str("/*");
                source.push_str(value);
                source.push_str("*/");
            }
            Token::Symbol(symbol) => self.write_symbol(source, *symbol),
            Token::BadUrl(value) => {
                self.write_cssparser_token(source, cssparser::Token::BadUrl(value.as_str().into()));
            }
            Token::BadString(value) => {
                self.write_cssparser_token(
                    source,
                    cssparser::Token::BadString(value.as_str().into()),
                );
            }
        }
    }

    /// Write one number token payload into CSS source.
    pub(crate) fn write_number(&self, source: &mut String, number: Number) {
        self.write_cssparser_token(
            source,
            cssparser::Token::Number {
                has_sign: number.has_sign,
                value: number.value,
                int_value: number.integer_value,
            },
        );
    }

    /// Write one percentage token payload into CSS source.
    pub(crate) fn write_percentage(&self, source: &mut String, number: Number) {
        self.write_cssparser_token(
            source,
            cssparser::Token::Percentage {
                has_sign: number.has_sign,
                unit_value: number.value,
                int_value: number.integer_value,
            },
        );
    }

    /// Write one dimension token payload into CSS source.
    pub(crate) fn write_dimension(&self, source: &mut String, dimension: &Dimension) {
        self.write_cssparser_token(
            source,
            cssparser::Token::Dimension {
                has_sign: dimension.number.has_sign,
                value: dimension.number.value,
                int_value: dimension.number.integer_value,
                unit: self.strings.get(dimension.unit).into(),
            },
        );
    }

    /// Write one punctuation token into CSS source.
    pub(crate) fn write_symbol(&self, source: &mut String, symbol: Symbol) {
        match symbol {
            Symbol::Colon => self.write_cssparser_token(source, cssparser::Token::Colon),
            Symbol::Semicolon => self.write_cssparser_token(source, cssparser::Token::Semicolon),
            Symbol::Comma => self.write_cssparser_token(source, cssparser::Token::Comma),
            Symbol::IncludeMatch => {
                self.write_cssparser_token(source, cssparser::Token::IncludeMatch)
            }
            Symbol::DashMatch => self.write_cssparser_token(source, cssparser::Token::DashMatch),
            Symbol::PrefixMatch => {
                self.write_cssparser_token(source, cssparser::Token::PrefixMatch)
            }
            Symbol::SuffixMatch => {
                self.write_cssparser_token(source, cssparser::Token::SuffixMatch)
            }
            Symbol::SubstringMatch => {
                self.write_cssparser_token(source, cssparser::Token::SubstringMatch);
            }
            Symbol::Cdo => self.write_cssparser_token(source, cssparser::Token::CDO),
            Symbol::Cdc => self.write_cssparser_token(source, cssparser::Token::CDC),
        }
    }

    /// Write one cssparser token into one output string.
    fn write_cssparser_token(&self, source: &mut String, token: cssparser::Token<'_>) {
        Self::write_raw_cssparser_token(source, token);
    }

    /// Write one raw cssparser token into one output string.
    fn write_raw_cssparser_token(source: &mut String, token: cssparser::Token<'_>) {
        token
            .to_css(source)
            .unwrap_or_else(|error| panic!("failed to print css token: {error}"));
    }
}
