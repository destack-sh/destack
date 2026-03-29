use cssparser::ToCss;

use super::printer::Printer;
use crate::{
    BlockKind, ComponentValue, ComponentValueList, Function, Number, SimpleBlock, Symbol, Token,
};

/// Print one component value list as canonical CSS source.
pub fn print_component_value_list(components: &ComponentValueList) -> String {
    Printer::render_component_value_list(components)
}

impl<'a> Printer<'a> {
    /// Render one component value list as canonical CSS source.
    pub(crate) fn render_component_value_list(components: &ComponentValueList) -> String {
        let mut source = String::new();

        Self::write_component_value_list(&mut source, components);

        source
    }

    /// Render one number token payload as canonical CSS source.
    pub(crate) fn render_number(number: Number) -> String {
        Self::render_component_value_list(&ComponentValueList {
            values: vec![ComponentValue::Token(Token::Number(number))],
        })
    }

    /// Write one component value list into CSS source.
    pub(crate) fn write_component_value_list(source: &mut String, components: &ComponentValueList) {
        for value in &components.values {
            Self::write_component_value(source, value);
        }
    }

    /// Write one component value into CSS source.
    pub(crate) fn write_component_value(source: &mut String, value: &ComponentValue) {
        match value {
            ComponentValue::Token(token) => Self::write_token(source, token),
            ComponentValue::Function(function) => Self::write_function(source, function),
            ComponentValue::Block(block) => Self::write_simple_block(source, block),
        }
    }

    /// Write one function into CSS source.
    pub(crate) fn write_function(source: &mut String, function: &Function) {
        Self::write_cssparser_token(
            source,
            cssparser::Token::Function(function.name.as_str().into()),
        );
        Self::write_component_value_list(source, &function.arguments);
        source.push(')');
    }

    /// Write one simple block into CSS source.
    pub(crate) fn write_simple_block(source: &mut String, block: &SimpleBlock) {
        source.push(match block.kind {
            BlockKind::Parenthesis => '(',
            BlockKind::SquareBracket => '[',
            BlockKind::CurlyBracket => '{',
        });
        Self::write_component_value_list(source, &block.value);
        source.push(match block.kind {
            BlockKind::Parenthesis => ')',
            BlockKind::SquareBracket => ']',
            BlockKind::CurlyBracket => '}',
        });
    }

    /// Write one token into CSS source.
    pub(crate) fn write_token(source: &mut String, token: &Token) {
        match token {
            Token::Ident(value) => {
                Self::write_cssparser_token(source, cssparser::Token::Ident(value.as_str().into()))
            }
            Token::AtKeyword(value) => Self::write_cssparser_token(
                source,
                cssparser::Token::AtKeyword(value.as_str().into()),
            ),
            Token::Hash {
                value,
                is_identifier,
            } => {
                if *is_identifier {
                    Self::write_cssparser_token(
                        source,
                        cssparser::Token::IDHash(value.as_str().into()),
                    );
                } else {
                    Self::write_cssparser_token(
                        source,
                        cssparser::Token::Hash(value.as_str().into()),
                    );
                }
            }
            Token::String(value) => Self::write_cssparser_token(
                source,
                cssparser::Token::QuotedString(value.as_str().into()),
            ),
            Token::UnquotedUrl(value) => Self::write_cssparser_token(
                source,
                cssparser::Token::UnquotedUrl(value.as_str().into()),
            ),
            Token::Delimiter(value) => {
                Self::write_cssparser_token(source, cssparser::Token::Delim(*value))
            }
            Token::Number(number) => Self::write_cssparser_token(
                source,
                cssparser::Token::Number {
                    has_sign: number.has_sign,
                    value: number.value,
                    int_value: number.integer_value,
                },
            ),
            Token::Percentage(number) => Self::write_cssparser_token(
                source,
                cssparser::Token::Percentage {
                    has_sign: number.has_sign,
                    unit_value: number.value,
                    int_value: number.integer_value,
                },
            ),
            Token::Dimension(dimension) => Self::write_cssparser_token(
                source,
                cssparser::Token::Dimension {
                    has_sign: dimension.number.has_sign,
                    value: dimension.number.value,
                    int_value: dimension.number.integer_value,
                    unit: dimension.unit.as_str().into(),
                },
            ),
            Token::WhiteSpace(value) => source.push_str(value),
            Token::Comment(value) => {
                source.push_str("/*");
                source.push_str(value);
                source.push_str("*/");
            }
            Token::Symbol(symbol) => Self::write_symbol(source, *symbol),
            Token::BadUrl(value) => {
                Self::write_cssparser_token(source, cssparser::Token::BadUrl(value.as_str().into()))
            }
            Token::BadString(value) => Self::write_cssparser_token(
                source,
                cssparser::Token::BadString(value.as_str().into()),
            ),
        }
    }

    /// Write one symbol into CSS source.
    pub(crate) fn write_symbol(source: &mut String, symbol: Symbol) {
        match symbol {
            Symbol::Colon => Self::write_cssparser_token(source, cssparser::Token::Colon),
            Symbol::Semicolon => Self::write_cssparser_token(source, cssparser::Token::Semicolon),
            Symbol::Comma => Self::write_cssparser_token(source, cssparser::Token::Comma),
            Symbol::IncludeMatch => {
                Self::write_cssparser_token(source, cssparser::Token::IncludeMatch)
            }
            Symbol::DashMatch => Self::write_cssparser_token(source, cssparser::Token::DashMatch),
            Symbol::PrefixMatch => {
                Self::write_cssparser_token(source, cssparser::Token::PrefixMatch)
            }
            Symbol::SuffixMatch => {
                Self::write_cssparser_token(source, cssparser::Token::SuffixMatch)
            }
            Symbol::SubstringMatch => {
                Self::write_cssparser_token(source, cssparser::Token::SubstringMatch)
            }
            Symbol::Cdo => Self::write_cssparser_token(source, cssparser::Token::CDO),
            Symbol::Cdc => Self::write_cssparser_token(source, cssparser::Token::CDC),
        }
    }

    /// Write one cssparser token into one output string.
    fn write_cssparser_token(source: &mut String, token: cssparser::Token<'_>) {
        token
            .to_css(source)
            .unwrap_or_else(|error| panic!("failed to print css token: {error}"));
    }
}
