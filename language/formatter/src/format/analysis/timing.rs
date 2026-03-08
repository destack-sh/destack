use destack_ast::NodeType;
#[cfg(feature = "timings")]
use std::cell::RefCell;
#[cfg(feature = "timings")]
use std::ptr::NonNull;
use std::time::Duration;

#[cfg(feature = "timings")]
#[cfg(not(target_os = "macos"))]
use std::time::Instant;

#[cfg(feature = "timings")]
#[cfg(target_os = "macos")]
use std::sync::OnceLock;

// timing tag inventory

/// Static timing tag for formatter instrumentation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum FormatterTimingTag {
    /// `format.statement_list`
    StatementList,
    /// `format.block.statements`
    BlockStatements,
    /// `format.expression`
    Expression,
    /// `format.expression.statement`
    ExpressionStatement,
    /// `format.expression.statement.import`
    ExpressionStatementImport,
    /// `format.expression.statement.export`
    ExpressionStatementExport,
    /// `format.expression.statement.let`
    ExpressionStatementLet,
    /// `format.expression.statement.control`
    ExpressionStatementControl,
    /// `format.expression.statement.return`
    ExpressionStatementReturn,
    /// `format.expression.primary`
    ExpressionPrimary,
    /// `format.expression.primary.path`
    ExpressionPrimaryPath,
    /// `format.expression.primary.parenthesized`
    ExpressionPrimaryParenthesized,
    /// `format.expression.primary.type_conditional`
    ExpressionPrimaryTypeConditional,
    /// `format.expression.primary.array`
    ExpressionPrimaryArray,
    /// `format.expression.primary.tuple`
    ExpressionPrimaryTuple,
    /// `format.expression.primary.object`
    ExpressionPrimaryObject,
    /// `format.expression.primary.tree`
    ExpressionPrimaryTree,
    /// `format.expression.primary.type_mapped`
    ExpressionPrimaryTypeMapped,
    /// `format.expression.primary.parenthesized.drop_mode`
    ExpressionPrimaryParenthesizedDropMode,
    /// `format.expression.primary.parenthesized.leading_trivia`
    ExpressionPrimaryParenthesizedLeadingTrivia,
    /// `format.expression.primary.parenthesized.boundary_comments`
    ExpressionPrimaryParenthesizedBoundaryComments,
    /// `format.expression.primary.parenthesized.type_drop`
    ExpressionPrimaryParenthesizedTypeDrop,
    /// `format.expression.operator`
    ExpressionOperator,
    /// `format.expression.operator.binary`
    ExpressionOperatorBinary,
    /// `format.expression.operator.chain`
    ExpressionOperatorChain,
    /// `format.expression.operator.call`
    ExpressionOperatorCall,
    /// `format.expression.call`
    ExpressionCall,
    /// `format.expression.call.arguments`
    ExpressionCallArguments,
    /// `format.expression.call.arguments.comment_scan`
    ExpressionCallArgumentsCommentScan,
    /// `format.expression.call.arguments.expansion_scan`
    ExpressionCallArgumentsExpansionScan,
    /// `format.expression.call.arguments.layout`
    ExpressionCallArgumentsLayout,
    /// `format.expression.call.arguments.layout.select`
    ExpressionCallArgumentsLayoutSelect,
    /// `format.expression.call.arguments.layout.render`
    ExpressionCallArgumentsLayoutRender,
    /// `format.expression.call.arguments.hug_last`
    ExpressionCallArgumentsHugLast,
    /// `format.expression.call.arguments.list_default`
    ExpressionCallArgumentsListDefault,
    /// `format.expression.call.empty_arguments`
    ExpressionCallEmptyArguments,
    /// `format.node.expression`
    NodeExpression,
    /// `format.node.block`
    NodeBlock,
    /// `format.node.declaration`
    NodeDeclaration,
    /// `format.node.property`
    NodeProperty,
    /// `format.node.member`
    NodeMember,
    /// `format.node.enum_field`
    NodeEnumField,
    /// `format.node.where_clause`
    NodeWhereClause,
    /// `format.node.dependency_item`
    NodeDependencyItem,
    /// `format.node.parameter`
    NodeParameter,
    /// `format.node.argument`
    NodeArgument,
    /// `format.node.match_case`
    NodeMatchCase,
    /// `format.node.pattern`
    NodePattern,
    /// `format.node.pattern_field`
    NodePatternField,
    /// `format.node.declarator`
    NodeDeclarator,
    /// `format.node.annotation`
    NodeAnnotation,
    /// `format.node.blank`
    NodeBlank,
    /// `format.node.doc`
    NodeDoc,
    /// `format.node.comment`
    NodeComment,
    /// `format.node.decorator`
    NodeDecorator,
}

impl FormatterTimingTag {
    /// Return the tag slot index.
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Return the tag name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::StatementList => "format.statement_list",
            Self::BlockStatements => "format.block.statements",
            Self::Expression => "format.expression",
            Self::ExpressionStatement => "format.expression.statement",
            Self::ExpressionStatementImport => "format.expression.statement.import",
            Self::ExpressionStatementExport => "format.expression.statement.export",
            Self::ExpressionStatementLet => "format.expression.statement.let",
            Self::ExpressionStatementControl => "format.expression.statement.control",
            Self::ExpressionStatementReturn => "format.expression.statement.return",
            Self::ExpressionPrimary => "format.expression.primary",
            Self::ExpressionPrimaryPath => "format.expression.primary.path",
            Self::ExpressionPrimaryParenthesized => "format.expression.primary.parenthesized",
            Self::ExpressionPrimaryTypeConditional => "format.expression.primary.type_conditional",
            Self::ExpressionPrimaryArray => "format.expression.primary.array",
            Self::ExpressionPrimaryTuple => "format.expression.primary.tuple",
            Self::ExpressionPrimaryObject => "format.expression.primary.object",
            Self::ExpressionPrimaryTree => "format.expression.primary.tree",
            Self::ExpressionPrimaryTypeMapped => "format.expression.primary.type_mapped",
            Self::ExpressionPrimaryParenthesizedDropMode => {
                "format.expression.primary.parenthesized.drop_mode"
            }
            Self::ExpressionPrimaryParenthesizedLeadingTrivia => {
                "format.expression.primary.parenthesized.leading_trivia"
            }
            Self::ExpressionPrimaryParenthesizedBoundaryComments => {
                "format.expression.primary.parenthesized.boundary_comments"
            }
            Self::ExpressionPrimaryParenthesizedTypeDrop => {
                "format.expression.primary.parenthesized.type_drop"
            }
            Self::ExpressionOperator => "format.expression.operator",
            Self::ExpressionOperatorBinary => "format.expression.operator.binary",
            Self::ExpressionOperatorChain => "format.expression.operator.chain",
            Self::ExpressionOperatorCall => "format.expression.operator.call",
            Self::ExpressionCall => "format.expression.call",
            Self::ExpressionCallArguments => "format.expression.call.arguments",
            Self::ExpressionCallArgumentsCommentScan => {
                "format.expression.call.arguments.comment_scan"
            }
            Self::ExpressionCallArgumentsExpansionScan => {
                "format.expression.call.arguments.expansion_scan"
            }
            Self::ExpressionCallArgumentsLayout => "format.expression.call.arguments.layout",
            Self::ExpressionCallArgumentsLayoutSelect => {
                "format.expression.call.arguments.layout.select"
            }
            Self::ExpressionCallArgumentsLayoutRender => {
                "format.expression.call.arguments.layout.render"
            }
            Self::ExpressionCallArgumentsHugLast => "format.expression.call.arguments.hug_last",
            Self::ExpressionCallArgumentsListDefault => {
                "format.expression.call.arguments.list_default"
            }
            Self::ExpressionCallEmptyArguments => "format.expression.call.empty_arguments",
            Self::NodeExpression => "format.node.expression",
            Self::NodeBlock => "format.node.block",
            Self::NodeDeclaration => "format.node.declaration",
            Self::NodeProperty => "format.node.property",
            Self::NodeMember => "format.node.member",
            Self::NodeEnumField => "format.node.enum_field",
            Self::NodeWhereClause => "format.node.where_clause",
            Self::NodeDependencyItem => "format.node.dependency_item",
            Self::NodeParameter => "format.node.parameter",
            Self::NodeArgument => "format.node.argument",
            Self::NodeMatchCase => "format.node.match_case",
            Self::NodePattern => "format.node.pattern",
            Self::NodePatternField => "format.node.pattern_field",
            Self::NodeDeclarator => "format.node.declarator",
            Self::NodeAnnotation => "format.node.annotation",
            Self::NodeBlank => "format.node.blank",
            Self::NodeDoc => "format.node.doc",
            Self::NodeComment => "format.node.comment",
            Self::NodeDecorator => "format.node.decorator",
        }
    }
}

/// Ordered timing tags used by the collector.
#[cfg(feature = "timings")]
const FORMATTER_TIMING_TAGS: &[FormatterTimingTag] = &[
    FormatterTimingTag::StatementList,
    FormatterTimingTag::BlockStatements,
    FormatterTimingTag::Expression,
    FormatterTimingTag::ExpressionStatement,
    FormatterTimingTag::ExpressionStatementImport,
    FormatterTimingTag::ExpressionStatementExport,
    FormatterTimingTag::ExpressionStatementLet,
    FormatterTimingTag::ExpressionStatementControl,
    FormatterTimingTag::ExpressionStatementReturn,
    FormatterTimingTag::ExpressionPrimary,
    FormatterTimingTag::ExpressionPrimaryPath,
    FormatterTimingTag::ExpressionPrimaryParenthesized,
    FormatterTimingTag::ExpressionPrimaryTypeConditional,
    FormatterTimingTag::ExpressionPrimaryArray,
    FormatterTimingTag::ExpressionPrimaryTuple,
    FormatterTimingTag::ExpressionPrimaryObject,
    FormatterTimingTag::ExpressionPrimaryTree,
    FormatterTimingTag::ExpressionPrimaryTypeMapped,
    FormatterTimingTag::ExpressionPrimaryParenthesizedDropMode,
    FormatterTimingTag::ExpressionPrimaryParenthesizedLeadingTrivia,
    FormatterTimingTag::ExpressionPrimaryParenthesizedBoundaryComments,
    FormatterTimingTag::ExpressionPrimaryParenthesizedTypeDrop,
    FormatterTimingTag::ExpressionOperator,
    FormatterTimingTag::ExpressionOperatorBinary,
    FormatterTimingTag::ExpressionOperatorChain,
    FormatterTimingTag::ExpressionOperatorCall,
    FormatterTimingTag::ExpressionCall,
    FormatterTimingTag::ExpressionCallArguments,
    FormatterTimingTag::ExpressionCallArgumentsCommentScan,
    FormatterTimingTag::ExpressionCallArgumentsExpansionScan,
    FormatterTimingTag::ExpressionCallArgumentsLayout,
    FormatterTimingTag::ExpressionCallArgumentsLayoutSelect,
    FormatterTimingTag::ExpressionCallArgumentsLayoutRender,
    FormatterTimingTag::ExpressionCallArgumentsHugLast,
    FormatterTimingTag::ExpressionCallArgumentsListDefault,
    FormatterTimingTag::ExpressionCallEmptyArguments,
    FormatterTimingTag::NodeExpression,
    FormatterTimingTag::NodeBlock,
    FormatterTimingTag::NodeDeclaration,
    FormatterTimingTag::NodeProperty,
    FormatterTimingTag::NodeMember,
    FormatterTimingTag::NodeEnumField,
    FormatterTimingTag::NodeWhereClause,
    FormatterTimingTag::NodeDependencyItem,
    FormatterTimingTag::NodeParameter,
    FormatterTimingTag::NodeArgument,
    FormatterTimingTag::NodeMatchCase,
    FormatterTimingTag::NodePattern,
    FormatterTimingTag::NodePatternField,
    FormatterTimingTag::NodeDeclarator,
    FormatterTimingTag::NodeAnnotation,
    FormatterTimingTag::NodeBlank,
    FormatterTimingTag::NodeDoc,
    FormatterTimingTag::NodeComment,
    FormatterTimingTag::NodeDecorator,
];

// timing clock

/// One platform timing stamp captured at scope entry.
#[cfg(feature = "timings")]
#[derive(Clone, Copy, Debug)]
pub struct FormatterTimingStamp {
    /// The raw stamp value.
    raw: FormatterTimingStampRaw,
}

#[cfg(feature = "timings")]
#[cfg(target_os = "macos")]
type FormatterTimingStampRaw = u64;

#[cfg(feature = "timings")]
#[cfg(not(target_os = "macos"))]
type FormatterTimingStampRaw = Instant;

/// Capture one timing stamp.
#[cfg(feature = "timings")]
fn timing_now() -> FormatterTimingStamp {
    #[cfg(target_os = "macos")]
    {
        FormatterTimingStamp {
            raw: unsafe { mach_absolute_time() },
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        FormatterTimingStamp {
            raw: Instant::now(),
        }
    }
}

/// Return elapsed timing units since one captured stamp.
#[cfg(feature = "timings")]
fn timing_elapsed_units(started_at: FormatterTimingStamp) -> u64 {
    #[cfg(target_os = "macos")]
    {
        unsafe { mach_absolute_time() }.saturating_sub(started_at.raw)
    }

    #[cfg(not(target_os = "macos"))]
    {
        started_at.raw.elapsed().as_nanos().min(u64::MAX as u128) as u64
    }
}

/// Convert raw timing units into a duration.
#[cfg(feature = "timings")]
fn timing_duration_from_units(units: u64) -> Duration {
    #[cfg(target_os = "macos")]
    {
        let info = timebase_info();
        let nanos = (units as u128 * info.numer as u128) / info.denom as u128;
        return Duration::from_nanos(nanos.min(u64::MAX as u128) as u64);
    }

    #[cfg(not(target_os = "macos"))]
    {
        Duration::from_nanos(units)
    }
}

#[cfg(feature = "timings")]
#[cfg(target_os = "macos")]
unsafe extern "C" {
    /// Return the current monotonic mach absolute time.
    fn mach_absolute_time() -> u64;

    /// Return the mach timebase conversion ratio.
    fn mach_timebase_info(info: *mut MachTimebaseInfo) -> i32;
}

#[cfg(feature = "timings")]
#[cfg(target_os = "macos")]
/// Cached mach timebase conversion info.
#[derive(Clone, Copy)]
struct TimebaseInfo {
    /// The numerator of the conversion ratio.
    numer: u32,
    /// The denominator of the conversion ratio.
    denom: u32,
}

#[cfg(feature = "timings")]
#[cfg(target_os = "macos")]
#[repr(C)]
struct MachTimebaseInfo {
    /// The numerator of the conversion ratio.
    numer: u32,
    /// The denominator of the conversion ratio.
    denom: u32,
}

#[cfg(feature = "timings")]
#[cfg(target_os = "macos")]
/// Load and cache the process timebase info.
fn timebase_info() -> TimebaseInfo {
    static TIMEBASE_INFO: OnceLock<TimebaseInfo> = OnceLock::new();

    *TIMEBASE_INFO.get_or_init(|| {
        let mut info = MachTimebaseInfo { numer: 1, denom: 1 };
        let status = unsafe { mach_timebase_info(&mut info) };
        if status != 0 || info.denom == 0 {
            return TimebaseInfo { numer: 1, denom: 1 };
        }

        TimebaseInfo {
            numer: info.numer,
            denom: info.denom,
        }
    })
}

// collector

/// Aggregated timing slot for one formatter tag.
#[cfg(feature = "timings")]
#[derive(Debug, Clone, Copy, Default)]
struct FormatterTimingSlot {
    /// The accumulated raw timing units.
    total_units: u64,
    /// The number of recorded samples.
    count: usize,
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
#[cfg(feature = "timings")]
#[derive(Debug, Clone)]
pub struct FormatterTimings {
    slots: RefCell<Vec<FormatterTimingSlot>>,
}

#[cfg(not(feature = "timings"))]
/// Timing collector for formatter instrumentation.
#[derive(Debug, Clone, Default)]
pub struct FormatterTimings;

#[cfg(feature = "timings")]
impl FormatterTimings {
    /// Create one timing collector sized for all known tags.
    pub fn new() -> Self {
        Self {
            slots: RefCell::new(vec![
                FormatterTimingSlot::default();
                FORMATTER_TIMING_TAGS.len()
            ]),
        }
    }

    /// Record one raw timing sample.
    pub fn record_units(&self, tag: FormatterTimingTag, elapsed_units: u64) {
        let mut slots = self.slots.borrow_mut();
        let slot = &mut slots[tag.index()];
        slot.total_units = slot.total_units.saturating_add(elapsed_units);
        slot.count = slot.count.saturating_add(1);
    }

    /// Snapshot current timing entries.
    pub fn snapshot(&self) -> Vec<FormatterTimingEntry> {
        let slots = self.slots.borrow();
        FORMATTER_TIMING_TAGS
            .iter()
            .copied()
            .zip(slots.iter().copied())
            .filter_map(|(tag, slot)| {
                (slot.count > 0).then_some(FormatterTimingEntry {
                    name: tag.name(),
                    duration: timing_duration_from_units(slot.total_units),
                    count: slot.count,
                })
            })
            .collect()
    }
}

#[cfg(not(feature = "timings"))]
impl FormatterTimings {
    /// Record one raw timing sample.
    #[inline]
    pub fn record_units(&self, _tag: FormatterTimingTag, _elapsed_units: u64) {}

    /// Snapshot current timing entries.
    #[inline]
    pub fn snapshot(&self) -> Vec<FormatterTimingEntry> {
        Vec::new()
    }
}

#[cfg(feature = "timings")]
impl Default for FormatterTimings {
    fn default() -> Self {
        Self::new()
    }
}

/// Scoped timing guard that records elapsed units on drop.
#[cfg(feature = "timings")]
#[derive(Debug)]
pub enum FormatterTimingScope {
    /// Disabled timing scope for normal formatter runs.
    Disabled,
    /// Enabled timing scope with start metadata.
    Enabled {
        timings: NonNull<FormatterTimings>,
        tag: FormatterTimingTag,
        started_at: FormatterTimingStamp,
    },
}

#[cfg(not(feature = "timings"))]
/// Scoped timing guard that records elapsed units on drop.
#[derive(Debug, Clone, Copy, Default)]
pub struct FormatterTimingScope;

#[cfg(feature = "timings")]
impl FormatterTimingScope {
    /// Start a timing scope if timings are enabled.
    pub fn new(timings: Option<&FormatterTimings>, tag: FormatterTimingTag) -> Self {
        let Some(timings) = timings else {
            return Self::Disabled;
        };
        Self::Enabled {
            timings: NonNull::from(timings),
            tag,
            started_at: timing_now(),
        }
    }
}

#[cfg(not(feature = "timings"))]
impl FormatterTimingScope {
    /// Start a timing scope if timings are enabled.
    #[inline]
    pub const fn new(_timings: Option<&FormatterTimings>, _tag: FormatterTimingTag) -> Self {
        Self
    }
}

#[cfg(feature = "timings")]
impl Drop for FormatterTimingScope {
    fn drop(&mut self) {
        let (timings, tag, started_at) = match self {
            Self::Disabled => return,
            Self::Enabled {
                timings,
                tag,
                started_at,
            } => (*timings, *tag, *started_at),
        };

        let elapsed_units = timing_elapsed_units(started_at);

        // timing scopes always originate from a live formatter context
        unsafe {
            timings.as_ref().record_units(tag, elapsed_units);
        }
    }
}

/// Return a node formatting timing tag for a node type.
pub fn tag_for_node_type(node_type: NodeType) -> FormatterTimingTag {
    match node_type {
        NodeType::Expression => FORMAT_NODE_EXPRESSION,
        NodeType::Block => FORMAT_NODE_BLOCK,
        NodeType::Declaration => FORMAT_NODE_DECLARATION,
        NodeType::Property => FORMAT_NODE_PROPERTY,
        NodeType::Member => FORMAT_NODE_MEMBER,
        NodeType::EnumField => FORMAT_NODE_ENUM_FIELD,
        NodeType::WhereClause => FORMAT_NODE_WHERE_CLAUSE,
        NodeType::DependencyItem => FORMAT_NODE_DEPENDENCY_ITEM,
        NodeType::Parameter => FORMAT_NODE_PARAMETER,
        NodeType::Argument => FORMAT_NODE_ARGUMENT,
        NodeType::MatchCase => FORMAT_NODE_MATCH_CASE,
        NodeType::Pattern => FORMAT_NODE_PATTERN,
        NodeType::PatternField => FORMAT_NODE_PATTERN_FIELD,
        NodeType::Declarator => FORMAT_NODE_DECLARATOR,
        NodeType::Annotation => FORMAT_NODE_ANNOTATION,
        NodeType::Blank => FORMAT_NODE_BLANK,
        NodeType::Doc => FORMAT_NODE_DOC,
        NodeType::Comment => FORMAT_NODE_COMMENT,
        NodeType::Decorator => FORMAT_NODE_DECORATOR,
    }
}

pub const FORMAT_STATEMENT_LIST: FormatterTimingTag = FormatterTimingTag::StatementList;
pub const FORMAT_BLOCK_STATEMENTS: FormatterTimingTag = FormatterTimingTag::BlockStatements;
pub const FORMAT_EXPRESSION: FormatterTimingTag = FormatterTimingTag::Expression;
pub const FORMAT_EXPRESSION_STATEMENT: FormatterTimingTag = FormatterTimingTag::ExpressionStatement;
pub const FORMAT_EXPRESSION_STATEMENT_IMPORT: FormatterTimingTag =
    FormatterTimingTag::ExpressionStatementImport;
pub const FORMAT_EXPRESSION_STATEMENT_EXPORT: FormatterTimingTag =
    FormatterTimingTag::ExpressionStatementExport;
pub const FORMAT_EXPRESSION_STATEMENT_LET: FormatterTimingTag =
    FormatterTimingTag::ExpressionStatementLet;
pub const FORMAT_EXPRESSION_STATEMENT_CONTROL: FormatterTimingTag =
    FormatterTimingTag::ExpressionStatementControl;
pub const FORMAT_EXPRESSION_STATEMENT_RETURN: FormatterTimingTag =
    FormatterTimingTag::ExpressionStatementReturn;
pub const FORMAT_EXPRESSION_PRIMARY: FormatterTimingTag = FormatterTimingTag::ExpressionPrimary;
pub const FORMAT_EXPRESSION_PRIMARY_PATH: FormatterTimingTag =
    FormatterTimingTag::ExpressionPrimaryPath;
pub const FORMAT_EXPRESSION_PRIMARY_PARENTHESES: FormatterTimingTag =
    FormatterTimingTag::ExpressionPrimaryParenthesized;
pub const FORMAT_EXPRESSION_PRIMARY_TYPE_CONDITIONAL: FormatterTimingTag =
    FormatterTimingTag::ExpressionPrimaryTypeConditional;
pub const FORMAT_EXPRESSION_PRIMARY_ARRAY: FormatterTimingTag =
    FormatterTimingTag::ExpressionPrimaryArray;
pub const FORMAT_EXPRESSION_PRIMARY_TUPLE: FormatterTimingTag =
    FormatterTimingTag::ExpressionPrimaryTuple;
pub const FORMAT_EXPRESSION_PRIMARY_OBJECT: FormatterTimingTag =
    FormatterTimingTag::ExpressionPrimaryObject;
pub const FORMAT_EXPRESSION_PRIMARY_TREE: FormatterTimingTag =
    FormatterTimingTag::ExpressionPrimaryTree;
pub const FORMAT_EXPRESSION_PRIMARY_TYPE_MAPPED: FormatterTimingTag =
    FormatterTimingTag::ExpressionPrimaryTypeMapped;
pub const FORMAT_EXPRESSION_PRIMARY_PARENTHESES_DROP_MODE: FormatterTimingTag =
    FormatterTimingTag::ExpressionPrimaryParenthesizedDropMode;
pub const FORMAT_EXPRESSION_PRIMARY_PARENTHESES_LEADING_TRIVIA: FormatterTimingTag =
    FormatterTimingTag::ExpressionPrimaryParenthesizedLeadingTrivia;
pub const FORMAT_EXPRESSION_PRIMARY_PARENTHESES_BOUNDARY_COMMENTS: FormatterTimingTag =
    FormatterTimingTag::ExpressionPrimaryParenthesizedBoundaryComments;
pub const FORMAT_EXPRESSION_PRIMARY_PARENTHESES_TYPE_DROP: FormatterTimingTag =
    FormatterTimingTag::ExpressionPrimaryParenthesizedTypeDrop;
pub const FORMAT_EXPRESSION_OPERATOR: FormatterTimingTag = FormatterTimingTag::ExpressionOperator;
pub const FORMAT_EXPRESSION_OPERATOR_BINARY: FormatterTimingTag =
    FormatterTimingTag::ExpressionOperatorBinary;
pub const FORMAT_EXPRESSION_OPERATOR_CHAIN: FormatterTimingTag =
    FormatterTimingTag::ExpressionOperatorChain;
pub const FORMAT_EXPRESSION_OPERATOR_CALL: FormatterTimingTag =
    FormatterTimingTag::ExpressionOperatorCall;
pub const FORMAT_EXPRESSION_CALL: FormatterTimingTag = FormatterTimingTag::ExpressionCall;
pub const FORMAT_EXPRESSION_CALL_ARGUMENTS: FormatterTimingTag =
    FormatterTimingTag::ExpressionCallArguments;
pub const FORMAT_EXPRESSION_CALL_ARGUMENTS_COMMENT_SCAN: FormatterTimingTag =
    FormatterTimingTag::ExpressionCallArgumentsCommentScan;
pub const FORMAT_EXPRESSION_CALL_ARGUMENTS_EXPANSION_SCAN: FormatterTimingTag =
    FormatterTimingTag::ExpressionCallArgumentsExpansionScan;
pub const FORMAT_EXPRESSION_CALL_ARGUMENTS_LAYOUT: FormatterTimingTag =
    FormatterTimingTag::ExpressionCallArgumentsLayout;
pub const FORMAT_EXPRESSION_CALL_ARGUMENTS_LAYOUT_DECIDE: FormatterTimingTag =
    FormatterTimingTag::ExpressionCallArgumentsLayoutSelect;
pub const FORMAT_EXPRESSION_CALL_ARGUMENTS_LAYOUT_RENDER: FormatterTimingTag =
    FormatterTimingTag::ExpressionCallArgumentsLayoutRender;
pub const FORMAT_EXPRESSION_CALL_ARGUMENTS_HUG_LAST: FormatterTimingTag =
    FormatterTimingTag::ExpressionCallArgumentsHugLast;
pub const FORMAT_EXPRESSION_CALL_ARGUMENTS_LIST_DEFAULT: FormatterTimingTag =
    FormatterTimingTag::ExpressionCallArgumentsListDefault;
pub const FORMAT_EXPRESSION_CALL_EMPTY_ARGUMENTS: FormatterTimingTag =
    FormatterTimingTag::ExpressionCallEmptyArguments;

pub const FORMAT_NODE_EXPRESSION: FormatterTimingTag = FormatterTimingTag::NodeExpression;
pub const FORMAT_NODE_BLOCK: FormatterTimingTag = FormatterTimingTag::NodeBlock;
pub const FORMAT_NODE_DECLARATION: FormatterTimingTag = FormatterTimingTag::NodeDeclaration;
pub const FORMAT_NODE_PROPERTY: FormatterTimingTag = FormatterTimingTag::NodeProperty;
pub const FORMAT_NODE_MEMBER: FormatterTimingTag = FormatterTimingTag::NodeMember;
pub const FORMAT_NODE_ENUM_FIELD: FormatterTimingTag = FormatterTimingTag::NodeEnumField;
pub const FORMAT_NODE_WHERE_CLAUSE: FormatterTimingTag = FormatterTimingTag::NodeWhereClause;
pub const FORMAT_NODE_DEPENDENCY_ITEM: FormatterTimingTag = FormatterTimingTag::NodeDependencyItem;
pub const FORMAT_NODE_PARAMETER: FormatterTimingTag = FormatterTimingTag::NodeParameter;
pub const FORMAT_NODE_ARGUMENT: FormatterTimingTag = FormatterTimingTag::NodeArgument;
pub const FORMAT_NODE_MATCH_CASE: FormatterTimingTag = FormatterTimingTag::NodeMatchCase;
pub const FORMAT_NODE_PATTERN: FormatterTimingTag = FormatterTimingTag::NodePattern;
pub const FORMAT_NODE_PATTERN_FIELD: FormatterTimingTag = FormatterTimingTag::NodePatternField;
pub const FORMAT_NODE_DECLARATOR: FormatterTimingTag = FormatterTimingTag::NodeDeclarator;
pub const FORMAT_NODE_ANNOTATION: FormatterTimingTag = FormatterTimingTag::NodeAnnotation;
pub const FORMAT_NODE_BLANK: FormatterTimingTag = FormatterTimingTag::NodeBlank;
pub const FORMAT_NODE_DOC: FormatterTimingTag = FormatterTimingTag::NodeDoc;
pub const FORMAT_NODE_COMMENT: FormatterTimingTag = FormatterTimingTag::NodeComment;
pub const FORMAT_NODE_DECORATOR: FormatterTimingTag = FormatterTimingTag::NodeDecorator;
