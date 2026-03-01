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
mod terminator;

#[cfg(test)]
mod tests;

pub(crate) use attachment::formatter_annotation_projection;
pub use render::{AnnotationCapture, Annotations};
pub(crate) use terminator::{
    expression_needs_statement_terminator, statement_wrapper_needs_semicolon,
};
