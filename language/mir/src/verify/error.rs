use std::fmt;

use crate::{Block, Instruction, Local, LocalNodeId, LocalNodeIdAny, Node, NodeType, Value};

/// Anchor for a verification error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VerifyAnchor {
    /// The MIR node tied to this error.
    pub node: LocalNodeIdAny,
}

impl VerifyAnchor {
    /// Create an anchor for a MIR node.
    pub fn node<T: Node>(node: LocalNodeId<T>) -> Self {
        Self { node: node.into() }
    }
}

/// Error produced when MIR verification fails.
#[derive(Debug, Clone)]
pub enum VerifyError {
    /// A function with no blocks still has an entry block.
    FunctionHasEntryWithoutBlocks {
        /// The anchor for this error.
        anchor: VerifyAnchor,
    },
    /// A function is missing its entry block.
    MissingEntryBlock {
        /// The anchor for this error.
        anchor: VerifyAnchor,
    },
    /// The entry block is not listed in the function blocks.
    EntryBlockNotInFunction {
        /// The anchor for this error.
        anchor: VerifyAnchor,
    },
    /// Entry block parameters do not match function parameters.
    EntryBlockParameterMismatch {
        /// The anchor for this error.
        anchor: VerifyAnchor,
    },
    /// A block id is listed more than once.
    DuplicateBlockId {
        /// The duplicate block id.
        block_id: LocalNodeId<Block>,
        /// The anchor for this error.
        anchor: VerifyAnchor,
    },
    /// A local id is listed more than once.
    DuplicateLocalId {
        /// The duplicate local id.
        local_id: LocalNodeId<Local>,
        /// The anchor for this error.
        anchor: VerifyAnchor,
    },
    /// An instruction id is listed more than once.
    DuplicateInstructionId {
        /// The duplicate instruction id.
        instruction_id: LocalNodeId<Instruction>,
        /// The anchor for this error.
        anchor: VerifyAnchor,
    },
    /// A value is defined more than once.
    DuplicateValueDefinition {
        /// The duplicate value.
        value: Value,
        /// The anchor for this error.
        anchor: VerifyAnchor,
    },
    /// A value definition is missing a type entry.
    MissingValueType {
        /// The value missing a type.
        value: Value,
        /// The anchor for this error.
        anchor: VerifyAnchor,
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
        anchor: VerifyAnchor,
    },
    /// A local reference is not defined in the function.
    LocalReferenceNotInFunction {
        /// The unknown local id.
        local_id: LocalNodeId<Local>,
        /// The anchor for this error.
        anchor: VerifyAnchor,
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
        anchor: VerifyAnchor,
    },
    /// A value is used before it is defined.
    UseOfUndefinedValue {
        /// The undefined value.
        value: Value,
        /// The anchor for this error.
        anchor: VerifyAnchor,
    },
    /// A terminator references an unknown block target.
    UnknownBlockTarget {
        /// The unknown block id.
        block_id: LocalNodeId<Block>,
        /// The anchor for this error.
        anchor: VerifyAnchor,
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
        anchor: VerifyAnchor,
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
        anchor: VerifyAnchor,
    },
    /// A switch case value is repeated.
    DuplicateSwitchCaseValue {
        /// The duplicated case value.
        value: i64,
        /// The anchor for this error.
        anchor: VerifyAnchor,
    },
    /// A call has the wrong number of arguments.
    CallArgumentCountMismatch {
        /// The expected argument count.
        expected: usize,
        /// The actual argument count.
        got: usize,
        /// The anchor for this error.
        anchor: VerifyAnchor,
    },
    /// An aggregate constructor has the wrong number of elements.
    AggregateArgumentCountMismatch {
        /// The expected argument count.
        expected: usize,
        /// The actual argument count.
        got: usize,
        /// The anchor for this error.
        anchor: VerifyAnchor,
    },
    /// An aggregate constructor uses the wrong type kind.
    AggregateTypeMismatch {
        /// The expected type kind.
        expected: &'static str,
        /// The found type kind.
        found: &'static str,
        /// The anchor for this error.
        anchor: VerifyAnchor,
    },
    /// Metadata violates a required invariant.
    MetadataInvariantViolation {
        /// The invariant description.
        message: String,
        /// The anchor for this error.
        anchor: VerifyAnchor,
    },
    /// A call returns a value for a void function.
    CallReturnValueNotAllowedForVoid {
        /// The anchor for this error.
        anchor: VerifyAnchor,
    },
    /// A void function returns a value.
    ReturnValueNotAllowedForVoid {
        /// The anchor for this error.
        anchor: VerifyAnchor,
    },
    /// A non void function does not return a value.
    ReturnValueRequiredForNonVoid {
        /// The anchor for this error.
        anchor: VerifyAnchor,
    },
    /// A tail call returns the wrong kind of value for the current function.
    TailCallReturnTypeMismatch {
        /// The anchor for this error.
        anchor: VerifyAnchor,
    },
}

impl VerifyError {
    /// Return the anchor associated with this error.
    pub fn anchor(&self) -> Option<VerifyAnchor> {
        match self {
            VerifyError::FunctionHasEntryWithoutBlocks { anchor }
            | VerifyError::MissingEntryBlock { anchor }
            | VerifyError::EntryBlockNotInFunction { anchor }
            | VerifyError::EntryBlockParameterMismatch { anchor }
            | VerifyError::DuplicateBlockId { anchor, .. }
            | VerifyError::DuplicateLocalId { anchor, .. }
            | VerifyError::DuplicateInstructionId { anchor, .. }
            | VerifyError::DuplicateValueDefinition { anchor, .. }
            | VerifyError::MissingValueType { anchor, .. }
            | VerifyError::InvalidNodeReference { anchor, .. }
            | VerifyError::LocalReferenceNotInFunction { anchor, .. }
            | VerifyError::ArgumentSliceOutOfBounds { anchor, .. }
            | VerifyError::UseOfUndefinedValue { anchor, .. }
            | VerifyError::UnknownBlockTarget { anchor, .. }
            | VerifyError::BlockArgumentCountMismatch { anchor, .. }
            | VerifyError::ResumeArgumentCountMismatch { anchor, .. }
            | VerifyError::DuplicateSwitchCaseValue { anchor, .. }
            | VerifyError::CallArgumentCountMismatch { anchor, .. }
            | VerifyError::AggregateArgumentCountMismatch { anchor, .. }
            | VerifyError::AggregateTypeMismatch { anchor, .. }
            | VerifyError::MetadataInvariantViolation { anchor, .. }
            | VerifyError::CallReturnValueNotAllowedForVoid { anchor }
            | VerifyError::ReturnValueNotAllowedForVoid { anchor }
            | VerifyError::ReturnValueRequiredForNonVoid { anchor }
            | VerifyError::TailCallReturnTypeMismatch { anchor } => Some(*anchor),
        }
    }
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerifyError::FunctionHasEntryWithoutBlocks { .. } => {
                write!(f, "function with no blocks must not have an entry block")
            }
            VerifyError::MissingEntryBlock { .. } => write!(f, "function must have an entry block"),
            VerifyError::EntryBlockNotInFunction { .. } => {
                write!(f, "entry block must be in function blocks")
            }
            VerifyError::EntryBlockParameterMismatch { .. } => {
                write!(f, "entry block parameters must match function parameters")
            }
            VerifyError::DuplicateBlockId { block_id, .. } => {
                write!(f, "duplicate block id block{}", block_id.id)
            }
            VerifyError::DuplicateLocalId { local_id, .. } => {
                write!(f, "duplicate local id local{}", local_id.id)
            }
            VerifyError::DuplicateInstructionId { instruction_id, .. } => {
                write!(f, "duplicate instruction id inst{}", instruction_id.id)
            }
            VerifyError::DuplicateValueDefinition { value, .. } => {
                write!(f, "duplicate value definition v{}", value.id())
            }
            VerifyError::MissingValueType { value, .. } => {
                write!(f, "missing type for value v{}", value.id())
            }
            VerifyError::InvalidNodeReference {
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
            VerifyError::LocalReferenceNotInFunction { local_id, .. } => {
                write!(
                    f,
                    "local reference local{} not defined in function",
                    local_id.id
                )
            }
            VerifyError::ArgumentSliceOutOfBounds {
                start, count, len, ..
            } => write!(
                f,
                "argument slice out of bounds start {start} count {count} len {len}"
            ),
            VerifyError::UseOfUndefinedValue { value, .. } => {
                write!(f, "use of undefined value v{}", value.id())
            }
            VerifyError::UnknownBlockTarget { block_id, .. } => {
                write!(f, "unknown block target block{}", block_id.id)
            }
            VerifyError::BlockArgumentCountMismatch {
                block_label,
                expected,
                got,
                ..
            } => write!(
                f,
                "block argument count mismatch for {block_label} expected {expected} got {got}"
            ),
            VerifyError::ResumeArgumentCountMismatch {
                block_label,
                expected,
                got,
                ..
            } => write!(
                f,
                "resume argument count mismatch for {block_label} expected {expected} got {got}"
            ),
            VerifyError::DuplicateSwitchCaseValue { value, .. } => {
                write!(f, "duplicate switch case value {value}")
            }
            VerifyError::CallArgumentCountMismatch { expected, got, .. } => {
                write!(
                    f,
                    "call argument count mismatch expected {expected} got {got}"
                )
            }
            VerifyError::AggregateArgumentCountMismatch { expected, got, .. } => {
                write!(
                    f,
                    "aggregate argument count mismatch expected {expected} got {got}"
                )
            }
            VerifyError::AggregateTypeMismatch {
                expected, found, ..
            } => {
                write!(f, "aggregate type mismatch expected {expected} got {found}")
            }
            VerifyError::MetadataInvariantViolation { message, .. } => {
                write!(f, "metadata invariant violation: {message}")
            }
            VerifyError::CallReturnValueNotAllowedForVoid { .. } => {
                write!(f, "call return value not allowed for void function")
            }
            VerifyError::ReturnValueNotAllowedForVoid { .. } => {
                write!(f, "return value not allowed for void function")
            }
            VerifyError::ReturnValueRequiredForNonVoid { .. } => {
                write!(f, "return value required for non void function")
            }
            VerifyError::TailCallReturnTypeMismatch { .. } => {
                write!(f, "tail call return type mismatch")
            }
        }
    }
}

impl std::error::Error for VerifyError {}

/// Result type for MIR verification.
pub type VerifyResult<T> = Result<T, VerifyError>;
