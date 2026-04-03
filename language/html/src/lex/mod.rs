mod macros;

#[allow(dead_code, unreachable_pub)]
pub(crate) mod atom;
mod attribute;
pub(crate) mod buffer;
mod charset;
mod content;
mod emit;
mod encoding;
mod entity;
mod input;
pub(crate) mod lexer;
mod markup;
mod name;
mod reference;
mod step;
mod string;
mod tag;
pub(crate) mod text;
pub(crate) mod token;

pub(crate) use atom::{
    LocalName, Namespace, Prefix, local_name, namespace_prefix, namespace_url, ns,
};
pub(crate) use charset::SmallCharSet;
#[cfg(test)]
pub(crate) use encoding::EncodingScanner;
pub(crate) use encoding::MetaEncodingScanner;
pub(crate) use lexer::{Lexer, LexerOptions, RawKind};
#[cfg(test)]
pub(crate) use macros::small_char_set;
pub(crate) use name::{Attribute, ExpandedName, QualifiedName, expanded_name};
pub(crate) use string::{lower_ascii_letter, to_escaped_string};
pub(crate) use text::HtmlString;
#[cfg(test)]
pub(crate) use text::ToHtmlString;
pub(crate) use token::{
    CharacterTokens, CommentToken, Doctype, DoctypeToken, EOFToken, LexHandler, LexerAction,
    LexerResult, NullCharacterToken, ParseError, StartTag, Tag, TagKind, TagToken, Token,
};
