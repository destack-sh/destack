use destack_source::StringId;

use crate::{Block, Linkage, Local, LocalNodeId, Node, NodeType, Type, TypedValue, Value};

/// Memory allocation restrictions for a function.
///
/// This allows marking functions as realtime-safe (no managed allocations)
/// or embedded-safe (stack only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AllocationMode {
    /// No restrictions on allocation.
    #[default]
    Any,
    /// Managed allocation forbidden (realtime-safe).
    /// Only `RawAlloc`, `RawFree`, and `StackAlloc` are allowed.
    NoManaged,
    /// No heap allocation at all (stack only, embedded-safe).
    /// Only `StackAlloc` is allowed.
    StackOnly,
}

impl AllocationMode {
    /// Text representation for formatting/parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            AllocationMode::Any => "any",
            AllocationMode::NoManaged => "no_managed",
            AllocationMode::StackOnly => "stack_only",
        }
    }
}

/// A function in MIR.
///
/// Functions are the top-level compilation unit, containing:
/// - Parameters as SSA values
/// - Local variables as stack slots
/// - Basic blocks forming a control flow graph
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// The function's name (for linking and debugging).
    pub name: StringId,
    /// Function parameters as typed SSA values.
    pub parameters: Vec<TypedValue>,
    /// The return type.
    pub return_type: LocalNodeId<Type>,
    /// Linkage (local, export, or import).
    pub linkage: Linkage,
    /// Memory allocation restrictions for this function.
    pub allocation_mode: AllocationMode,
    /// Local variables (stack-allocated slots for mutable bindings).
    /// Empty for imported functions.
    pub locals: Vec<LocalNodeId<Local>>,
    /// All basic blocks in this function.
    /// Empty for imported functions.
    pub blocks: Vec<LocalNodeId<Block>>,
    /// The entry block (execution starts here).
    /// None for imported functions.
    pub entry: Option<LocalNodeId<Block>>,

    /// Counter for allocating unique SSA value IDs.
    pub(crate) next_value_id: u32,
}

impl Node for Function {
    const TYPE: NodeType = NodeType::Function;
}

impl Function {
    /// Create a new local (private) function with the given signature.
    pub fn new(
        name: StringId,
        parameters: Vec<TypedValue>,
        return_type: LocalNodeId<Type>,
        entry: LocalNodeId<Block>,
    ) -> Self {
        let next_value_id = parameters.iter().map(|p| p.value.0 + 1).max().unwrap_or(0);
        Self {
            name,
            parameters,
            return_type,
            linkage: Linkage::Local,
            allocation_mode: AllocationMode::Any,
            locals: Vec::new(),
            blocks: Vec::new(),
            entry: Some(entry),
            next_value_id,
        }
    }

    /// Create an imported function declaration (no body).
    pub fn import(
        name: StringId,
        parameters: Vec<TypedValue>,
        return_type: LocalNodeId<Type>,
    ) -> Self {
        Self {
            name,
            parameters,
            return_type,
            linkage: Linkage::Import,
            allocation_mode: AllocationMode::Any,
            locals: Vec::new(),
            blocks: Vec::new(),
            entry: None,
            next_value_id: 0,
        }
    }

    /// Set the linkage and return self (builder pattern).
    pub fn with_linkage(mut self, linkage: Linkage) -> Self {
        self.linkage = linkage;
        self
    }

    /// Check if this function is imported (defined elsewhere).
    pub fn is_import(&self) -> bool {
        self.linkage.is_import()
    }

    /// Allocate a new SSA value.
    pub fn next_value(&mut self) -> Value {
        let id = self.next_value_id;
        self.next_value_id += 1;
        Value::new(id)
    }

    /// Add a local variable and return its id.
    pub fn add_local(&mut self, local: LocalNodeId<Local>) {
        self.locals.push(local);
    }

    /// Add a basic block and return its id.
    pub fn add_block(&mut self, block: LocalNodeId<Block>) {
        self.blocks.push(block);
    }
}
