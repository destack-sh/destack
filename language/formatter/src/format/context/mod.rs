mod annotation;
mod cache;
mod format_context;
mod source;

pub use cache::*;
pub use format_context::*;

pub(crate) use std::borrow::Cow;
pub(crate) use std::cell::{Cell, OnceCell, Ref, RefCell};
pub(crate) use std::rc::Rc;

pub(crate) use ast::{
    AnnotationPosition, Argument, Blank, Block, Comment, Declaration, Declarator, Decorator,
    DependencyItem, Doc, EnumField, Expression, Keyword, LocalNodeId, LocalNodeIdAny, MatchCase,
    Member, Node, NodeParentIndex, NodeTree, NodeTreeImpl, NodeType, Parameter, Pattern,
    PatternField, Property, TokenSpan, TokenType, WhereClause, normalize_comment_payload,
};
pub(crate) use destack_ast as ast;
pub(crate) use destack_core::ImmutableStringPool;
pub(crate) use destack_fir::format::{Format, FormatContext, FormatResult, Formatter, GroupId};
pub(crate) use destack_source::{File, MultiSpan, NodeSourceMap, Span};
pub(crate) use rustc_hash::FxHashMap;
pub(crate) use smallvec::SmallVec;

pub(crate) use crate::format::analysis::timing::{
    FormatterTimingEntry, FormatterTimingScope, FormatterTimingTag, FormatterTimings,
    tag_for_node_type,
};
pub(crate) use crate::format::annotation::formatter_annotation_projection;

pub(crate) const ANNOTATION_STATE_NONE: u8 = 1;
pub(crate) const ANNOTATION_STATE_PRESENT: u8 = 2;
pub(crate) const ANNOTATION_STATE_CACHED: u8 = 3;
pub(crate) const NODE_BOOL_STATE_UNKNOWN: u8 = 0;
pub(crate) const NODE_BOOL_STATE_FALSE: u8 = 1;
pub(crate) const NODE_BOOL_STATE_TRUE: u8 = 2;
pub(crate) const NODE_SPAN_CHAR_LEN_UNKNOWN: u32 = u32::MAX;
pub(crate) const TYPE_CONTEXT_STATE_UNKNOWN: u8 = 0;
pub(crate) const TYPE_CONTEXT_STATE_FALSE: u8 = 1;
pub(crate) const TYPE_CONTEXT_STATE_TRUE: u8 = 2;
