use destack_dir as dir;

use super::{InferId, Reachability};

/// Guarded table write emitted after inference and validation.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckOutput {
    /// The reachability guard for the output.
    pub reachability: Reachability,
    /// The guarded output.
    pub value: CheckOutputValue,
}

impl CheckOutput {
    /// Create an always-reachable output.
    pub fn always(value: CheckOutputValue) -> Self {
        Self {
            reachability: Reachability::Always,
            value,
        }
    }

    /// Create an output with a reachability guard.
    pub fn new(reachability: Reachability, value: CheckOutputValue) -> Self {
        Self {
            reachability,
            value,
        }
    }
}

/// Table write emitted after inference and validation.
#[derive(Debug, Clone, PartialEq)]
pub enum CheckOutputValue {
    /// Set one node type attachment.
    ///
    /// ```ds
    /// source + 1
    /// // attach the solved expression type to the binary node
    /// ```
    SetNodeType {
        /// The node.
        node: dir::GlobalNodeIdAny,
        /// The type attachment slot.
        attachment: NodeTypeOutput,
        /// The inferred type.
        ty: InferId,
    },
    /// Set one symbol type attachment.
    ///
    /// ```ds
    /// let value: int32;
    /// // attach int32 to the value symbol
    /// ```
    SetSymbolType {
        /// The symbol.
        symbol: dir::GlobalSymbolId,
        /// The type attachment slot.
        attachment: SymbolTypeOutput,
        /// The inferred type.
        ty: InferId,
    },
    /// Set one name resolution.
    ///
    /// ```ds
    /// value
    /// // resolve the path to symbol value
    /// ```
    SetNameResolution {
        /// The source node.
        node: dir::GlobalNodeIdAny,
        /// The selected symbol.
        symbol: dir::GlobalSymbolId,
    },
    /// Set one member resolution.
    ///
    /// ```ds
    /// point.x
    /// // resolve x to the selected field or accessor
    /// ```
    SetMemberResolution {
        /// The source node.
        node: dir::GlobalNodeIdAny,
        /// The selected member.
        resolution: dir::MemberResolution,
    },
    /// Set one call resolution.
    ///
    /// ```ds
    /// parse(text)
    /// // resolve parse to the selected callable overload
    /// ```
    SetCallResolution {
        /// The source node.
        node: dir::GlobalNodeIdAny,
        /// The selected callable.
        resolution: dir::CallResolution,
    },
}

/// Node type attachment written by check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeTypeOutput {
    /// Declared type.
    ///
    /// ```ds
    /// let value: int32;
    /// // int32 is the declared node type
    /// ```
    Declared,
    /// Inferred type.
    ///
    /// ```ds
    /// let value = 1;
    /// // 1 is the inferred initializer type
    /// ```
    Inferred,
    /// Receiver type.
    ///
    /// ```ds
    /// point.x
    /// // type(point) is the receiver type
    /// ```
    Receiver,
    /// Contextual type.
    ///
    /// ```ds
    /// let value: int32 = 1;
    /// // int32 is the contextual type for 1
    /// ```
    Contextual,
    /// Function signature type.
    ///
    /// ```ds
    /// function parse(text: string): int32;
    /// // the function node has this signature type
    /// ```
    Signature,
}

/// Symbol type attachment written by check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolTypeOutput {
    /// Value type.
    ///
    /// ```ds
    /// let value: int32;
    /// // value has value type int32
    /// ```
    Value,
    /// Instance type.
    ///
    /// ```ds
    /// struct Box<T> {}
    /// // Box has an instance type for constructed values
    /// ```
    Instance,
    /// Alias target type.
    ///
    /// ```ds
    /// type Count = int32;
    /// // Count aliases int32
    /// ```
    Alias,
}
