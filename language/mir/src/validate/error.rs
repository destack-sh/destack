use std::fmt;

use crate::{
    Block, Instruction, Local, LocalNodeId, LocalNodeIdAny, Node, NodeType, Terminator, Tree, Value,
};

/// Anchor for a validation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValidateAnchor {
    /// The MIR node tied to this error.
    pub node: LocalNodeIdAny,
}

impl ValidateAnchor {
    /// Create an anchor for a MIR node.
    pub fn node<T: Node>(node: LocalNodeId<T>) -> Self {
        Self { node: node.into() }
    }

    /// Create an anchor for one raw node id.
    pub fn for_raw_node(tree: &Tree, node_id: u32) -> Self {
        let node_type = tree.get_node_type(node_id);

        Self {
            node: LocalNodeIdAny::new(node_id, node_type),
        }
    }
}

/// Error produced when MIR validation fails.
#[derive(Debug, Clone)]
pub enum ValidateError {
    /// A function with no blocks still has an entry block.
    FunctionHasEntryWithoutBlocks {
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// A function is missing its entry block.
    MissingEntryBlock {
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// The entry block is not listed in the function blocks.
    EntryBlockNotInFunction {
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// Entry block parameters do not match function parameters.
    EntryBlockParameterMismatch {
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// A block id is listed more than once.
    DuplicateBlockId {
        /// The duplicate block id.
        block_id: LocalNodeId<Block>,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// A function block is missing its finalized name.
    MissingBlockName {
        /// The block missing a name.
        block_id: LocalNodeId<Block>,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// Two blocks in one function share a name.
    DuplicateBlockName {
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// A local id is listed more than once.
    DuplicateLocalId {
        /// The duplicate local id.
        local_id: LocalNodeId<Local>,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// An instruction id is listed more than once.
    DuplicateInstructionId {
        /// The duplicate instruction id.
        instruction_id: LocalNodeId<Instruction>,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// A terminator id is listed more than once.
    DuplicateTerminatorId {
        /// The duplicate terminator id.
        terminator_id: LocalNodeId<Terminator>,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// A value is defined more than once.
    DuplicateValueDefinition {
        /// The duplicate value.
        value: Value,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// A defined SSA value is missing its finalized name.
    MissingValueName {
        /// The value missing a name.
        value: Value,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// Two values in one function share a name.
    DuplicateValueName {
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// Recovered parse-only MIR was passed to the normal validator.
    RecoveredSyntaxNode {
        /// The recovered node kind.
        kind: &'static str,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// A value definition is missing a type entry.
    MissingValueType {
        /// The value missing a type.
        value: Value,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// A node reference does not point to the expected node type.
    InvalidNodeReference {
        /// The expected node type.
        expected: NodeType,
        /// The found node type, if any.
        found: Option<NodeType>,
        /// The referenced node id.
        node_id: u32,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// A local reference is not defined in the function.
    LocalReferenceNotInFunction {
        /// The unknown local id.
        local_id: LocalNodeId<Local>,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// An argument slice points outside the argument buffer.
    ArgumentSliceOutOfBounds {
        /// The slice start index.
        start: u32,
        /// The slice count.
        count: u16,
        /// The argument buffer length.
        len: usize,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// A value is used before it is defined.
    UseOfUndefinedValue {
        /// The undefined value.
        value: Value,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// A terminator references an unknown block target.
    UnknownBlockTarget {
        /// The unknown block id.
        block_id: LocalNodeId<Block>,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// A block is called with the wrong number of arguments.
    BlockArgumentCountMismatch {
        /// The block label in source order.
        block_label: String,
        /// The expected argument count.
        expected: usize,
        /// The actual argument count.
        got: usize,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// A resume edge has the wrong number of arguments.
    ResumeArgumentCountMismatch {
        /// The block label in source order.
        block_label: String,
        /// The expected argument count.
        expected: usize,
        /// The actual argument count.
        got: usize,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// A switch case value is repeated.
    DuplicateSwitchCaseValue {
        /// The duplicated case value.
        value: i64,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// A call has the wrong number of arguments.
    CallArgumentCountMismatch {
        /// The expected argument count.
        expected: usize,
        /// The actual argument count.
        got: usize,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// An aggregate constructor has the wrong number of elements.
    AggregateArgumentCountMismatch {
        /// The expected argument count.
        expected: usize,
        /// The actual argument count.
        got: usize,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// An aggregate constructor uses the wrong type kind.
    AggregateTypeMismatch {
        /// The expected type kind.
        expected: &'static str,
        /// The found type kind.
        found: &'static str,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// Metadata violates a required invariant.
    MetadataInvariantViolation {
        /// The invariant description.
        message: String,
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// A call returns a value for a void function.
    CallReturnValueNotAllowedForVoid {
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// A void function returns a value.
    ReturnValueNotAllowedForVoid {
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// A non void function does not return a value.
    ReturnValueRequiredForNonVoid {
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
    /// A tail call returns the wrong kind of value for the current function.
    TailCallReturnTypeMismatch {
        /// The anchor for this error.
        anchor: ValidateAnchor,
    },
}

impl ValidateError {
    /// Return the anchor associated with this error.
    pub fn anchor(&self) -> Option<ValidateAnchor> {
        match self {
            ValidateError::FunctionHasEntryWithoutBlocks { anchor }
            | ValidateError::MissingEntryBlock { anchor }
            | ValidateError::EntryBlockNotInFunction { anchor }
            | ValidateError::EntryBlockParameterMismatch { anchor }
            | ValidateError::DuplicateBlockId { anchor, .. }
            | ValidateError::MissingBlockName { anchor, .. }
            | ValidateError::DuplicateBlockName { anchor, .. }
            | ValidateError::DuplicateLocalId { anchor, .. }
            | ValidateError::DuplicateInstructionId { anchor, .. }
            | ValidateError::DuplicateTerminatorId { anchor, .. }
            | ValidateError::DuplicateValueDefinition { anchor, .. }
            | ValidateError::MissingValueName { anchor, .. }
            | ValidateError::DuplicateValueName { anchor, .. }
            | ValidateError::RecoveredSyntaxNode { anchor, .. }
            | ValidateError::MissingValueType { anchor, .. }
            | ValidateError::InvalidNodeReference { anchor, .. }
            | ValidateError::LocalReferenceNotInFunction { anchor, .. }
            | ValidateError::ArgumentSliceOutOfBounds { anchor, .. }
            | ValidateError::UseOfUndefinedValue { anchor, .. }
            | ValidateError::UnknownBlockTarget { anchor, .. }
            | ValidateError::BlockArgumentCountMismatch { anchor, .. }
            | ValidateError::ResumeArgumentCountMismatch { anchor, .. }
            | ValidateError::DuplicateSwitchCaseValue { anchor, .. }
            | ValidateError::CallArgumentCountMismatch { anchor, .. }
            | ValidateError::AggregateArgumentCountMismatch { anchor, .. }
            | ValidateError::AggregateTypeMismatch { anchor, .. }
            | ValidateError::MetadataInvariantViolation { anchor, .. }
            | ValidateError::CallReturnValueNotAllowedForVoid { anchor }
            | ValidateError::ReturnValueNotAllowedForVoid { anchor }
            | ValidateError::ReturnValueRequiredForNonVoid { anchor }
            | ValidateError::TailCallReturnTypeMismatch { anchor } => Some(*anchor),
        }
    }
}

impl fmt::Display for ValidateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidateError::FunctionHasEntryWithoutBlocks { .. } => {
                write!(f, "function with no blocks must not have an entry block")
            }
            ValidateError::MissingEntryBlock { .. } => {
                write!(f, "function must have an entry block")
            }
            ValidateError::EntryBlockNotInFunction { .. } => {
                write!(f, "entry block must be in function blocks")
            }
            ValidateError::EntryBlockParameterMismatch { .. } => {
                write!(f, "entry block parameters must match function parameters")
            }
            ValidateError::DuplicateBlockId { block_id, .. } => {
                write!(f, "duplicate block id block{}", block_id.id)
            }
            ValidateError::MissingBlockName { block_id, .. } => {
                write!(f, "missing MIR block name for block{}", block_id.id)
            }
            ValidateError::DuplicateBlockName { .. } => {
                write!(f, "duplicate MIR block name")
            }
            ValidateError::DuplicateLocalId { local_id, .. } => {
                write!(f, "duplicate local id local{}", local_id.id)
            }
            ValidateError::DuplicateInstructionId { instruction_id, .. } => {
                write!(f, "duplicate instruction id inst{}", instruction_id.id)
            }
            ValidateError::DuplicateTerminatorId { terminator_id, .. } => {
                write!(f, "duplicate terminator id term{}", terminator_id.id)
            }
            ValidateError::DuplicateValueDefinition { value, .. } => {
                write!(f, "duplicate value definition v{}", value.id())
            }
            ValidateError::MissingValueName { value, .. } => {
                write!(f, "missing MIR value name for v{}", value.id())
            }
            ValidateError::DuplicateValueName { .. } => {
                write!(f, "duplicate MIR value name")
            }
            ValidateError::RecoveredSyntaxNode { kind, .. } => {
                write!(f, "recovered MIR {kind} is not valid")
            }
            ValidateError::MissingValueType { value, .. } => {
                write!(f, "missing type for value v{}", value.id())
            }
            ValidateError::InvalidNodeReference {
                expected,
                found,
                node_id,
                ..
            } => match found {
                Some(found) => write!(
                    f,
                    "invalid node reference id{node_id} expected {expected:?} got {found:?}"
                ),
                None => write!(
                    f,
                    "invalid node reference id{node_id} expected {expected:?} got none"
                ),
            },
            ValidateError::LocalReferenceNotInFunction { local_id, .. } => {
                write!(
                    f,
                    "local reference local{} not defined in function",
                    local_id.id
                )
            }
            ValidateError::ArgumentSliceOutOfBounds {
                start, count, len, ..
            } => write!(
                f,
                "argument slice out of bounds start {start} count {count} len {len}"
            ),
            ValidateError::UseOfUndefinedValue { value, .. } => {
                write!(f, "use of undefined value v{}", value.id())
            }
            ValidateError::UnknownBlockTarget { block_id, .. } => {
                write!(f, "unknown block target block{}", block_id.id)
            }
            ValidateError::BlockArgumentCountMismatch {
                block_label,
                expected,
                got,
                ..
            } => write!(
                f,
                "block argument count mismatch for {block_label} expected {expected} got {got}"
            ),
            ValidateError::ResumeArgumentCountMismatch {
                block_label,
                expected,
                got,
                ..
            } => write!(
                f,
                "resume argument count mismatch for {block_label} expected {expected} got {got}"
            ),
            ValidateError::DuplicateSwitchCaseValue { value, .. } => {
                write!(f, "duplicate switch case value {value}")
            }
            ValidateError::CallArgumentCountMismatch { expected, got, .. } => {
                write!(
                    f,
                    "call argument count mismatch expected {expected} got {got}"
                )
            }
            ValidateError::AggregateArgumentCountMismatch { expected, got, .. } => {
                write!(
                    f,
                    "aggregate argument count mismatch expected {expected} got {got}"
                )
            }
            ValidateError::AggregateTypeMismatch {
                expected, found, ..
            } => {
                write!(f, "aggregate type mismatch expected {expected} got {found}")
            }
            ValidateError::MetadataInvariantViolation { message, .. } => {
                write!(f, "metadata invariant violation: {message}")
            }
            ValidateError::CallReturnValueNotAllowedForVoid { .. } => {
                write!(f, "call return value not allowed for void function")
            }
            ValidateError::ReturnValueNotAllowedForVoid { .. } => {
                write!(f, "return value not allowed for void function")
            }
            ValidateError::ReturnValueRequiredForNonVoid { .. } => {
                write!(f, "return value required for non void function")
            }
            ValidateError::TailCallReturnTypeMismatch { .. } => {
                write!(f, "tail call return type mismatch")
            }
        }
    }
}

impl std::error::Error for ValidateError {}

/// Result type for MIR validation.
pub type ValidateResult<T> = Result<T, ValidateError>;
