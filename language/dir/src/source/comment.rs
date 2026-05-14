use std::borrow::Cow;
use std::fmt::Debug;

use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::TokenType;

/// Indicates a line or block comment.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommentKind {
    /// Line comment.
    Line,
    /// Single-line block comment.
    SingleLineBlock,
    /// Multi-line block comment.
    MultiLineBlock,
}

/// Structured content classification for one comment.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CommentContent {
    /// No structured content classification.
    #[default]
    None,
    /// A legal or preserved comment.
    Legal,
    /// A jsdoc-style comment.
    Jsdoc,
    /// A jsdoc comment with legal or preserve semantics.
    JsdocLegal,
}

/// A comment's position relative to a token boundary.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CommentPosition {
    /// The comment belongs to the following token boundary.
    #[default]
    Leading,
    /// The comment trails the previous token boundary.
    Trailing,
}

/// Newline shape flags captured around one raw comment.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CommentNewlines {
    /// Bit flags that describe newline boundaries.
    pub bits: u8,
}

impl CommentNewlines {
    /// Leading newline bit.
    pub const LEADING: u8 = 1 << 0;
    /// Trailing newline bit.
    pub const TRAILING: u8 = 1 << 1;

    /// Create flags from booleans.
    #[inline]
    pub fn from_bools(has_leading_newline: bool, has_trailing_newline: bool) -> Self {
        let mut bits = 0u8;

        if has_leading_newline {
            bits |= Self::LEADING;
        }
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

/// A raw source comment attached through the source side table.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    /// The span of the raw comment, including delimiters.
    pub span: Span,
    /// The token boundary this leading comment is attached to.
    pub attached_to: u32,
    /// The kind of the comment.
    pub kind: CommentKind,
    /// The token-relative comment position.
    pub position: CommentPosition,
    /// The newline shape around the comment.
    pub newlines: CommentNewlines,
    /// The structured comment content classification.
    pub content: CommentContent,
}

impl Comment {
    /// Create a comment with one token-relative position.
    #[inline]
    pub fn new(span: Span, kind: CommentKind) -> Self {
        Self {
            span,
            attached_to: 0,
            kind,
            position: CommentPosition::Trailing,
            newlines: CommentNewlines::default(),
            content: CommentContent::None,
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

    /// Return whether this comment is before a token boundary.
    #[inline]
    pub fn is_leading(self) -> bool {
        self.position == CommentPosition::Leading
    }

    /// Return whether this comment is after a token boundary.
    #[inline]
    pub fn is_trailing(self) -> bool {
        self.position == CommentPosition::Trailing
    }

    /// Return whether this comment is classified as jsdoc.
    #[inline]
    pub fn is_jsdoc(self) -> bool {
        matches!(
            self.content,
            CommentContent::Jsdoc | CommentContent::JsdocLegal
        )
    }

    /// Return whether this comment has no special meaning.
    #[inline]
    pub fn is_normal(self) -> bool {
        self.content == CommentContent::None
    }

    /// Return whether this comment carries an annotation.
    #[inline]
    pub fn is_annotation(self) -> bool {
        !self.is_normal()
    }

    /// Return whether this comment is classified as legal.
    #[inline]
    pub fn is_legal(self) -> bool {
        matches!(
            self.content,
            CommentContent::Legal | CommentContent::JsdocLegal
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

/// Normalize one comment payload from raw source text.
pub fn normalize_comment_payload<'a>(raw: &'a str) -> Cow<'a, str> {
    let token_type = if raw.starts_with("///") {
        Some(TokenType::DocLineComment)
    } else if raw.starts_with("//") {
        Some(TokenType::LineComment)
    } else if raw.starts_with("/**") {
        Some(TokenType::DocBlockComment)
    } else if raw.starts_with("/*") {
        Some(TokenType::BlockComment)
    } else {
        None
    };

    let mut inner = match token_type {
        Some(TokenType::LineComment) => raw.strip_prefix("//").unwrap_or(raw),
        Some(TokenType::DocLineComment) => raw.strip_prefix("///").unwrap_or(raw),
        Some(TokenType::BlockComment) => raw
            .strip_prefix("/*")
            .unwrap_or(raw)
            .strip_suffix("*/")
            .unwrap_or(raw),
        Some(TokenType::DocBlockComment) => raw
            .strip_prefix("/**")
            .unwrap_or(raw)
            .strip_suffix("*/")
            .unwrap_or(raw),
        _ => raw,
    };

    if matches!(
        token_type,
        Some(TokenType::LineComment | TokenType::DocLineComment)
    ) && inner.starts_with(' ')
    {
        inner = &inner[1..];
    }

    if matches!(
        token_type,
        Some(TokenType::BlockComment | TokenType::DocBlockComment)
    ) && inner.contains('\n')
    {
        let has_trailing_newline = inner.ends_with('\n');
        let mut cleaned = inner
            .lines()
            .map(|line| {
                let line = line.trim_end();
                let line = line.trim_start();
                let line = line.strip_prefix('*').unwrap_or(line);
                line.strip_prefix(' ').unwrap_or(line)
            })
            .collect::<Vec<_>>()
            .join("\n");

        if has_trailing_newline {
            cleaned.push('\n');
        }

        return Cow::Owned(cleaned);
    }

    let trimmed = inner.trim_end();
    if std::ptr::eq(trimmed.as_ptr(), inner.as_ptr()) && trimmed.len() == inner.len() {
        Cow::Borrowed(inner)
    } else {
        Cow::Owned(trimmed.to_string())
    }
}
