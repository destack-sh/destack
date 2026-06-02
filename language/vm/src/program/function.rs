use std::collections::HashMap;

use destack_engine as engine;
use destack_mir as mir;

use super::{ArgumentRange, CellLayout, Instruction, MovePair, MoveRange, cell_layout_from_type};
use crate::Error;

/// Lowered function with executable code and frame metadata.
#[derive(Clone, Debug)]
pub(crate) struct Function {
    /// Original MIR function id.
    pub mir_function: mir::LocalNodeId<mir::Function>,
    /// The logical frame layout for this function.
    pub frame_layout: engine::FrameLayoutId,
    /// Function parameters.
    pub parameters: ArgumentRange,
    /// Entry block index.
    pub entry: u32,
    /// Contiguous instruction code.
    pub code: Vec<Instruction>,
    /// Lowered block ranges.
    pub blocks: Vec<Block>,
    /// Pool of argument values referenced by ranges.
    pub argument_pool: Vec<mir::Value>,
    /// Pool of value move pairs referenced by ranges.
    pub move_pool: Vec<MovePair>,
}

impl Function {
    /// Return one block's instruction count.
    #[inline(always)]
    pub(crate) fn block_len(&self, block: u32) -> Option<usize> {
        self.blocks
            .get(block as usize)
            .map(|block| block.len as usize)
    }
}

/// Lowered function registry owned by one program.
#[derive(Debug)]
pub(crate) struct FunctionTable {
    /// Lowered functions by dense index.
    functions: Vec<Function>,
    /// Call target by function id.
    target_by_id: HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
}

impl FunctionTable {
    /// Build a lowered function table from lowered functions and call targets.
    pub(super) fn new(
        functions: Vec<Function>,
        target_by_id: HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
    ) -> Self {
        Self {
            functions,
            target_by_id,
        }
    }

    /// Return the call target for the given function id.
    pub(crate) fn call_target(
        &self,
        func_id: mir::LocalNodeId<mir::Function>,
    ) -> Option<CallTarget> {
        self.target_by_id.get(&func_id).copied()
    }

    /// Return a lowered local function index for the given function id.
    pub(crate) fn local_index(&self, func_id: mir::LocalNodeId<mir::Function>) -> Option<u32> {
        match self.call_target(func_id)? {
            CallTarget::Local(index) => Some(index),
            CallTarget::Import => None,
        }
    }

    /// Return a lowered function by dense index.
    pub(crate) fn function_by_index(&self, index: u32) -> Option<&Function> {
        self.functions.get(index as usize)
    }

    /// Return one lowered function by function id.
    pub(crate) fn function_by_id(
        &self,
        func_id: mir::LocalNodeId<mir::Function>,
    ) -> Option<&Function> {
        let index = self.local_index(func_id)?;

        self.functions.get(index as usize)
    }

    /// Return the closure environment cell layout for one function id.
    pub(crate) fn environment_layout(
        &self,
        tree: &mir::Tree,
        func_id: mir::LocalNodeId<mir::Function>,
    ) -> Option<CellLayout> {
        if !self.target_by_id.contains_key(&func_id) {
            return None;
        }

        environment_layout(tree, func_id)
    }

    /// Require one function to match one bare signature type.
    pub(crate) fn validate_signature(
        &self,
        tree: &mir::Tree,
        function_id: mir::LocalNodeId<mir::Function>,
        signature: mir::LocalNodeId<mir::Type>,
    ) -> Result<(), Error> {
        if !self.target_by_id.contains_key(&function_id) {
            return Err(Error::undefined_function(function_id));
        }

        if function_signature_matches(tree, function_id, signature)? {
            return Ok(());
        }

        Err(Error::type_mismatch(
            format!("function signature {signature:?}"),
            format!("function {function_id:?}"),
        ))
    }
}

/// Return one closure environment cell layout.
fn environment_layout(
    tree: &mir::Tree,
    function_id: mir::LocalNodeId<mir::Function>,
) -> Option<CellLayout> {
    let function = tree.get(function_id);
    let environment = function.environment.as_ref()?;
    let environment_type = environment.ty()?;

    cell_layout_from_type(tree, environment_type)
}

/// Return whether one function matches one bare signature type.
fn function_signature_matches(
    tree: &mir::Tree,
    function_id: mir::LocalNodeId<mir::Function>,
    signature: mir::LocalNodeId<mir::Type>,
) -> Result<bool, Error> {
    let mir::Type::FunctionSignature {
        parameters, result, ..
    } = tree.get(signature)
    else {
        return Err(Error::invalid_instruction());
    };
    let function = tree.get(function_id);

    if function.parameters.len() != parameters.len() {
        return Ok(false);
    }

    // compare parameter and result types directly from immutable MIR
    let parameters_match = function
        .parameters
        .iter()
        .zip(parameters.iter())
        .all(|(actual, expected)| actual.ty == *expected);
    let result_matches = function.return_type == *result;

    Ok(parameters_match && result_matches)
}

/// Program call target for one function id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CallTarget {
    /// The function id names one imported function.
    Import,
    /// The function id names one lowered function.
    Local(u32),
}

/// Lowered basic block position inside one function.
#[derive(Clone, Debug)]
pub(crate) struct Block {
    /// Original MIR block id.
    pub mir_block: mir::LocalNodeId<mir::Block>,
    /// First instruction in the function code.
    pub start: u32,
    /// Number of instructions in this block.
    pub len: u32,
    /// Source MIR point for each lowered PC in this block.
    pub mir_point_by_pc: Vec<u32>,
}

/// Instruction bytes emitted for one block during lowering.
#[derive(Clone, Debug)]
pub(crate) struct BlockCode {
    /// Original MIR block id.
    pub mir_block: mir::LocalNodeId<mir::Block>,
    /// Instructions including terminator.
    pub instructions: Vec<Instruction>,
    /// Source MIR point for each lowered PC in this block.
    pub mir_point_by_pc: Vec<u32>,
}

/// One lowered switch case.
#[derive(Clone, Debug)]
pub(crate) struct SwitchCase {
    /// Match value.
    pub value: i128,
    /// Target block.
    pub target: u32,
    /// Block parameter moves.
    pub moves: MoveRange,
}
