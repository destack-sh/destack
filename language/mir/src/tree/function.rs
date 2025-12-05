use destack_source::StringId;

use crate::{Block, Local, LocalNodeId, Node, NodeType, Type, TypedValue, Value};

/// Functions are the top-level compilation unit. Each function contains:
/// - Parameters (SSA values)
/// - Local variables (stack slots for mutable bindings)
/// - Basic blocks (control flow graph)
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// Name of the function.
    pub name: StringId,
    /// Function parameters (SSA values).
    pub parameters: Vec<TypedValue>,
    /// Return type.
    pub return_type: LocalNodeId<Type>,
    /// Local variables (stack slots).
    pub locals: Vec<LocalNodeId<Local>>,
    /// Basic blocks (CFG).
    pub blocks: Vec<LocalNodeId<Block>>,
    /// Entry block.
    pub entry: LocalNodeId<Block>,

    // counters for generating unique ids
    /// Next available SSA value id.
    pub(crate) next_value_id: u32,
}

impl Node for Function {
    const TYPE: NodeType = NodeType::Function;
}

impl Function {
    /// Create a new function with the given signature.
    pub fn new(
        name: StringId,
        parameters: Vec<TypedValue>,
        return_type: LocalNodeId<Type>,
        entry: LocalNodeId<Block>,
    ) -> Self {
        // count the values used by params
        let next_value_id = parameters.iter().map(|p| p.value.0 + 1).max().unwrap_or(0);

        Self {
            name,
            parameters,
            return_type,
            locals: Vec::new(),
            blocks: Vec::new(),
            entry,
            next_value_id,
        }
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
