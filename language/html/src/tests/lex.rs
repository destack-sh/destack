use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::{char, fs};

use serde_json::{Map, Value};

use crate::lex::buffer::{BufferQueue, SetResult};
use crate::lex::lexer::LexerMode::{Data, Plaintext, RawData};
use crate::lex::lexer::{LexerMode, RawKind};
use crate::lex::token::{
    CharacterTokens, CommentToken, Doctype, DoctypeToken, EOFToken, EndTag, LexHandler,
    LexerAction, LexerResult, NullCharacterToken, ParseError, StartTag, Tag, TagKind, TagToken,
    Token,
};
use crate::lex::{
    Attribute, EncodingScanner, HtmlString, Lexer, LexerOptions, LocalName, MetaEncodingScanner,
    QualifiedName, ToHtmlString, lower_ascii_letter, ns, small_char_set,
};

const MAX_SPLITS: usize = 1000;
const HTML_FIXTURE_DIRECTORY: &str = "../test/fixtures/html";
const HTML5LIB_ENCODING_DIRECTORY: &str = "html5lib/encoding";
const HTML5LIB_TOKENIZER_DIRECTORY: &str = "html5lib/tokenizer";
const CUSTOM_TOKENIZER_DIRECTORY: &str = "tokenizer";

/// Push one character into one optional string.
fn push_optional_string(value: &mut Option<HtmlString>, character: char) {
    match *value {
        Some(ref mut string) => string.push_char(character),
        None => *value = Some(HtmlString::from_char(character)),
    }
}

/// Return the local html fixture directory.
fn html_test_directory(subdirectory: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(HTML_FIXTURE_DIRECTORY)
        .join(subdirectory)
}

/// One normalized parse-error marker for tokenizer expectations.
#[derive(Debug)]
struct TestError;

impl PartialEq for TestError {
    fn eq(&self, _: &TestError) -> bool {
        true
    }
}

/// One test logger that tracks lexer line numbers.
struct LineMatcher {
    /// The buffered character token payload.
    current_text: RefCell<HtmlString>,
    /// The observed tokens and their line numbers.
    lines: RefCell<Vec<(Token, u64)>>,
}

impl LineMatcher {
    /// Create one empty line matcher.
    fn new() -> Self {
        Self {
            current_text: RefCell::new(HtmlString::new()),
            lines: RefCell::new(Vec::new()),
        }
    }

    /// Push one completed token.
    fn push(&self, token: Token, line_number: u64) {
        self.finish_text(line_number);
        self.lines.borrow_mut().push((token, line_number));
    }

    /// Flush one pending character token.
    fn finish_text(&self, line_number: u64) {
        if self.current_text.borrow().is_empty() {
            return;
        }

        let text = self.current_text.take();
        self.lines
            .borrow_mut()
            .push((CharacterTokens(text), line_number));
    }
}

impl LexHandler for LineMatcher {
    type Handle = ();

    /// Record one lexer token with its line number.
    fn process_token(&self, token: Token, line_number: u64) -> LexerAction<Self::Handle> {
        // character buffering
        match token {
            CharacterTokens(text) => {
                self.current_text.borrow_mut().push_slice(&text);
            }

            NullCharacterToken => {
                self.current_text.borrow_mut().push_char('\0');
            }

            ParseError(_) => {
                panic!("unexpected parse error");
            }

            // normalize end tag payloads
            TagToken(mut tag) => {
                match tag.kind {
                    EndTag => {
                        tag.self_closing = false;
                        tag.attrs = vec![];
                    }
                    _ => tag.attrs.sort_by(|left, right| left.name.cmp(&right.name)),
                }

                self.push(TagToken(tag), line_number);
            }

            EOFToken => {}

            _ => self.push(token, line_number),
        }

        LexerAction::Continue
    }
}

/// One tokenizer logger that matches the html5lib tokenizer harness.
struct TokenLogger {
    /// The emitted tokens so far.
    tokens: RefCell<Vec<Token>>,
    /// The parse errors so far.
    errors: RefCell<Vec<TestError>>,
    /// The current buffered character data.
    current_text: RefCell<HtmlString>,
    /// Whether exact error counting is enabled.
    exact_errors: bool,
}

impl TokenLogger {
    /// Create one empty token logger.
    fn new(exact_errors: bool) -> Self {
        Self {
            tokens: RefCell::new(Vec::new()),
            errors: RefCell::new(Vec::new()),
            current_text: RefCell::new(HtmlString::new()),
            exact_errors,
        }
    }

    /// Push one non-character token.
    fn push(&self, token: Token) {
        self.finish_text();
        self.tokens.borrow_mut().push(token);
    }

    /// Flush one pending character token.
    fn finish_text(&self) {
        if self.current_text.borrow().is_empty() {
            return;
        }

        let text = self.current_text.take();
        self.tokens.borrow_mut().push(CharacterTokens(text));
    }

    /// Return the collected tokenizer output.
    fn into_output(self) -> (Vec<Token>, Vec<TestError>) {
        self.finish_text();

        (self.tokens.into_inner(), self.errors.into_inner())
    }
}

impl LexHandler for TokenLogger {
    type Handle = ();

    /// Record one html5lib token output item.
    fn process_token(&self, token: Token, _line_number: u64) -> LexerAction<Self::Handle> {
        match token {
            CharacterTokens(text) => {
                self.current_text.borrow_mut().push_slice(&text);
            }

            NullCharacterToken => {
                self.current_text.borrow_mut().push_char('\0');
            }

            ParseError(_) => {
                if self.exact_errors {
                    self.errors.borrow_mut().push(TestError);
                }
            }

            TagToken(mut tag) => {
                match tag.kind {
                    EndTag => {
                        tag.self_closing = false;
                        tag.attrs = vec![];
                    }
                    _ => tag.attrs.sort_by(|left, right| left.name.cmp(&right.name)),
                }

                tag.had_duplicate_attributes = false;
                self.push(TagToken(tag));
            }

            EOFToken => {}

            _ => self.push(token),
        }

        LexerAction::Continue
    }
}

/// One JSON access helper for the html5lib corpora.
trait JsonValueExt {
    /// Return this value as one owned string.
    fn get_str(&self) -> String;

    /// Return this value as one HTML string buffer.
    fn get_html_string(&self) -> HtmlString;

    /// Return this value as one optional HTML string buffer.
    fn get_optional_html_string(&self) -> Option<HtmlString>;

    /// Return this value as one boolean.
    fn get_bool(&self) -> bool;

    /// Return this value as one object map.
    fn get_object(&self) -> &Map<String, Value>;

    /// Return this value as one list.
    fn get_list(&self) -> &[Value];

    /// Return one required object field.
    fn find(&self, key: &str) -> &Value;
}

impl JsonValueExt for Value {
    fn get_str(&self) -> String {
        match self {
            Value::String(string) => string.clone(),
            _ => panic!("expected JSON string"),
        }
    }

    fn get_html_string(&self) -> HtmlString {
        match self {
            Value::String(string) => string.as_str().to_html_string(),
            _ => panic!("expected JSON string"),
        }
    }

    fn get_optional_html_string(&self) -> Option<HtmlString> {
        match self {
            Value::Null => None,
            Value::String(string) => Some(string.as_str().to_html_string()),
            _ => panic!("expected JSON nullable string"),
        }
    }

    fn get_bool(&self) -> bool {
        match self {
            Value::Bool(value) => *value,
            _ => panic!("expected JSON bool"),
        }
    }

    fn get_object(&self) -> &Map<String, Value> {
        match self {
            Value::Object(object) => object,
            _ => panic!("expected JSON object"),
        }
    }

    fn get_list(&self) -> &[Value] {
        match self {
            Value::Array(list) => list.as_slice(),
            _ => panic!("expected JSON array"),
        }
    }

    fn find(&self, key: &str) -> &Value {
        self.get_object()
            .get(key)
            .unwrap_or_else(|| panic!("missing JSON key {key}"))
    }
}

/// Return the sorted files in one corpus directory.
fn collect_test_files(directory: &Path, extension: &str) -> Vec<PathBuf> {
    let mut files = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("failed to read entry: {error}"))
                .path()
        })
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some(extension))
        .collect::<Vec<_>>();

    files.sort();
    files
}

/// Decode one raw fixture line with one byte-preserving mapping.
fn decode_fixture_line(bytes: &[u8]) -> String {
    bytes
        .strip_suffix(b"\r")
        .unwrap_or(bytes)
        .iter()
        .map(|byte| char::from(*byte))
        .collect()
}

/// One raw html5lib encoding case.
struct EncodingCase {
    /// The raw input bytes.
    data: Vec<u8>,
    /// The expected encoding label.
    expected: String,
}

/// Finish one html5lib encoding section inside the current case.
fn finish_encoding_value(
    current_data: &mut Option<Vec<u8>>,
    current_encoding: &mut Option<String>,
    current_key: &mut Option<String>,
    current_value: &mut Vec<u8>,
) {
    let Some(key) = current_key.take() else {
        return;
    };

    let value = std::mem::take(current_value);

    if key == "data" {
        assert!(
            current_data.replace(value).is_none(),
            "duplicate #data section"
        );
    } else if key == "encoding" {
        let value = decode_fixture_line(&value);
        assert!(
            current_encoding.replace(value).is_none(),
            "duplicate #encoding section"
        );
    }
}

/// Finish one html5lib encoding case when both required fields are present.
fn finish_encoding_case(
    cases: &mut Vec<EncodingCase>,
    current_data: &mut Option<Vec<u8>>,
    current_encoding: &mut Option<String>,
) {
    let (Some(data), Some(expected)) = (current_data.take(), current_encoding.take()) else {
        return;
    };

    cases.push(EncodingCase { data, expected });
}

/// Parse one raw html5lib encoding corpus file.
fn parse_encoding_cases(bytes: &[u8]) -> Vec<EncodingCase> {
    let mut cases = Vec::new();
    let mut current_data = None;
    let mut current_encoding = None;
    let mut current_key = None;
    let mut current_value = Vec::new();

    // line scan
    for line in bytes.split(|byte| *byte == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);

        if let Some(rest) = line.strip_prefix(b"#") {
            finish_encoding_value(
                &mut current_data,
                &mut current_encoding,
                &mut current_key,
                &mut current_value,
            );

            if line == b"#data" {
                finish_encoding_case(&mut cases, &mut current_data, &mut current_encoding);
            }

            current_key = Some(String::from_utf8_lossy(rest).into_owned());
            continue;
        }

        current_value.extend_from_slice(line);
        current_value.push(b'\n');
    }

    finish_encoding_value(
        &mut current_data,
        &mut current_encoding,
        &mut current_key,
        &mut current_value,
    );
    finish_encoding_case(&mut cases, &mut current_data, &mut current_encoding);

    cases
}

/// Normalize one detected encoding label with HTML-specific adjustments.
fn normalize_html_encoding_label(label: &str) -> String {
    MetaEncodingScanner::canonical_label(label)
        .map(|label| label.to_string())
        .unwrap_or_else(|| "windows-1252".to_string())
}

/// Return one charset label from one meta content payload.
fn scan_meta_content_encoding(content: &str) -> Option<HtmlString> {
    let content = HtmlString::from_slice(content);
    MetaEncodingScanner::new_content(&content).scan_content_encoding()
}

/// Return the incremental input splits used by the upstream tokenizer harness.
fn splits(source: &str, count: usize) -> Vec<Vec<HtmlString>> {
    if count == 1 {
        return vec![vec![source.to_html_string()]];
    }

    let mut output = Vec::new();

    // every split point
    for boundary in source
        .char_indices()
        .map(|(index, _)| index)
        .chain(Some(source.len()))
    {
        let suffix = &source[boundary..];

        for mut prefix in splits(&source[..boundary], count - 1) {
            prefix.push(suffix.to_html_string());
            output.push(prefix);
        }
    }

    // shorter split counts
    output.extend(splits(source, count - 1));
    output.truncate(MAX_SPLITS);

    output
}

/// Tokenize one chunked input sequence with one option set.
fn tokenize_chunks(chunks: Vec<HtmlString>, options: LexerOptions) -> (Vec<Token>, Vec<TestError>) {
    let logger = TokenLogger::new(options.exact_errors);
    let lexer = Lexer::new(logger, options);
    let buffer = BufferQueue::default();

    // one-shot chunks
    for chunk in chunks {
        buffer.push_back(chunk);
    }

    // tokenization
    while lexer.feed(&buffer) != LexerResult::Done {}

    lexer.end();
    lexer.parser.into_output()
}

/// Tokenize one sequence of chunks while preserving line numbers.
fn tokenize_lines(input: Vec<HtmlString>, options: LexerOptions) -> Vec<(Token, u64)> {
    let matcher = LineMatcher::new();
    let lexer = Lexer::new(matcher, options);
    let buffer = BufferQueue::default();

    // one-shot chunks
    for chunk in input {
        buffer.push_back(chunk);
        let _ = lexer.feed(&buffer);
    }

    lexer.end();
    lexer.parser.lines.into_inner()
}

/// Create one tag token for expectations.
fn create_tag(name: HtmlString, kind: TagKind) -> Token {
    let name = LocalName::from(&*name);

    TagToken(Tag {
        kind,
        name,
        self_closing: false,
        attrs: vec![],
        had_duplicate_attributes: false,
    })
}

/// Lower one JSON token item into one lexer token.
fn json_to_token(token: &Value) -> Token {
    let parts = token.get_list();
    let args: Vec<&Value> = parts[1..].iter().collect();

    match parts[0].get_str().as_str() {
        "DOCTYPE" => DoctypeToken(Doctype {
            name: args[0].get_optional_html_string(),
            public_id: args[1].get_optional_html_string(),
            system_id: args[2].get_optional_html_string(),
            force_quirks: !args[3].get_bool(),
        }),

        "StartTag" => TagToken(Tag {
            kind: StartTag,
            name: LocalName::from(args[0].get_str().as_str()),
            attrs: args[1]
                .get_object()
                .iter()
                .map(|(name, value)| Attribute {
                    name: QualifiedName::new(None, ns!(), LocalName::from(name.as_str())),
                    value: value.get_html_string(),
                })
                .collect(),
            self_closing: match args.get(2) {
                Some(value) => value.get_bool(),
                None => false,
            },
            had_duplicate_attributes: false,
        }),

        "EndTag" => TagToken(Tag {
            kind: EndTag,
            name: LocalName::from(args[0].get_str().as_str()),
            attrs: vec![],
            self_closing: false,
            had_duplicate_attributes: false,
        }),

        "Comment" => CommentToken(args[0].get_html_string()),
        "Character" => CharacterTokens(args[0].get_html_string()),
        other => panic!("unexpected JSON token kind {other}"),
    }
}

/// Lower the expected JSON output into one normalized token stream.
fn json_to_tokens(
    tokens: &Value,
    errors: &[Value],
    exact_errors: bool,
) -> (Vec<Token>, Vec<TestError>) {
    let logger = TokenLogger::new(exact_errors);

    // expected tokens
    for token in tokens.get_list() {
        assert_eq!(
            logger.process_token(json_to_token(token), 0),
            LexerAction::Continue
        );
    }

    // expected errors
    for error in errors {
        assert_eq!(
            logger.process_token(ParseError(error.find("code").get_str().into()), 0),
            LexerAction::Continue
        );
    }

    logger.into_output()
}

/// Undo one double-escaped tokenizer input or output payload.
fn unescape(source: &str) -> Option<String> {
    let mut output = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();

    loop {
        let Some(character) = chars.next() else {
            return Some(output);
        };

        if character != '\\' {
            output.push(character);
            continue;
        }

        assert_eq!(chars.next(), Some('u'), "unexpected escaped sequence");
        let digits = chars.by_ref().take(4).collect::<String>();
        let codepoint = u32::from_str_radix(&digits, 16).ok();
        let character = codepoint.and_then(char::from_u32)?;
        output.push(character);
    }
}

/// Undo one double-escaped JSON subtree.
fn unescape_json(value: &Value) -> Value {
    match value {
        Value::String(string) => Value::String(
            unescape(string).unwrap_or_else(|| panic!("expected valid escaped output")),
        ),
        Value::Array(values) => Value::Array(values.iter().map(unescape_json).collect()),
        Value::Object(object) => {
            let mut next = Map::new();

            for (key, value) in object {
                next.insert(key.clone(), unescape_json(value));
            }

            Value::Object(next)
        }
        _ => value.clone(),
    }
}

/// Return the tokenizer modes requested by one html5lib test case.
fn initial_lexer_modes(test: &Value) -> Vec<Option<LexerMode>> {
    match test.get_object().get("initialStates") {
        Some(Value::Array(states)) => states
            .iter()
            .map(|state| {
                Some(match state.get_str().as_str() {
                    "Data state" => Data,
                    "PLAINTEXT state" => Plaintext,
                    "RAWTEXT state" => RawData(RawKind::Rawtext),
                    "RCDATA state" => RawData(RawKind::Rcdata),
                    "Script data state" => RawData(RawKind::ScriptData),
                    "CDATA section state" => LexerMode::CdataSection,
                    other => panic!("unexpected initial state {other}"),
                })
            })
            .collect(),
        None => vec![None],
        Some(_) => panic!("unexpected initialStates payload"),
    }
}

/// Run one html5lib tokenizer test case.
fn run_tokenizer_case(filename: &str, test: &Value) {
    let object = test.get_object();
    let mut input = test.find("input").get_str();
    let mut output = test.find("output").clone();
    let errors = object
        .get("errors")
        .map(JsonValueExt::get_list)
        .map(|errors| errors.to_vec())
        .unwrap_or_default();
    let start_tag_name = object.get("lastStartTag").map(JsonValueExt::get_str);

    // double-escaped cases
    if object
        .get("doubleEscaped")
        .is_some_and(JsonValueExt::get_bool)
    {
        let Some(unescaped_input) = unescape(&input) else {
            return;
        };

        input = unescaped_input;
        output = unescape_json(&output);
    }

    // mode and exact-error variants
    for initial_state in initial_lexer_modes(test) {
        for exact_errors in [false, true] {
            let options = LexerOptions {
                exact_errors,
                discard_bom: false,
                profile: false,
                initial_state,
                last_start_tag_name: start_tag_name.clone(),
            };
            let expected = json_to_tokens(&output, &errors, exact_errors);
            let description = test.find("description").get_str();

            for chunks in splits(&input, 3) {
                let actual = tokenize_chunks(chunks.clone(), options.clone());

                assert_eq!(
                    actual, expected,
                    "tokenizer mismatch in {filename}: {description}\ninput: {chunks:?}"
                );
            }
        }
    }
}

/// Run one tokenizer corpus directory.
fn run_tokenizer_directory(directory: &Path) {
    for path in collect_test_files(directory, "test") {
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_else(|| panic!("invalid tokenizer test filename"));
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let value: Value = serde_json::from_str(&source)
            .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()));

        let Some(tests) = value.get_object().get("tests") else {
            continue;
        };

        for test in tests.get_list() {
            run_tokenizer_case(file_name, test);
        }
    }
}

#[test]
fn test_html5lib_tokenizer_corpus() {
    let tokenizer_directory = html_test_directory(HTML5LIB_TOKENIZER_DIRECTORY);
    let custom_directory = html_test_directory(CUSTOM_TOKENIZER_DIRECTORY);

    run_tokenizer_directory(&tokenizer_directory);
    run_tokenizer_directory(&custom_directory);
}

#[test]
fn test_buffer_smoke() {
    let buffer = BufferQueue::default();

    assert_eq!(buffer.peek(), None);
    assert_eq!(buffer.next(), None);

    buffer.push_back("abc".to_html_string());

    assert_eq!(buffer.peek(), Some('a'));
    assert_eq!(buffer.next(), Some('a'));
    assert_eq!(buffer.peek(), Some('b'));
    assert_eq!(buffer.peek(), Some('b'));
    assert_eq!(buffer.next(), Some('b'));
    assert_eq!(buffer.peek(), Some('c'));
    assert_eq!(buffer.next(), Some('c'));
    assert_eq!(buffer.peek(), None);
    assert_eq!(buffer.next(), None);
}

#[test]
fn test_buffer_unconsume() {
    let buffer = BufferQueue::default();

    buffer.push_back("abc".to_html_string());
    assert_eq!(buffer.next(), Some('a'));

    buffer.push_front("xy".to_html_string());

    assert_eq!(buffer.next(), Some('x'));
    assert_eq!(buffer.next(), Some('y'));
    assert_eq!(buffer.next(), Some('b'));
    assert_eq!(buffer.next(), Some('c'));
    assert_eq!(buffer.next(), None);
}

#[test]
fn test_buffer_pop_except_from() {
    let buffer = BufferQueue::default();
    buffer.push_back("abc&def".to_html_string());

    assert_eq!(
        buffer.pop_except_from(small_char_set!('&')),
        Some(SetResult::NotFromSet("abc".to_html_string()))
    );
    assert_eq!(
        buffer.pop_except_from(small_char_set!('&')),
        Some(SetResult::FromSet('&'))
    );
    assert_eq!(
        buffer.pop_except_from(small_char_set!('&')),
        Some(SetResult::NotFromSet("def".to_html_string()))
    );
    assert_eq!(buffer.pop_except_from(small_char_set!('&')), None);
}

#[test]
fn test_buffer_eat() {
    let buffer = BufferQueue::default();

    buffer.push_back("a".to_html_string());
    buffer.push_back("bc".to_html_string());

    assert_eq!(buffer.eat("abcd", u8::eq_ignore_ascii_case), None);
    assert_eq!(buffer.eat("ax", u8::eq_ignore_ascii_case), Some(false));
    assert_eq!(buffer.eat("ab", u8::eq_ignore_ascii_case), Some(true));
    assert_eq!(buffer.next(), Some('c'));
    assert_eq!(buffer.next(), None);
}

#[test]
fn test_charset_nonmember_prefix() {
    for character in ['&', '\0'] {
        for prefix_length in 0..48u32 {
            for suffix_length in 0..48u32 {
                let mut string = "x".repeat(prefix_length as usize);
                string.push(character);
                string.push_str(&"x".repeat(suffix_length as usize));

                let set = small_char_set!('&' '\0');

                assert_eq!(prefix_length, set.nonmember_prefix_len(&string));
            }
        }
    }
}

#[test]
fn test_extract_meta_encoding_without_charset() {
    assert_eq!(scan_meta_content_encoding("foobar"), None);
}

#[test]
fn test_extract_meta_encoding_with_capitalized_charset() {
    assert_eq!(
        scan_meta_content_encoding("cHarSet=utf8"),
        Some(HtmlString::from_slice("utf8"))
    );
}

#[test]
fn test_extract_meta_encoding_with_no_equals() {
    assert_eq!(scan_meta_content_encoding("charset utf8"), None);
}

#[test]
fn test_extract_meta_encoding_with_whitespace() {
    assert_eq!(
        scan_meta_content_encoding("charset \t=\tutf8"),
        Some(HtmlString::from_slice("utf8"))
    );
}

#[test]
fn test_extract_meta_encoding_with_quotes() {
    assert_eq!(
        scan_meta_content_encoding("charset='utf8'"),
        Some(HtmlString::from_slice("utf8"))
    );
    assert_eq!(
        scan_meta_content_encoding("charset=\"utf8\""),
        Some(HtmlString::from_slice("utf8"))
    );
    assert_eq!(scan_meta_content_encoding("charset='utf8"), None);
    assert_eq!(scan_meta_content_encoding("charset=\"utf8"), None);
}

#[test]
fn test_extract_meta_encoding_with_implicit_terminator() {
    assert_eq!(
        scan_meta_content_encoding("charset=utf8 foo"),
        Some(HtmlString::from_slice("utf8"))
    );
    assert_eq!(
        scan_meta_content_encoding("charset=utf8;foo"),
        Some(HtmlString::from_slice("utf8"))
    );
}

#[test]
fn test_extract_meta_encoding_from_content_type() {
    assert_eq!(
        scan_meta_content_encoding("text/html; charset=utf8"),
        Some(HtmlString::from_slice("utf8"))
    );
}

#[test]
fn test_html5lib_encoding_corpus() {
    let directory = html_test_directory(HTML5LIB_ENCODING_DIRECTORY);

    for path in collect_test_files(&directory, "dat") {
        let bytes = fs::read(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let cases = parse_encoding_cases(&bytes);

        for (index, case) in cases.into_iter().enumerate() {
            let mut data = case.data;
            let expected = case.expected.trim_end_matches('\n').to_ascii_lowercase();

            data.pop();

            let actual = EncodingScanner::new(&data)
                .scan_document_encoding()
                .map(|label| normalize_html_encoding_label(label.as_ref()))
                .unwrap_or_else(|| "windows-1252".to_string());
            let input = decode_fixture_line(&data);

            assert_eq!(
                actual,
                expected,
                "encoding mismatch in {} case {}\ninput:\n{}\n",
                path.display(),
                index,
                input,
            );
        }
    }
}

#[test]
fn test_lower_ascii_letter() {
    assert_eq!(lower_ascii_letter('a'), Some('a'));
    assert_eq!(lower_ascii_letter('A'), Some('a'));
    assert_eq!(lower_ascii_letter('!'), None);
    assert_eq!(lower_ascii_letter('\u{a66e}'), None);
}

#[test]
fn test_option_push_into_none() {
    let mut string = None;

    push_optional_string(&mut string, 'x');

    assert_eq!(string, Some("x".to_html_string()));
}

#[test]
fn test_option_push_into_empty() {
    let mut string = Some(HtmlString::new());

    push_optional_string(&mut string, 'x');

    assert_eq!(string, Some("x".to_html_string()));
}

#[test]
fn test_option_push_into_nonempty() {
    let mut string = Some(HtmlString::from_slice("y"));

    push_optional_string(&mut string, 'x');

    assert_eq!(string, Some("yx".to_html_string()));
}

#[test]
fn test_lexer_tracks_line_numbers_with_lf() {
    let options = LexerOptions {
        exact_errors: false,
        discard_bom: true,
        profile: false,
        initial_state: None,
        last_start_tag_name: None,
    };
    let input = vec![
        HtmlString::from("<a>\n"),
        HtmlString::from("<b>\n"),
        HtmlString::from("</b>\n"),
        HtmlString::from("</a>\n"),
    ];
    let expected = vec![
        (create_tag(HtmlString::from("a"), StartTag), 1),
        (CharacterTokens(HtmlString::from("\n")), 2),
        (create_tag(HtmlString::from("b"), StartTag), 2),
        (CharacterTokens(HtmlString::from("\n")), 3),
        (create_tag(HtmlString::from("b"), EndTag), 3),
        (CharacterTokens(HtmlString::from("\n")), 4),
        (create_tag(HtmlString::from("a"), EndTag), 4),
    ];

    let actual = tokenize_lines(input, options);

    assert_eq!(actual, expected);
}

#[test]
fn test_lexer_tracks_line_numbers_with_crlf() {
    let options = LexerOptions {
        exact_errors: false,
        discard_bom: true,
        profile: false,
        initial_state: None,
        last_start_tag_name: None,
    };
    let input = vec![
        HtmlString::from("<a>\r\n"),
        HtmlString::from("<b>\r\n"),
        HtmlString::from("</b>\r\n"),
        HtmlString::from("</a>\r\n"),
    ];
    let expected = vec![
        (create_tag(HtmlString::from("a"), StartTag), 1),
        (CharacterTokens(HtmlString::from("\n")), 2),
        (create_tag(HtmlString::from("b"), StartTag), 2),
        (CharacterTokens(HtmlString::from("\n")), 3),
        (create_tag(HtmlString::from("b"), EndTag), 3),
        (CharacterTokens(HtmlString::from("\n")), 4),
        (create_tag(HtmlString::from("a"), EndTag), 4),
    ];

    let actual = tokenize_lines(input, options);

    assert_eq!(actual, expected);
}
