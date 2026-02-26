mod annotation;
mod attachment;
mod blank;
mod boundary;
mod declaration;
mod endofline;
mod expression;
mod facts;
mod operator;
mod ownership;
mod ownline;
mod placement;
mod remaining;
mod render;
mod semicolon;
mod statement;

pub(crate) use attachment::formatter_annotation_projection;
pub use render::{AnnotationCapture, Annotations};
pub(crate) use semicolon::{
    classify_semicolon_guard_comment_seam, expression_needs_statement_terminator,
    statement_wrapper_needs_semicolon,
};
