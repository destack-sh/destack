use std::borrow::Cow;
use std::fmt;
use std::ops::{Deref, Range};

/// One local HTML string buffer.
#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct HtmlString(String);

/// One marker trait for local string slicing helpers.
#[cfg(test)]
pub(crate) trait ToHtmlString {
    /// Convert this value into one local HTML string buffer.
    fn to_html_string(&self) -> HtmlString;
}

#[cfg(test)]
impl ToHtmlString for HtmlString {
    /// Convert this value into one local HTML string buffer.
    fn to_html_string(&self) -> HtmlString {
        self.clone()
    }
}

#[cfg(test)]
impl ToHtmlString for str {
    /// Convert this value into one local HTML string buffer.
    fn to_html_string(&self) -> HtmlString {
        HtmlString::from_slice(self)
    }
}

#[cfg(test)]
impl ToHtmlString for String {
    /// Convert this value into one local HTML string buffer.
    fn to_html_string(&self) -> HtmlString {
        HtmlString::from_slice(self)
    }
}

impl HtmlString {
    /// Create one empty string buffer.
    pub(crate) fn new() -> Self {
        Self(String::new())
    }

    /// Create one string buffer from one string slice.
    pub(crate) fn from_slice(value: &str) -> Self {
        Self(value.to_string())
    }

    /// Create one string buffer from one character.
    pub(crate) fn from_char(value: char) -> Self {
        let mut string = String::new();

        // single character
        string.push(value);

        Self(string)
    }

    /// Return whether this buffer is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Return the byte length as `u32`.
    pub(crate) fn len32(&self) -> u32 {
        self.0.len() as u32
    }

    /// Push one character.
    pub(crate) fn push_char(&mut self, value: char) {
        self.0.push(value);
    }

    /// Push one string slice.
    pub(crate) fn push_slice(&mut self, value: &str) {
        self.0.push_str(value);
    }

    /// Push one whole string buffer.
    pub(crate) fn push_html_string<T>(&mut self, value: T)
    where
        T: AsRef<str>,
    {
        self.0.push_str(value.as_ref());
    }

    /// Clear this buffer.
    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }

    /// Return one owned substring.
    pub(crate) fn substring(&self, start: u32, length: u32) -> Self {
        let range = slice_range(&self.0, start, length);

        Self(self.0[range].to_string())
    }

    /// Return one owned substring without additional bounds checks.
    ///
    /// The caller must pass one valid UTF 8 byte range.
    pub(crate) unsafe fn unsafe_substring(&self, start: u32, length: u32) -> Self {
        self.substring(start, length)
    }

    /// Pop one byte prefix without additional bounds checks.
    ///
    /// The caller must pass one valid UTF 8 byte length.
    pub(crate) unsafe fn unsafe_pop_front(&mut self, length: u32) {
        self.pop_front(length);
    }

    /// Pop one byte prefix.
    pub(crate) fn pop_front(&mut self, length: u32) {
        let range = slice_range(&self.0, length, self.0.len() as u32 - length);

        self.0 = self.0[range].to_string();
    }

    /// Pop one leading character.
    pub(crate) fn pop_front_char(&mut self) -> Option<char> {
        let mut chars = self.0.chars();
        let character = chars.next()?;
        let next_index = character.len_utf8();

        // leading scalar
        self.0.drain(..next_index);

        Some(character)
    }

    /// Pop one leading character run.
    pub(crate) fn pop_front_char_run(
        &mut self,
        mut predicate: impl FnMut(char) -> bool,
    ) -> Option<(Self, bool)> {
        let first_character = self.0.chars().next()?;
        let is_matching = predicate(first_character);

        // leading run
        let length = self
            .0
            .chars()
            .take_while(|character| predicate(*character) == is_matching)
            .map(char::len_utf8)
            .sum::<usize>();

        let prefix = self.0[..length].to_string();

        // consume run
        self.0.drain(..length);

        Some((Self(prefix), is_matching))
    }
}

impl fmt::Debug for HtmlString {
    /// Format this string buffer for debugging.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, formatter)
    }
}

impl fmt::Display for HtmlString {
    /// Format this string buffer for display.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Deref for HtmlString {
    type Target = str;

    /// Return the underlying string slice.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for HtmlString {
    /// Return the underlying string slice.
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for HtmlString {
    /// Convert one owned string into one HTML string buffer.
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for HtmlString {
    /// Convert one string slice into one HTML string buffer.
    fn from(value: &str) -> Self {
        Self::from_slice(value)
    }
}

impl From<Cow<'_, str>> for HtmlString {
    /// Convert one borrowed or owned string into one HTML string buffer.
    fn from(value: Cow<'_, str>) -> Self {
        Self(value.into_owned())
    }
}

/// Return one validated byte range for one string slice.
fn slice_range(source: &str, start: u32, length: u32) -> Range<usize> {
    let start = start as usize;
    let end = start + length as usize;

    debug_assert!(source.is_char_boundary(start));
    debug_assert!(source.is_char_boundary(end));

    start..end
}
