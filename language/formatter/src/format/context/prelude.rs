pub(super) use std::borrow::Cow;
pub(super) use std::cell::{Cell, OnceCell, Ref, RefCell};
pub(super) use std::rc::Rc;

pub(super) use ast::{
    AnnotationPosition, Argument, Blank, Block, Comment, Declaration, Declarator, Decorator,
    DependencyItem, Doc, EnumField, Expression, LocalNodeId, LocalNodeIdAny, MatchCase, Member,
    Node, NodeParentIndex, NodeTree, NodeTreeImpl, NodeType, Parameter, Pattern, PatternField,
    Property, TokenSpan, TokenType, WhereClause, normalize_comment_payload,
};
pub(super) use destack_ast as ast;
pub(super) use destack_base::ImmutableStringPool;
pub(super) use destack_fir::format::{Format, FormatContext, FormatResult, Formatter, GroupId};
pub(super) use destack_source::{File, MultiSpan, NodeSourceMap, Span};
pub(super) use rustc_hash::FxHashMap;
pub(super) use smallvec::SmallVec;

pub(super) use crate::format::analysis::timing::{
    FormatterTimingEntry, FormatterTimingScope, FormatterTimingTag, FormatterTimings,
    tag_for_node_type, timings_enabled_from_env,
};
pub(super) use crate::format::comments::build_formatter_annotation_projection;

pub(super) const ANNOTATION_STATE_NONE: u8 = 1;
pub(super) const ANNOTATION_STATE_PRESENT: u8 = 2;
pub(super) const ANNOTATION_STATE_CACHED: u8 = 3;
pub(super) const NODE_BOOL_STATE_UNKNOWN: u8 = 0;
pub(super) const NODE_BOOL_STATE_FALSE: u8 = 1;
pub(super) const NODE_BOOL_STATE_TRUE: u8 = 2;
pub(super) const NODE_SPAN_CHAR_LEN_UNKNOWN: u32 = u32::MAX;
pub(super) const TYPE_CONTEXT_STATE_UNKNOWN: u8 = 0;
pub(super) const TYPE_CONTEXT_STATE_FALSE: u8 = 1;
pub(super) const TYPE_CONTEXT_STATE_TRUE: u8 = 2;
