/// Match one HTML tag token pattern.
macro_rules! tag {
    (<>) => {
        crate::lex::Tag {
            kind: crate::lex::TagKind::StartTag,
            ..
        }
    };
    (<>|$($tail:tt)*) => {
        tag!(<>) | tag!($($tail)*)
    };

    (</>) => {
        crate::lex::Tag {
            kind: crate::lex::TagKind::EndTag,
            ..
        }
    };
    (</>|$($tail:tt)*) => {
        tag!(</>) | tag!($($tail)*)
    };

    (<$tag:tt>) => {
        crate::lex::Tag {
            kind: crate::lex::TagKind::StartTag,
            name: $crate::lex::local_name!($tag),
            ..
        }
    };
    (<$tag:tt>|$($tail:tt)*) => {
        tag!(<$tag>) | tag!($($tail)*)
    };

    (</$tag:tt>) => {
        crate::lex::Tag {
            kind: crate::lex::TagKind::EndTag,
            name: $crate::lex::local_name!($tag),
            ..
        }
    };
    (</$tag:tt>|$($tail:tt)*) => {
        tag!(</$tag>) | tag!($($tail)*)
    };
}

pub(crate) use tag;

/// Declare one HTML tag-set predicate.
macro_rules! declare_tag_set (
    (pub $name:ident = $($toks:tt)+) => (
        pub(crate) fn $name(name: crate::lex::ExpandedName<'_>) -> bool {
            $crate::parse::tag::declare_tag_set!(impl name = $($toks)+)
        }
    );

    ($name:ident = $($toks:tt)+) => (
        fn $name(name: crate::lex::ExpandedName<'_>) -> bool {
            $crate::parse::tag::declare_tag_set!(impl name = $($toks)+)
        }
    );

    (impl $param:ident = [$supr:ident] - $($tag:tt)+) => (
        match $param {
            $( $crate::lex::expanded_name!(html $tag) => false, )+
            other => $supr(other),
        }
    );

    (impl $param:ident = [$supr:ident] + $($tag:tt)+) => (
        match $param {
            $( $crate::lex::expanded_name!(html $tag) => true, )+
            other => $supr(other),
        }
    );

    (impl $param:ident = $($tag:tt)+) => (
        match $param {
            $( $crate::lex::expanded_name!(html $tag) => true, )+
            _ => false,
        }
    );
);

pub(crate) use declare_tag_set;

use crate::lex::expanded_name;

declare_tag_set!(pub html_default_scope =
    "applet" "caption" "html" "table" "td" "th" "marquee" "object" "select" "template");

/// Return whether one name is in the default HTML scope.
#[inline(always)]
pub(crate) fn default_scope(name: crate::lex::ExpandedName<'_>) -> bool {
    html_default_scope(name)
        || mathml_text_integration_point(name)
        || svg_html_integration_point(name)
}

declare_tag_set!(pub list_item_scope = [default_scope] + "ol" "ul");
declare_tag_set!(pub button_scope = [default_scope] + "button");
declare_tag_set!(pub table_scope = "html" "table" "template");

declare_tag_set!(pub table_body_context = "tbody" "tfoot" "thead" "template" "html");
declare_tag_set!(pub table_row_context = "tr" "template" "html");
declare_tag_set!(pub td_th = "td" "th");

declare_tag_set!(pub cursory_implied_end =
    "dd" "dt" "li" "option" "optgroup" "p" "rb" "rp" "rt" "rtc");

declare_tag_set!(pub thorough_implied_end = [cursory_implied_end]
    + "caption" "colgroup" "tbody" "td" "tfoot" "th" "thead" "tr");

declare_tag_set!(pub heading_tag = "h1" "h2" "h3" "h4" "h5" "h6");

declare_tag_set!(pub special_tag =
    "address" "applet" "area" "article" "aside" "base" "basefont" "bgsound" "blockquote" "body"
    "br" "button" "caption" "center" "col" "colgroup" "dd" "details" "dir" "div" "dl" "dt" "embed"
    "fieldset" "figcaption" "figure" "footer" "form" "frame" "frameset" "h1" "h2" "h3" "h4" "h5"
    "h6" "head" "header" "hgroup" "hr" "html" "iframe" "img" "input" "isindex" "li" "link"
    "listing" "main" "marquee" "menu" "meta" "nav" "noembed" "noframes" "noscript"
    "object" "ol" "p" "param" "plaintext" "pre" "script" "section" "select" "source" "style"
    "summary" "table" "tbody" "td" "template" "textarea" "tfoot" "th" "thead" "title" "tr" "track"
    "ul" "wbr" "xmp");

/// Return whether one MathML element is one text integration point.
pub(crate) fn mathml_text_integration_point(name: crate::lex::ExpandedName<'_>) -> bool {
    matches!(
        name,
        expanded_name!(mathml "mi")
            | expanded_name!(mathml "mo")
            | expanded_name!(mathml "mn")
            | expanded_name!(mathml "ms")
            | expanded_name!(mathml "mtext")
    )
}

/// Return whether one SVG element is one HTML integration point.
pub(crate) fn svg_html_integration_point(name: crate::lex::ExpandedName<'_>) -> bool {
    matches!(
        name,
        expanded_name!(svg "foreignObject")
            | expanded_name!(svg "desc")
            | expanded_name!(svg "title")
    )
}
