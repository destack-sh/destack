use std::borrow::Cow;
use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;
use tspp_source::Span;

/// Indicates a line or block comment.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum CommentKind {
    /// Line comment.
    Line,
    /// Single-line block comment.
    SingleLineBlock,
    /// Multi-line block comment.
    MultiLineBlock,
}

/// The authored intent of one comment.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Default, Reflect)]
pub enum CommentRole {
    /// An ordinary source comment.
    #[default]
    Ordinary,
    /// An authored documentation comment.
    Documentation,
    /// A legal or preserved comment.
    Legal,
    /// Authored documentation with legal or preserve semantics.
    LegalDocumentation,
}

/// A comment's attachment to the semantic token stream.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum CommentAnchor {
    /// The comment belongs before the token at this byte offset.
    Before(u32),
    /// The comment belongs after the token at this byte offset.
    After(u32),
    /// The comment belongs at the end of the source file.
    End,
}

/// Newline flags captured around one source comment.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Default, Reflect)]
pub struct CommentNewlines {
    /// Bit flags that describe newline boundaries.
    bits: u8,
}

impl CommentNewlines {
    /// Leading newline bit.
    const LEADING: u8 = 1 << 0;
    /// Trailing newline bit.
    const TRAILING: u8 = 1 << 1;

    /// Create flags from booleans.
    #[inline]
    pub fn from_bools(has_leading_newline: bool, has_trailing_newline: bool) -> Self {
        let mut bits = 0u8;

        // leading boundary
        if has_leading_newline {
            bits |= Self::LEADING;
        }

        // trailing boundary
        if has_trailing_newline {
            bits |= Self::TRAILING;
        }

        Self { bits }
    }

    /// Return whether a leading newline exists.
    #[inline]
    pub fn has_leading_newline(self) -> bool {
        self.bits & Self::LEADING != 0
    }

    /// Return whether a trailing newline exists.
    #[inline]
    pub fn has_trailing_newline(self) -> bool {
        self.bits & Self::TRAILING != 0
    }
}

/// One retained source comment.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Comment {
    /// The source span of the raw comment, including delimiters.
    pub span: Span,
    /// The semantic token attachment.
    pub anchor: CommentAnchor,
    /// The kind of the comment.
    pub kind: CommentKind,
    /// The newlines around the comment.
    pub newlines: CommentNewlines,
    /// The authored role of the comment.
    pub role: CommentRole,
}

impl Comment {
    /// Return comment text without its source markers.
    pub fn text<'a>(self, source: &'a str) -> Cow<'a, str> {
        let raw = &source[self.span.start as usize..self.span.end as usize];
        let marker_width = 2 + usize::from(self.is_documentation());
        let closing_width = usize::from(self.is_block()) * 2;
        let content = &raw[marker_width..raw.len() - closing_width];

        match self.kind {
            // trim the conventional space after a line marker
            CommentKind::Line => {
                let content = content.strip_prefix(' ').unwrap_or(content);

                Cow::Borrowed(content.trim_end())
            }

            // preserve leading content inside single line block comments
            CommentKind::SingleLineBlock => Cow::Borrowed(content.trim_end()),

            // remove conventional multiline block leaders
            CommentKind::MultiLineBlock => {
                let mut text = String::with_capacity(content.len());
                for (index, line) in content.lines().enumerate() {
                    if index > 0 {
                        text.push('\n');
                    }

                    let line = line.trim_end().trim_start();
                    let line = line.strip_prefix('*').unwrap_or(line);
                    let line = line.strip_prefix(' ').unwrap_or(line);
                    text.push_str(line);
                }

                if content.ends_with('\n') {
                    text.push('\n');
                }

                Cow::Owned(text)
            }
        }
    }

    /// Return the content span inside the comment delimiters.
    #[inline]
    pub fn content_span(self) -> Span {
        match self.kind {
            CommentKind::Line => Span::new(self.span.file, self.span.start + 2, self.span.end),
            CommentKind::SingleLineBlock | CommentKind::MultiLineBlock => {
                Span::new(self.span.file, self.span.start + 2, self.span.end - 2)
            }
        }
    }

    /// Return whether this is a line comment.
    #[inline]
    pub fn is_line(self) -> bool {
        self.kind == CommentKind::Line
    }

    /// Return whether this is a block comment.
    #[inline]
    pub fn is_block(self) -> bool {
        matches!(
            self.kind,
            CommentKind::SingleLineBlock | CommentKind::MultiLineBlock
        )
    }

    /// Return whether this is a multiline block comment.
    #[inline]
    pub fn is_multiline_block(self) -> bool {
        self.kind == CommentKind::MultiLineBlock
    }

    /// Return whether this comment belongs before a token.
    #[inline]
    pub fn is_leading(self) -> bool {
        matches!(self.anchor, CommentAnchor::Before(_))
    }

    /// Return whether this comment belongs after a token.
    #[inline]
    pub fn is_trailing(self) -> bool {
        matches!(self.anchor, CommentAnchor::After(_))
    }

    /// Return the following token offset for a leading comment.
    #[inline]
    pub fn following_token_start(self) -> Option<u32> {
        match self.anchor {
            CommentAnchor::Before(offset) => Some(offset),
            CommentAnchor::After(_) | CommentAnchor::End => None,
        }
    }

    /// Return whether this comment contains documentation.
    #[inline]
    pub fn is_documentation(self) -> bool {
        matches!(
            self.role,
            CommentRole::Documentation | CommentRole::LegalDocumentation
        )
    }

    /// Return whether this comment is classified as legal.
    #[inline]
    pub fn is_legal(self) -> bool {
        matches!(
            self.role,
            CommentRole::Legal | CommentRole::LegalDocumentation
        )
    }

    /// Return whether this comment is preceded by a newline.
    #[inline]
    pub fn preceded_by_newline(self) -> bool {
        self.newlines.has_leading_newline()
    }

    /// Return whether this comment is followed by a newline.
    #[inline]
    pub fn followed_by_newline(self) -> bool {
        self.newlines.has_trailing_newline()
    }

    /// Set whether this comment is preceded by a newline.
    #[inline]
    pub fn set_preceded_by_newline(&mut self, preceded_by_newline: bool) {
        if preceded_by_newline {
            self.newlines.bits |= CommentNewlines::LEADING;
        } else {
            self.newlines.bits &= !CommentNewlines::LEADING;
        }
    }

    /// Set whether this comment is followed by a newline.
    #[inline]
    pub fn set_followed_by_newline(&mut self, followed_by_newline: bool) {
        if followed_by_newline {
            self.newlines.bits |= CommentNewlines::TRAILING;
        } else {
            self.newlines.bits &= !CommentNewlines::TRAILING;
        }
    }
}
