use std::io::{ErrorKind, Read, Result};

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
enum State {
    Top,
    InString,
    StringEscape,
    InComment,
    InBlockComment,
    MaybeCommentEnd,
    InLineComment,
}

use State::*;

/// A [`Read`] that transforms another [`Read`] so that it changes all comments to spaces so that a downstream json parser
/// (such as json-serde) doesn't choke on them.
///
/// The supported comments are:
///   - C style block comments (`/* ... */`)
///   - C style line comments (`// ...`)
///   - Shell style line comments (`# ...`)
///
/// ## Example
/// ```
/// use json_comments::StripComments;
/// use std::io::Read;
///
/// let input = r#"{
/// // c line comment
/// "a": "comment in string /* a */",
/// ## shell line comment
/// } /** end */"#;
///
/// let mut stripped = String::new();
/// StripComments::new(input.as_bytes()).read_to_string(&mut stripped).unwrap();
///
/// assert_eq!(stripped, "{
///                  \n\"a\": \"comment in string /* a */\",
///                     \n}           ");
///
/// ```
///
#[derive(Debug)]
pub struct StripComments<T: Read> {
    inner: T,
    state: State,
    settings: StripJsonOptions,
}

impl<T> StripComments<T>
where
    T: Read,
{
    pub fn new(input: T) -> Self {
        Self {
            inner: input,
            state: Top,
            settings: StripJsonOptions::default(),
        }
    }

    /// Create a new `StripComments` with settings which may be different from the default.
    ///
    /// This is useful if you wish to disable allowing certain kinds of comments.
    #[inline]
    pub fn with_settings(settings: StripJsonOptions, input: T) -> Self {
        Self {
            inner: input,
            state: Top,
            settings,
        }
    }
}

macro_rules! invalid_data {
    () => {
        return Err(ErrorKind::InvalidData.into())
    };
}

impl<T> Read for StripComments<T>
where
    T: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let count = self.inner.read(buf)?;
        if count > 0 {
            for c in buf[..count].iter_mut() {
                self.state = match self.state {
                    Top => top(c, &self.settings),
                    InString => in_string(*c),
                    StringEscape => InString,
                    InComment => in_comment(c, &self.settings)?,
                    InBlockComment => in_block_comment(c),
                    MaybeCommentEnd => maybe_comment_end(c),
                    InLineComment => in_line_comment(c),
                }
            }
        } else if self.state != Top && self.state != InLineComment {
            invalid_data!();
        }
        Ok(count)
    }
}

/// Settings for `StripComments`
///
/// The default is for all comment types to be enabled.
#[derive(Copy, Clone, Debug)]
pub struct StripJsonOptions {
    /// True if c-style block comments (`/* ... */`) are allowed
    block_comments: bool,
    /// True if c-style `//` line comments are allowed
    slash_line_comments: bool,
    /// True if shell-style `#` line comments are allowed
    hash_line_comments: bool,
}

impl Default for StripJsonOptions {
    fn default() -> Self {
        Self::all()
    }
}

impl StripJsonOptions {
    /// Enable all comment Styles
    pub const fn all() -> Self {
        Self {
            block_comments: true,
            slash_line_comments: true,
            hash_line_comments: true,
        }
    }
    /// Only allow line comments starting with `#`
    pub const fn hash_only() -> Self {
        Self {
            hash_line_comments: true,
            block_comments: false,
            slash_line_comments: false,
        }
    }
    /// Only allow "c-style" comments.
    ///
    /// Specifically, line comments beginning with `//` and
    /// block comment like `/* ... */`.
    pub const fn c_style() -> Self {
        Self {
            block_comments: true,
            slash_line_comments: true,
            hash_line_comments: false,
        }
    }

    /// Create a new `StripComments` for `input`, using these settings.
    ///
    /// Transform `input` into a [`Read`] that strips out comments.
    /// The types of comments to support are determined by the configuration of
    /// `self`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use json_comments::CommentSettings;
    /// use std::io::Read;
    ///
    /// let input = r#"{
    /// // c line comment
    /// "a": "b"
    /// /** multi line
    /// comment
    /// */ }"#;
    ///
    /// let mut stripped = String::new();
    /// CommentSettings::c_style().strip_comments(input.as_bytes()).read_to_string(&mut stripped).unwrap();
    ///
    /// assert_eq!(stripped, "{
    ///                  \n\"a\": \"b\"
    ///                           }");
    /// ```
    ///
    /// ```
    /// use json_comments::CommentSettings;
    /// use std::io::Read;
    ///
    /// let input = r#"{
    /// ## shell line comment
    /// "a": "b"
    /// }"#;
    ///
    /// let mut stripped = String::new();
    /// CommentSettings::hash_only().strip_comments(input.as_bytes()).read_to_string(&mut stripped).unwrap();
    ///
    /// assert_eq!(stripped, "{
    ///                     \n\"a\": \"b\"\n}");
    /// ```
    #[inline]
    pub fn strip<I: Read>(self, input: I) -> StripComments<I> {
        StripComments::with_settings(self, input)
    }
}

/// Strip JSON comments into a new string using the default settings.
pub fn strip_json(input: &str) -> Result<String> {
    let mut stripped = String::with_capacity(input.len());
    StripJsonOptions::default()
        .strip(input.as_bytes())
        .read_to_string(&mut stripped)?;
    Ok(stripped)
}

fn top(c: &mut u8, settings: &StripJsonOptions) -> State {
    match *c {
        b'"' => InString,
        b'/' => {
            *c = b' ';
            InComment
        }
        b'#' if settings.hash_line_comments => {
            *c = b' ';
            InLineComment
        }
        _ => Top,
    }
}

fn in_string(c: u8) -> State {
    match c {
        b'"' => Top,
        b'\\' => StringEscape,
        _ => InString,
    }
}

fn in_comment(c: &mut u8, settings: &StripJsonOptions) -> Result<State> {
    let new_state = match c {
        b'*' if settings.block_comments => InBlockComment,
        b'/' if settings.slash_line_comments => InLineComment,
        _ => invalid_data!(),
    };
    *c = b' ';
    Ok(new_state)
}

fn in_block_comment(c: &mut u8) -> State {
    let old = *c;
    *c = b' ';
    if old == b'*' {
        MaybeCommentEnd
    } else {
        InBlockComment
    }
}

fn maybe_comment_end(c: &mut u8) -> State {
    let old = *c;
    *c = b' ';
    match old {
        b'/' => Top,
        b'*' => MaybeCommentEnd,
        _ => InBlockComment,
    }
}

fn in_line_comment(c: &mut u8) -> State {
    if *c == b'\n' {
        Top
    } else {
        *c = b' ';
        InLineComment
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{ErrorKind, Read};

    fn strip_string(input: &str) -> String {
        let mut out = String::new();
        let count = StripComments::new(input.as_bytes())
            .read_to_string(&mut out)
            .unwrap();
        assert_eq!(count, input.len());
        out
    }

    #[test]
    fn block_comments() {
        let json = r#"{/* Comment */"hi": /** abc */ "bye"}"#;
        let stripped = strip_string(json);
        assert_eq!(stripped, r#"{             "hi":            "bye"}"#);
    }

    #[test]
    fn block_comments_with_possible_end() {
        let json = r#"{/* Comment*PossibleEnd */"hi": /** abc */ "bye"}"#;
        let stripped = strip_string(json);
        assert_eq!(
            stripped,
            r#"{                         "hi":            "bye"}"#
        );
    }

    // See https://github.com/tmccombs/json-comments-rs/issues/12
    // Make sure we can parse a block comment that ends with more than one "*"
    #[test]
    fn doc_comment() {
        let json = r##"/** C **/ { "foo": 123 }"##;
        let stripped = strip_string(json);
        assert_eq!(stripped, r##"          { "foo": 123 }"##);
    }

    #[test]
    fn line_comments() {
        let json = r#"{
            // line comment
            "a": 4,
            # another
        }"#;

        let expected = "{
                           \n            \"a\": 4,
                     \n        }";

        assert_eq!(strip_string(json), expected);
    }

    #[test]
    fn incomplete_string() {
        let json = r#""foo"#;
        let mut stripped = String::new();

        let err = StripComments::new(json.as_bytes())
            .read_to_string(&mut stripped)
            .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
    }

    #[test]
    fn incomplete_comment() {
        let json = r#"/* foo "#;
        let mut stripped = String::new();

        let err = StripComments::new(json.as_bytes())
            .read_to_string(&mut stripped)
            .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
    }

    #[test]
    fn incomplete_comment2() {
        let json = r#"/* foo *"#;
        let mut stripped = String::new();

        let err = StripComments::new(json.as_bytes())
            .read_to_string(&mut stripped)
            .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
    }

    #[test]
    fn no_hash_comments() {
        let json = r#"# bad comment
        {"a": "b"}"#;
        let mut stripped = String::new();
        StripJsonOptions::c_style()
            .strip(json.as_bytes())
            .read_to_string(&mut stripped)
            .unwrap();
        assert_eq!(stripped, json);
    }

    #[test]
    fn no_slash_line_comments() {
        let json = r#"// bad comment
        {"a": "b"}"#;
        let mut stripped = String::new();
        let err = StripJsonOptions::hash_only()
            .strip(json.as_bytes())
            .read_to_string(&mut stripped)
            .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
    }

    #[test]
    fn no_block_comments() {
        let json = r#"/* bad comment */ {"a": "b"}"#;
        let mut stripped = String::new();
        let err = StripJsonOptions::hash_only()
            .strip(json.as_bytes())
            .read_to_string(&mut stripped)
            .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
    }
}
