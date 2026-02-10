use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use destack_ast::NodeType;

/// Static timing tag for formatter instrumentation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct FormatterTimingTag {
    name: &'static str,
}

impl FormatterTimingTag {
    /// Create a timing tag from a static name.
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }

    /// Return the tag name.
    pub const fn name(self) -> &'static str {
        self.name
    }
}

/// Aggregated timing entry for formatter tags.
#[derive(Debug, Clone, Copy)]
pub struct FormatterTimingEntry {
    /// The timing tag name.
    pub name: &'static str,
    /// The total duration recorded for this tag.
    pub duration: Duration,
    /// The number of samples recorded.
    pub count: usize,
}

/// Timing collector for formatter instrumentation.
#[derive(Debug, Default)]
pub struct FormatterTimings {
    entries: RefCell<HashMap<&'static str, FormatterTimingEntry>>,
}

impl FormatterTimings {
    /// Record a timing sample.
    pub fn record(&self, tag: FormatterTimingTag, duration: Duration) {
        let mut entries = self.entries.borrow_mut();
        let entry = entries.entry(tag.name()).or_insert(FormatterTimingEntry {
            name: tag.name(),
            duration: Duration::new(0, 0),
            count: 0,
        });
        entry.duration += duration;
        entry.count += 1;
    }

    /// Snapshot current timing entries.
    pub fn snapshot(&self) -> Vec<FormatterTimingEntry> {
        let entries = self.entries.borrow();
        let mut snapshot = entries.values().copied().collect::<Vec<_>>();
        snapshot.sort_by_key(|entry| entry.name);
        snapshot
    }
}

/// Scoped timing guard that records elapsed time on drop.
#[derive(Debug)]
pub enum FormatterTimingScope {
    /// Disabled timing scope for normal formatter runs.
    Disabled,
    /// Enabled timing scope with start metadata.
    Enabled {
        timings: Rc<FormatterTimings>,
        name: &'static str,
        started_at: Instant,
    },
}

impl FormatterTimingScope {
    /// Start a timing scope if timings are enabled.
    pub fn new(timings: Option<&Rc<FormatterTimings>>, tag: FormatterTimingTag) -> Self {
        let Some(timings) = timings else {
            return Self::Disabled;
        };
        Self::Enabled {
            timings: Rc::clone(timings),
            name: tag.name(),
            started_at: Instant::now(),
        }
    }
}

impl Drop for FormatterTimingScope {
    fn drop(&mut self) {
        let (timings, name, started_at) = match self {
            Self::Disabled => return,
            Self::Enabled {
                timings,
                name,
                started_at,
            } => (timings, *name, *started_at),
        };

        let elapsed = started_at.elapsed();
        timings.record(FormatterTimingTag::new(name), elapsed);
    }
}

/// Return whether formatter timings are enabled from environment variables.
pub fn timings_enabled_from_env() -> bool {
    std::env::var("DESTACK_FORMATTER_TIMINGS")
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .map(|value| value > 0)
        .or_else(|| {
            std::env::var("DESTACK_TIMINGS")
                .ok()
                .and_then(|value| value.parse::<u8>().ok())
                .map(|value| value > 0)
        })
        .unwrap_or(false)
}

/// Return a node formatting timing tag for a node type.
pub fn tag_for_node_type(node_type: NodeType) -> FormatterTimingTag {
    match node_type {
        NodeType::Expression => tags::FORMAT_NODE_EXPRESSION,
        NodeType::Block => tags::FORMAT_NODE_BLOCK,
        NodeType::Declaration => tags::FORMAT_NODE_DECLARATION,
        NodeType::Property => tags::FORMAT_NODE_PROPERTY,
        NodeType::Member => tags::FORMAT_NODE_MEMBER,
        NodeType::EnumField => tags::FORMAT_NODE_ENUM_FIELD,
        NodeType::WhereClause => tags::FORMAT_NODE_WHERE_CLAUSE,
        NodeType::DependencyItem => tags::FORMAT_NODE_DEPENDENCY_ITEM,
        NodeType::Parameter => tags::FORMAT_NODE_PARAMETER,
        NodeType::Argument => tags::FORMAT_NODE_ARGUMENT,
        NodeType::MatchCase => tags::FORMAT_NODE_MATCH_CASE,
        NodeType::Pattern => tags::FORMAT_NODE_PATTERN,
        NodeType::PatternField => tags::FORMAT_NODE_PATTERN_FIELD,
        NodeType::Declarator => tags::FORMAT_NODE_DECLARATOR,
        NodeType::Annotation => tags::FORMAT_NODE_ANNOTATION,
        NodeType::Blank => tags::FORMAT_NODE_BLANK,
        NodeType::Doc => tags::FORMAT_NODE_DOC,
        NodeType::Comment => tags::FORMAT_NODE_COMMENT,
        NodeType::Decorator => tags::FORMAT_NODE_DECORATOR,
    }
}

pub mod tags {
    use super::FormatterTimingTag;

    pub const FORMAT_STATEMENT_LIST: FormatterTimingTag =
        FormatterTimingTag::new("format.statement_list");
    pub const FORMAT_BLOCK_STATEMENTS: FormatterTimingTag =
        FormatterTimingTag::new("format.block.statements");
    pub const FORMAT_EXPRESSION: FormatterTimingTag = FormatterTimingTag::new("format.expression");
    pub const FORMAT_EXPRESSION_STATEMENT: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.statement");
    pub const FORMAT_EXPRESSION_STATEMENT_IMPORT: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.statement.import");
    pub const FORMAT_EXPRESSION_STATEMENT_EXPORT: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.statement.export");
    pub const FORMAT_EXPRESSION_STATEMENT_LET: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.statement.let");
    pub const FORMAT_EXPRESSION_STATEMENT_CONTROL: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.statement.control");
    pub const FORMAT_EXPRESSION_STATEMENT_RETURN: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.statement.return");
    pub const FORMAT_EXPRESSION_PRIMARY: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.primary");
    pub const FORMAT_EXPRESSION_PRIMARY_PATH: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.primary.path");
    pub const FORMAT_EXPRESSION_PRIMARY_PARENTHESES: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.primary.parenthesized");
    pub const FORMAT_EXPRESSION_PRIMARY_TYPE_CONDITIONAL: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.primary.type_conditional");
    pub const FORMAT_EXPRESSION_PRIMARY_ARRAY: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.primary.array");
    pub const FORMAT_EXPRESSION_PRIMARY_TUPLE: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.primary.tuple");
    pub const FORMAT_EXPRESSION_PRIMARY_OBJECT: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.primary.object");
    pub const FORMAT_EXPRESSION_PRIMARY_TREE: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.primary.tree");
    pub const FORMAT_EXPRESSION_PRIMARY_TYPE_MAPPED: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.primary.type_mapped");
    pub const FORMAT_EXPRESSION_PRIMARY_PARENTHESES_DROP_POLICY: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.primary.parenthesized.drop_policy");
    pub const FORMAT_EXPRESSION_PRIMARY_PARENTHESES_LEADING_TRIVIA: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.primary.parenthesized.leading_trivia");
    pub const FORMAT_EXPRESSION_PRIMARY_PARENTHESES_BOUNDARY_COMMENTS: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.primary.parenthesized.boundary_comments");
    pub const FORMAT_EXPRESSION_PRIMARY_PARENTHESES_TYPE_DROP: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.primary.parenthesized.type_drop");
    pub const FORMAT_EXPRESSION_OPERATOR: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.operator");
    pub const FORMAT_EXPRESSION_OPERATOR_BINARY: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.operator.binary");
    pub const FORMAT_EXPRESSION_OPERATOR_CHAIN: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.operator.chain");
    pub const FORMAT_EXPRESSION_OPERATOR_CALL: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.operator.call");
    pub const FORMAT_EXPRESSION_CALL: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.call");
    pub const FORMAT_EXPRESSION_CALL_ARGUMENTS: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.call.arguments");
    pub const FORMAT_EXPRESSION_CALL_ARGUMENTS_COMMENT_PROFILE: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.call.arguments.comment_profile");
    pub const FORMAT_EXPRESSION_CALL_ARGUMENTS_EXPANSION_PROFILE: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.call.arguments.expansion_profile");
    pub const FORMAT_EXPRESSION_CALL_EMPTY_ARGUMENTS: FormatterTimingTag =
        FormatterTimingTag::new("format.expression.call.empty_arguments");

    pub const FORMAT_NODE_EXPRESSION: FormatterTimingTag =
        FormatterTimingTag::new("format.node.expression");
    pub const FORMAT_NODE_BLOCK: FormatterTimingTag = FormatterTimingTag::new("format.node.block");
    pub const FORMAT_NODE_DECLARATION: FormatterTimingTag =
        FormatterTimingTag::new("format.node.declaration");
    pub const FORMAT_NODE_PROPERTY: FormatterTimingTag =
        FormatterTimingTag::new("format.node.property");
    pub const FORMAT_NODE_MEMBER: FormatterTimingTag =
        FormatterTimingTag::new("format.node.member");
    pub const FORMAT_NODE_ENUM_FIELD: FormatterTimingTag =
        FormatterTimingTag::new("format.node.enum_field");
    pub const FORMAT_NODE_WHERE_CLAUSE: FormatterTimingTag =
        FormatterTimingTag::new("format.node.where_clause");
    pub const FORMAT_NODE_DEPENDENCY_ITEM: FormatterTimingTag =
        FormatterTimingTag::new("format.node.dependency_item");
    pub const FORMAT_NODE_PARAMETER: FormatterTimingTag =
        FormatterTimingTag::new("format.node.parameter");
    pub const FORMAT_NODE_ARGUMENT: FormatterTimingTag =
        FormatterTimingTag::new("format.node.argument");
    pub const FORMAT_NODE_MATCH_CASE: FormatterTimingTag =
        FormatterTimingTag::new("format.node.match_case");
    pub const FORMAT_NODE_PATTERN: FormatterTimingTag =
        FormatterTimingTag::new("format.node.pattern");
    pub const FORMAT_NODE_PATTERN_FIELD: FormatterTimingTag =
        FormatterTimingTag::new("format.node.pattern_field");
    pub const FORMAT_NODE_DECLARATOR: FormatterTimingTag =
        FormatterTimingTag::new("format.node.declarator");
    pub const FORMAT_NODE_ANNOTATION: FormatterTimingTag =
        FormatterTimingTag::new("format.node.annotation");
    pub const FORMAT_NODE_BLANK: FormatterTimingTag = FormatterTimingTag::new("format.node.blank");
    pub const FORMAT_NODE_DOC: FormatterTimingTag = FormatterTimingTag::new("format.node.doc");
    pub const FORMAT_NODE_COMMENT: FormatterTimingTag =
        FormatterTimingTag::new("format.node.comment");
    pub const FORMAT_NODE_DECORATOR: FormatterTimingTag =
        FormatterTimingTag::new("format.node.decorator");
}
