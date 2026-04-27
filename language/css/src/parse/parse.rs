use crate::{
    BlockKind, ComponentFragment, ComponentValue, ComponentValueList, Dimension, Function,
    LocalNodeId, Number, SimpleBlock, Stylesheet, Symbol, Token, Tree, UrlResource,
};
use cssparser::{Parser as CssParser, ParserInput};
use destack_core::StringPool;
use destack_source::{File, FileId, Span};

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

/// One authored keyframe slice within one `@keyframes` block body.
#[derive(Debug, Clone, Copy)]
pub(crate) struct KeyframeSource<'a> {
    /// The authored keyframe selector slice.
    pub selector: &'a str,
    /// The authored keyframe declaration body slice.
    pub body: &'a str,
    /// The relative keyframe rule byte start.
    pub rule_start: usize,
    /// The relative keyframe rule byte end.
    pub rule_end: usize,
    /// The relative keyframe body byte start.
    pub body_start: usize,
}

impl<'a> Parser<'a> {
    /// Create one CSS parser.
    pub fn new(file: &'a File, source: &'a str) -> Self {
        Self { file, source }
    }

    /// Parse one CSS stylesheet from authored source.
    pub fn parse(self) -> Result<(Tree, LocalNodeId<Stylesheet>), ParseError> {
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

        let (tree, stylesheet_id) =
            Lowerer::new(self.file, self.source).lower_stylesheet(stylesheet);

        Ok((tree, stylesheet_id))
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

    /// Parse one canonical component fragment from source.
    pub fn parse_component_fragment(source: &str) -> (Tree, LocalNodeId<ComponentFragment>) {
        let mut tree = Tree::new();
        let mut next_resource_id = 0;
        let value = Self::parse_component_value_list_with_pool(
            &tree.strings,
            &mut next_resource_id,
            source,
        );
        let fragment = tree.insert(ComponentFragment { value }, Span::empty(FileId::new(0)));

        (tree, fragment)
    }

    /// Parse one canonical component value list into one provided pool.
    pub(crate) fn parse_component_value_list_with_pool(
        strings: &StringPool,
        next_resource_id: &mut u32,
        source: &str,
    ) -> ComponentValueList {
        let mut input = ParserInput::new(source);
        let mut parser = CssParser::new(&mut input);
        let mut values = Vec::new();

        if let Err(error) =
            Self::parse_component_values(strings, &mut parser, &mut values, 0, next_resource_id)
        {
            panic!("failed to parse canonical css component values: {error:?}");
        }

        ComponentValueList { values }
    }

    /// Parse one authored declaration value and split one trailing `!important`.
    pub(crate) fn parse_declaration_value_with_pool(
        strings: &StringPool,
        next_resource_id: &mut u32,
        source: &str,
    ) -> (ComponentValueList, bool) {
        let mut values =
            Self::parse_component_value_list_with_pool(strings, next_resource_id, source).values;
        let is_important = Self::split_trailing_important(strings, &mut values);

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

    /// Parse one authored `@keyframes` block body into keyframe source slices.
    pub(crate) fn parse_keyframe_block(source: &'a str) -> Vec<KeyframeSource<'a>> {
        let bytes = source.as_bytes();
        let mut keyframes = Vec::new();
        let mut index = 0;

        while let Some(next_index) = Self::skip_qualified_rule_trivia(bytes, index) {
            let rule_start = next_index;

            let Some((selector_end, body_start)) =
                Self::scan_qualified_rule_prelude(bytes, rule_start)
            else {
                break;
            };
            let Some((body_end, rule_end)) = Self::scan_qualified_rule_block(bytes, body_start)
            else {
                break;
            };

            keyframes.push(KeyframeSource {
                selector: &source[rule_start..selector_end],
                body: &source[body_start + 1..body_end],
                rule_start,
                rule_end,
                body_start: body_start + 1,
            });

            index = rule_end;
        }

        keyframes
    }

    /// Split one trailing `!important` marker from parsed component values.
    fn split_trailing_important(strings: &StringPool, values: &mut Vec<ComponentValue>) -> bool {
        let Some(important_index) = Self::last_non_trivia(values) else {
            return false;
        };

        let ComponentValue::Token(Token::Ident(important)) = &values[important_index] else {
            return false;
        };

        if !strings
            .get(*important)
            .as_ref()
            .eq_ignore_ascii_case("important")
        {
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

    /// Skip leading qualified rule trivia.
    fn skip_qualified_rule_trivia(bytes: &[u8], mut index: usize) -> Option<usize> {
        while index < bytes.len() {
            if bytes[index].is_ascii_whitespace() {
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

    /// Scan one qualified rule prelude up to one top level `{`.
    fn scan_qualified_rule_prelude(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
        let mut index = start;
        let mut state = DeclarationScanState::default();

        while index < bytes.len() {
            let next = state.skip(bytes, index);

            if next != index {
                index = next;
                continue;
            }

            if state.is_top_level() && bytes[index] == b'{' {
                let selector_end = Self::trim_ascii_whitespace_end(bytes, start, index);

                return Some((selector_end, index));
            }

            state.advance(bytes[index]);
            index += 1;
        }

        None
    }

    /// Scan one qualified rule block starting at one opening brace.
    fn scan_qualified_rule_block(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
        let mut index = start;
        let mut state = DeclarationScanState::default();

        while index < bytes.len() {
            let next = state.skip(bytes, index);

            if next != index {
                index = next;
                continue;
            }

            if bytes[index] == b'}' && state.brace_depth == 1 {
                let body_end = Self::trim_ascii_whitespace_end(bytes, start + 1, index);

                return Some((body_end, index + 1));
            }

            state.advance(bytes[index]);
            index += 1;
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
        strings: &StringPool,
        parser: &mut CssParser<'i, 't>,
        values: &mut Vec<ComponentValue>,
        depth: usize,
        next_resource_id: &mut u32,
    ) -> Result<(), cssparser::ParseError<'i, ()>> {
        if depth > 500 {
            return Err(parser.new_custom_error(()));
        }

        loop {
            match parser.next_including_whitespace_and_comments() {
                Ok(&cssparser::Token::ParenthesisBlock) => {
                    let value = parser.parse_nested_block(|parser| {
                        let mut values = Vec::new();
                        Self::parse_component_values(
                            strings,
                            parser,
                            &mut values,
                            depth + 1,
                            next_resource_id,
                        )?;
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
                        Self::parse_component_values(
                            strings,
                            parser,
                            &mut values,
                            depth + 1,
                            next_resource_id,
                        )?;
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
                        Self::parse_component_values(
                            strings,
                            parser,
                            &mut values,
                            depth + 1,
                            next_resource_id,
                        )?;
                        Ok(ComponentValueList { values })
                    })?;
                    values.push(ComponentValue::Block(SimpleBlock {
                        kind: BlockKind::CurlyBracket,
                        value,
                    }));
                }
                Ok(cssparser::Token::Function(name)) => {
                    let name = strings.intern(name);
                    let arguments = parser.parse_nested_block(|parser| {
                        let mut values = Vec::new();
                        Self::parse_component_values(
                            strings,
                            parser,
                            &mut values,
                            depth + 1,
                            next_resource_id,
                        )?;
                        Ok(ComponentValueList { values })
                    })?;
                    let url_resource = if strings.get(name).as_ref() == "url" {
                        Some(Self::build_function_url_resource(
                            &arguments,
                            next_resource_id,
                        ))
                    } else {
                        None
                    };

                    values.push(ComponentValue::Function(Function {
                        name,
                        url_resource,
                        arguments,
                    }));
                }
                Ok(token) => values.push(ComponentValue::Token(Self::parse_cssparser_token(
                    strings,
                    token,
                    next_resource_id,
                ))),
                Err(_) => break,
            }
        }

        Ok(())
    }

    /// Parse one cssparser token into one owned token.
    fn parse_cssparser_token(
        strings: &StringPool,
        token: &cssparser::Token<'_>,
        next_resource_id: &mut u32,
    ) -> Token {
        match token {
            cssparser::Token::Ident(value) => Token::Ident(strings.intern(value)),
            cssparser::Token::AtKeyword(value) => Token::AtKeyword(strings.intern(value)),
            cssparser::Token::Hash(value) => Token::Hash {
                value: value.to_string(),
                is_identifier: false,
            },
            cssparser::Token::IDHash(value) => Token::Hash {
                value: value.to_string(),
                is_identifier: true,
            },
            cssparser::Token::QuotedString(value) => Token::String(value.to_string()),
            cssparser::Token::UnquotedUrl(value) => Token::UnquotedUrl {
                value: value.to_string(),
                url_resource: Some(Self::build_url_resource(value, next_resource_id)),
            },
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
                unit: strings.intern(unit),
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

    /// Build one `url(...)` resource payload from parsed function arguments.
    fn build_function_url_resource(
        arguments: &ComponentValueList,
        next_resource_id: &mut u32,
    ) -> UrlResource {
        let specifier = arguments
            .values
            .iter()
            .find_map(|value| match value {
                ComponentValue::Token(Token::String(value))
                | ComponentValue::Token(Token::UnquotedUrl { value, .. }) => Some(value.as_str()),
                ComponentValue::Token(Token::WhiteSpace(_)) => None,
                _ => None,
            })
            .unwrap_or_default();

        Self::build_url_resource(specifier, next_resource_id)
    }

    /// Build one rewriteable url resource payload.
    fn build_url_resource(specifier: &str, next_resource_id: &mut u32) -> UrlResource {
        UrlResource::new(allocate_resource_id(next_resource_id), specifier)
    }
}

/// Return one stable resource id.
fn allocate_resource_id(next_resource_id: &mut u32) -> u32 {
    let id = *next_resource_id;
    *next_resource_id += 1;
    id
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
pub fn parse_css(file: &File, source: &str) -> Result<(Tree, LocalNodeId<Stylesheet>), ParseError> {
    Parser::new(file, source).parse()
}
