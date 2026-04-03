use crate::{
    BlockKind, ComponentValue, ComponentValueList, Dimension, Function, LocalNodeId, NodeTree,
    Number, SimpleBlock, Stylesheet, Symbol, Token,
};
use cssparser::{Parser as CssParser, ParserInput};
use destack_source::{File, Span};

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

/// One authored declaration slice within one block body.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DeclarationSource<'a> {
    /// The authored property name slice.
    pub name: &'a str,
    /// The authored property value slice.
    pub value: &'a str,
    /// The relative property name byte start.
    pub name_start: usize,
    /// The relative property name byte end.
    pub name_end: usize,
    /// The relative property value byte start.
    pub value_start: usize,
    /// The relative property value byte end.
    pub value_end: usize,
}

impl<'a> Parser<'a> {
    /// Create one CSS parser.
    pub fn new(file: &'a File, source: &'a str) -> Self {
        Self { file, source }
    }

    /// Parse one CSS stylesheet from authored source.
    pub fn parse(self) -> Result<(NodeTree, LocalNodeId<Stylesheet>), ParseError> {
        let stylesheet = lightning::LightningStylesheet::parse(
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
        let mut input = ParserInput::new(source);
        let mut parser = CssParser::new(&mut input);

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

    /// Parse one authored declaration block body into source slices.
    pub(crate) fn parse_declaration_block(source: &'a str) -> Vec<DeclarationSource<'a>> {
        let bytes = source.as_bytes();
        let mut declarations = Vec::new();
        let mut index = 0;

        while let Some(next_index) = Self::skip_declaration_trivia(bytes, index) {
            // nested top level at rules
            if bytes[next_index] == b'@' {
                index = Self::skip_top_level_at_rule(bytes, next_index);
                continue;
            }

            let name_start = next_index;

            // property name and value
            let Some((name_end, colon_index)) = Self::scan_declaration_name(bytes, name_start)
            else {
                break;
            };
            let value_start = Self::skip_ascii_whitespace(bytes, colon_index + 1);
            let (value_end, next_index) = Self::scan_declaration_value(bytes, value_start);

            declarations.push(DeclarationSource {
                name: &source[name_start..name_end],
                value: &source[value_start..value_end],
                name_start,
                name_end,
                value_start,
                value_end,
            });

            index = next_index;
        }

        declarations
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

    /// Skip leading declaration whitespace, comments, and empty semicolons.
    fn skip_declaration_trivia(bytes: &[u8], mut index: usize) -> Option<usize> {
        while index < bytes.len() {
            if bytes[index].is_ascii_whitespace() || bytes[index] == b';' {
                index += 1;
                continue;
            }

            if bytes.get(index) == Some(&b'/') && bytes.get(index + 1) == Some(&b'*') {
                index += 2;

                while index + 1 < bytes.len() {
                    if bytes[index] == b'*' && bytes[index + 1] == b'/' {
                        index += 2;
                        break;
                    }

                    index += 1;
                }

                continue;
            }

            return Some(index);
        }

        None
    }

    /// Scan one declaration name up to the top-level colon.
    fn scan_declaration_name(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
        let mut index = start;
        let mut state = DeclarationScanState::default();

        while index < bytes.len() {
            let next = state.skip(bytes, index);

            if next != index {
                index = next;
                continue;
            }

            if state.is_top_level() {
                if bytes[index] == b':' {
                    let name_end = Self::trim_ascii_whitespace_end(bytes, start, index);

                    return Some((name_end, index));
                }

                if bytes[index] == b';' {
                    return None;
                }
            }

            state.advance(bytes[index]);
            index += 1;
        }

        None
    }

    /// Scan one declaration value up to the next top-level semicolon.
    fn scan_declaration_value(bytes: &[u8], start: usize) -> (usize, usize) {
        let mut index = start;
        let mut state = DeclarationScanState::default();

        while index < bytes.len() {
            let next = state.skip(bytes, index);

            if next != index {
                index = next;
                continue;
            }

            if state.is_top_level() && bytes[index] == b';' {
                let value_end = Self::trim_ascii_whitespace_end(bytes, start, index);

                return (value_end, index + 1);
            }

            state.advance(bytes[index]);
            index += 1;
        }

        let value_end = Self::trim_ascii_whitespace_end(bytes, start, bytes.len());

        (value_end, bytes.len())
    }

    /// Skip one top level nested at rule within one declaration block.
    fn skip_top_level_at_rule(bytes: &[u8], start: usize) -> usize {
        let mut index = start;
        let mut state = DeclarationScanState::default();

        while index < bytes.len() {
            let next = state.skip(bytes, index);

            if next != index {
                index = next;
                continue;
            }

            // top level at rule boundary
            if state.is_top_level() {
                if bytes[index] == b';' {
                    return index + 1;
                }

                if bytes[index] == b'{' {
                    state.advance(bytes[index]);
                    index += 1;

                    while index < bytes.len() {
                        let next = state.skip(bytes, index);

                        if next != index {
                            index = next;
                            continue;
                        }

                        if bytes[index] == b'}' && state.brace_depth == 1 {
                            return index + 1;
                        }

                        state.advance(bytes[index]);
                        index += 1;
                    }

                    return bytes.len();
                }
            }

            state.advance(bytes[index]);
            index += 1;
        }

        bytes.len()
    }

    /// Skip one ASCII whitespace run.
    fn skip_ascii_whitespace(bytes: &[u8], mut index: usize) -> usize {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }

        index
    }

    /// Trim one trailing ASCII whitespace run.
    fn trim_ascii_whitespace_end(bytes: &[u8], start: usize, mut end: usize) -> usize {
        while end > start && bytes[end - 1].is_ascii_whitespace() {
            end -= 1;
        }

        end
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

/// One scan state for authored declaration source.
#[derive(Debug, Default, Clone, Copy)]
struct DeclarationScanState {
    /// The parenthesis nesting depth.
    parenthesis_depth: usize,
    /// The bracket nesting depth.
    bracket_depth: usize,
    /// The brace nesting depth.
    brace_depth: usize,
    /// The current string delimiter.
    string_delimiter: Option<u8>,
}

impl DeclarationScanState {
    /// Return whether the scanner is at top level.
    fn is_top_level(&self) -> bool {
        self.parenthesis_depth == 0 && self.bracket_depth == 0 && self.brace_depth == 0
    }

    /// Advance this state by one ordinary byte.
    fn advance(&mut self, byte: u8) {
        match byte {
            b'(' => self.parenthesis_depth += 1,
            b')' => self.parenthesis_depth = self.parenthesis_depth.saturating_sub(1),
            b'[' => self.bracket_depth += 1,
            b']' => self.bracket_depth = self.bracket_depth.saturating_sub(1),
            b'{' => self.brace_depth += 1,
            b'}' => self.brace_depth = self.brace_depth.saturating_sub(1),
            b'\'' | b'"' => self.string_delimiter = Some(byte),
            _ => {}
        }
    }

    /// Skip comments and string bodies.
    fn skip(&mut self, bytes: &[u8], index: usize) -> usize {
        if let Some(delimiter) = self.string_delimiter {
            let mut index = index;

            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index += 2;
                    continue;
                }

                index += 1;

                if bytes[index - 1] == delimiter {
                    self.string_delimiter = None;
                    break;
                }
            }

            return index;
        }

        if bytes.get(index) == Some(&b'/') && bytes.get(index + 1) == Some(&b'*') {
            let mut index = index + 2;

            while index + 1 < bytes.len() {
                if bytes[index] == b'*' && bytes[index + 1] == b'/' {
                    return index + 2;
                }

                index += 1;
            }

            return bytes.len();
        }

        index
    }
}

/// Parse one CSS stylesheet from authored source.
pub fn parse_css(
    file: &File,
    source: &str,
) -> Result<(NodeTree, LocalNodeId<Stylesheet>), ParseError> {
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

    /// Preserve authored declarations in `@page` blocks with nested margin rules.
    #[test]
    fn test_roundtrip_preserves_page_declarations_with_nested_margin_rules() {
        let file = File::from_text(
            FileId::new(1),
            "style.css".to_string(),
            Uri::from_string("test:///style.css"),
            None,
            FileType::Css,
            String::new(),
        );
        let source = "@page :left{size:a4;margin:1cm;@top-left{content:\"x\";color:red!important}}";
        let (tree, stylesheet) = parse_css(&file, source).unwrap();

        assert_eq!(
            print_stylesheet(&tree, stylesheet),
            "@page :left{size:a4;margin:1cm;@top-left{content:\"x\";color:red!important}}"
        );
    }
}
