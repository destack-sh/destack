use crate::{
    BlockKind, ComponentValue, ComponentValueList, Dimension, Function, LocalNodeId, NodeTree,
    Number, SimpleBlock, StyleSheet, Symbol, Token,
};
use cssparser::{Parser as CssParser, ParserInput};
use destack_source::{File, Span};
use selector_cssparser::{Parser as SelectorCssParser, ParserInput as SelectorParserInput};

use super::lightning;
use super::lower::Lowerer;

/// One CSS parse error.
#[derive(Debug, Clone)]
pub struct ParseError {
    /// The source span of the parse failure.
    pub span: Span,
    /// The parser error message.
    pub message: String,
}

/// One CSS parser entrypoint.
#[derive(Debug)]
pub struct Parser<'a> {
    /// The authored source file.
    file: &'a File,
    /// The authored CSS source.
    source: &'a str,
}

impl<'a> Parser<'a> {
    /// Create one CSS parser.
    pub fn new(file: &'a File, source: &'a str) -> Self {
        Self { file, source }
    }

    /// Parse one CSS stylesheet from authored source.
    pub fn parse(self) -> Result<(NodeTree, LocalNodeId<StyleSheet>), ParseError> {
        let stylesheet = lightning::StyleSheet::parse(
            self.source,
            lightning::ParserOptions {
                filename: self
                    .file
                    .path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| self.file.uri.to_string()),
                ..lightning::ParserOptions::default()
            },
        )
        .map_err(|error| {
            let span = error
                .loc
                .as_ref()
                .map(|location| {
                    let offset = File::byte_offset_from_position(
                        self.source,
                        location.line as usize,
                        location.column as usize,
                    );

                    Span::at(self.file.id, offset, 1)
                })
                .unwrap_or_else(|| Span::empty(self.file.id));

            ParseError {
                span,
                message: error.to_string(),
            }
        })?;

        Ok(Lowerer::new(self.file, self.source).lower_stylesheet(stylesheet))
    }

    /// Parse one canonical selector list from source.
    pub(crate) fn parse_selector_list_source(source: &'a str) -> lightning::SelectorList<'a> {
        let mut input = SelectorParserInput::new(source);
        let mut parser = SelectorCssParser::new(&mut input);

        lightning::ParseWithOptions::parse_with_options(
            &mut parser,
            &lightning::ParserOptions::default(),
        )
        .unwrap_or_else(|error| panic!("failed to parse canonical css selector list: {error:?}"))
    }

    /// Parse one canonical component value list from source.
    pub(crate) fn parse_component_value_list(source: &str) -> ComponentValueList {
        let mut input = ParserInput::new(source);
        let mut parser = CssParser::new(&mut input);
        let mut values = Vec::new();

        if let Err(error) = Self::parse_component_values(&mut parser, &mut values, 0) {
            panic!("failed to parse canonical css component values: {error:?}");
        }

        ComponentValueList { values }
    }

    /// Parse one authored declaration value and split one trailing `!important`.
    pub(crate) fn parse_declaration_value(source: &str) -> (ComponentValueList, bool) {
        let mut values = Self::parse_component_value_list(source).values;
        let is_important = Self::split_trailing_important(&mut values);

        (ComponentValueList { values }, is_important)
    }

    /// Split one trailing `!important` marker from parsed component values.
    fn split_trailing_important(values: &mut Vec<ComponentValue>) -> bool {
        let Some(important_index) = Self::last_non_trivia(values) else {
            return false;
        };

        let ComponentValue::Token(Token::Ident(important)) = &values[important_index] else {
            return false;
        };

        if !important.eq_ignore_ascii_case("important") {
            return false;
        }

        let Some(bang_index) = Self::last_non_trivia_before(values, important_index) else {
            return false;
        };

        let ComponentValue::Token(Token::Delimiter('!')) = values[bang_index] else {
            return false;
        };

        values.truncate(bang_index);
        true
    }

    /// Return the last non-trivia component index.
    fn last_non_trivia(values: &[ComponentValue]) -> Option<usize> {
        values.iter().rposition(|value| !Self::is_trivia(value))
    }

    /// Return the last non-trivia component index before one position.
    fn last_non_trivia_before(values: &[ComponentValue], before: usize) -> Option<usize> {
        values[..before]
            .iter()
            .rposition(|value| !Self::is_trivia(value))
    }

    /// Return whether one component value is trivia.
    fn is_trivia(value: &ComponentValue) -> bool {
        matches!(
            value,
            ComponentValue::Token(Token::WhiteSpace(_) | Token::Comment(_))
        )
    }

    /// Parse nested component values.
    fn parse_component_values<'i, 't>(
        parser: &mut CssParser<'i, 't>,
        values: &mut Vec<ComponentValue>,
        depth: usize,
    ) -> Result<(), cssparser::ParseError<'i, ()>> {
        if depth > 500 {
            return Err(parser.new_custom_error(()));
        }

        loop {
            match parser.next_including_whitespace_and_comments() {
                Ok(&cssparser::Token::ParenthesisBlock) => {
                    let value = parser.parse_nested_block(|parser| {
                        let mut values = Vec::new();
                        Self::parse_component_values(parser, &mut values, depth + 1)?;
                        Ok(ComponentValueList { values })
                    })?;
                    values.push(ComponentValue::Block(SimpleBlock {
                        kind: BlockKind::Parenthesis,
                        value,
                    }));
                }
                Ok(&cssparser::Token::SquareBracketBlock) => {
                    let value = parser.parse_nested_block(|parser| {
                        let mut values = Vec::new();
                        Self::parse_component_values(parser, &mut values, depth + 1)?;
                        Ok(ComponentValueList { values })
                    })?;
                    values.push(ComponentValue::Block(SimpleBlock {
                        kind: BlockKind::SquareBracket,
                        value,
                    }));
                }
                Ok(&cssparser::Token::CurlyBracketBlock) => {
                    let value = parser.parse_nested_block(|parser| {
                        let mut values = Vec::new();
                        Self::parse_component_values(parser, &mut values, depth + 1)?;
                        Ok(ComponentValueList { values })
                    })?;
                    values.push(ComponentValue::Block(SimpleBlock {
                        kind: BlockKind::CurlyBracket,
                        value,
                    }));
                }
                Ok(cssparser::Token::Function(name)) => {
                    let name = name.to_string();
                    let arguments = parser.parse_nested_block(|parser| {
                        let mut values = Vec::new();
                        Self::parse_component_values(parser, &mut values, depth + 1)?;
                        Ok(ComponentValueList { values })
                    })?;

                    values.push(ComponentValue::Function(Function { name, arguments }));
                }
                Ok(token) => values.push(ComponentValue::Token(Self::parse_cssparser_token(token))),
                Err(_) => break,
            }
        }

        Ok(())
    }

    /// Parse one cssparser token into one owned token.
    fn parse_cssparser_token(token: &cssparser::Token<'_>) -> Token {
        match token {
            cssparser::Token::Ident(value) => Token::Ident(value.to_string()),
            cssparser::Token::AtKeyword(value) => Token::AtKeyword(value.to_string()),
            cssparser::Token::Hash(value) => Token::Hash {
                value: value.to_string(),
                is_identifier: false,
            },
            cssparser::Token::IDHash(value) => Token::Hash {
                value: value.to_string(),
                is_identifier: true,
            },
            cssparser::Token::QuotedString(value) => Token::String(value.to_string()),
            cssparser::Token::UnquotedUrl(value) => Token::UnquotedUrl(value.to_string()),
            cssparser::Token::Delim(value) => Token::Delimiter(*value),
            cssparser::Token::Number {
                has_sign,
                value,
                int_value,
            } => Token::Number(Number {
                has_sign: *has_sign,
                value: *value,
                integer_value: *int_value,
            }),
            cssparser::Token::Percentage {
                has_sign,
                unit_value,
                int_value,
            } => Token::Percentage(Number {
                has_sign: *has_sign,
                value: *unit_value,
                integer_value: *int_value,
            }),
            cssparser::Token::Dimension {
                has_sign,
                value,
                int_value,
                unit,
            } => Token::Dimension(Dimension {
                number: Number {
                    has_sign: *has_sign,
                    value: *value,
                    integer_value: *int_value,
                },
                unit: unit.to_string(),
            }),
            cssparser::Token::WhiteSpace(value) => Token::WhiteSpace(value.to_string()),
            cssparser::Token::Comment(value) => Token::Comment(value.to_string()),
            cssparser::Token::Colon => Token::Symbol(Symbol::Colon),
            cssparser::Token::Semicolon => Token::Symbol(Symbol::Semicolon),
            cssparser::Token::Comma => Token::Symbol(Symbol::Comma),
            cssparser::Token::IncludeMatch => Token::Symbol(Symbol::IncludeMatch),
            cssparser::Token::DashMatch => Token::Symbol(Symbol::DashMatch),
            cssparser::Token::PrefixMatch => Token::Symbol(Symbol::PrefixMatch),
            cssparser::Token::SuffixMatch => Token::Symbol(Symbol::SuffixMatch),
            cssparser::Token::SubstringMatch => Token::Symbol(Symbol::SubstringMatch),
            cssparser::Token::CDO => Token::Symbol(Symbol::Cdo),
            cssparser::Token::CDC => Token::Symbol(Symbol::Cdc),
            cssparser::Token::BadUrl(value) => Token::BadUrl(value.to_string()),
            cssparser::Token::BadString(value) => Token::BadString(value.to_string()),
            cssparser::Token::Function(_)
            | cssparser::Token::ParenthesisBlock
            | cssparser::Token::SquareBracketBlock
            | cssparser::Token::CurlyBracketBlock
            | cssparser::Token::CloseParenthesis
            | cssparser::Token::CloseSquareBracket
            | cssparser::Token::CloseCurlyBracket => {
                panic!("nested css component values must be lowered structurally")
            }
        }
    }
}

/// Parse one CSS stylesheet from authored source.
pub fn parse_css(
    file: &File,
    source: &str,
) -> Result<(NodeTree, LocalNodeId<StyleSheet>), ParseError> {
    Parser::new(file, source).parse()
}

#[cfg(test)]
mod tests {
    use crate::print::{Printer, print_stylesheet};
    use crate::{NodeSpanType, RenderOptions, Rule, parse_css};
    use destack_source::{File, FileId, FileType, Span, Uri};

    /// Preserve one parsed stylesheet root and authored rule headers.
    #[test]
    fn test_roundtrip_owned_stylesheet() {
        let file = File::from_text(
            FileId::new(1),
            "style.css".to_string(),
            Uri::from_string("test:///style.css"),
            None,
            FileType::Css,
            String::new(),
        );
        let source = "@import \"./base.css\" layer(theme);@media screen{.button{color:red;&:hover{color:blue}}}";
        let (tree, stylesheet) = parse_css(&file, source).unwrap();
        let stylesheet = tree.get(stylesheet);

        assert_eq!(stylesheet.rules.len(), 2);

        let import_rule = tree.get(stylesheet.rules[0]);
        let Rule::Import(import_rule) = import_rule else {
            panic!("expected import rule");
        };
        assert_eq!(import_rule.url, "./base.css");
        assert_eq!(
            import_rule
                .layer
                .as_ref()
                .and_then(|layer| layer.name.as_ref())
                .map(|name| name.names.join(".")),
            Some("theme".to_string())
        );

        let media_rule = tree.get(stylesheet.rules[1]);
        let Rule::Media(media_rule) = media_rule else {
            panic!("expected media rule");
        };

        let printer = Printer::new(&tree, RenderOptions::default());

        assert_eq!(printer.render_media_query_list(media_rule.query), "screen");
    }

    /// Preserve full rule spans and rule side spans from authored source.
    #[test]
    fn test_store_rule_and_import_side_spans() {
        let file = File::from_text(
            FileId::new(1),
            "style.css".to_string(),
            Uri::from_string("test:///style.css"),
            None,
            FileType::Css,
            String::new(),
        );
        let source =
            "@import \"./base.css\" layer(theme);\n@media screen {.button { color: red; }}";
        let (tree, stylesheet) = parse_css(&file, source).unwrap();
        let stylesheet = tree.get(stylesheet);
        let import_rule = stylesheet.rules[0];
        let media_rule = stylesheet.rules[1];

        assert_eq!(
            tree.span(import_rule),
            Span::new(
                FileId::new(1),
                0,
                "@import \"./base.css\" layer(theme);".len() as u32
            )
        );
        assert_eq!(
            tree.side_span(import_rule, NodeSpanType::Segment(0)),
            Some(Span::new(
                FileId::new(1),
                0,
                "@import \"./base.css\" layer(theme)".len() as u32,
            ))
        );
        assert_eq!(
            tree.side_span(import_rule, NodeSpanType::Segment(1)),
            Some(Span::new(FileId::new(1), 9, 19))
        );
        assert_eq!(
            tree.side_span(media_rule, NodeSpanType::Segment(0)),
            Some(Span::new(
                FileId::new(1),
                35,
                35 + "@media screen".len() as u32,
            ))
        );
    }

    /// Preserve selector-taking pseudo arguments through parse and print.
    #[test]
    fn test_roundtrip_selector_argument_pseudo_selectors() {
        let file = File::from_text(
            FileId::new(1),
            "style.css".to_string(),
            Uri::from_string("test:///style.css"),
            None,
            FileType::Css,
            String::new(),
        );
        let source = ".root:local(.button,.button:hover)::cue(.caption){color:red}";
        let (tree, stylesheet) = parse_css(&file, source).unwrap();

        assert_eq!(
            print_stylesheet(&tree, stylesheet),
            ":local(.button,.button:hover).root::cue(.caption){color:red}"
        );
    }

    /// Preserve authored declaration order around interleaved `!important`.
    #[test]
    fn test_roundtrip_preserves_interleaved_important_declaration_order() {
        let file = File::from_text(
            FileId::new(1),
            "style.css".to_string(),
            Uri::from_string("test:///style.css"),
            None,
            FileType::Css,
            String::new(),
        );
        let source = ".root{color:red!important;background:blue;border:1px solid red!important}";
        let (tree, stylesheet) = parse_css(&file, source).unwrap();

        assert_eq!(
            print_stylesheet(&tree, stylesheet),
            ".root{color:red!important;background:blue;border:1px solid red!important}"
        );
    }
}
