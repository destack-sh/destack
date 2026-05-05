use std::collections::HashMap;
use std::ptr::NonNull;

use {destack_engine as engine, destack_mir as mir};

use super::{ArgumentRange, Instruction, MovePair, MoveRange, WordLayout, word_layout_from_type};
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
    /// Callable target by function id.
    target_by_id: HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
    /// Callable environment word layout by function id.
    environment_layout_by_id: HashMap<mir::LocalNodeId<mir::Function>, WordLayout>,
    /// Call signature by function id.
    signature_by_function_id: HashMap<mir::LocalNodeId<mir::Function>, FunctionSignature>,
    /// Bare signature type by type id.
    signature_by_type_id: HashMap<mir::LocalNodeId<mir::Type>, FunctionSignature>,
}

impl FunctionTable {
    /// Build a lowered function table from lowered functions and callable targets.
    pub(super) fn new(
        tree: &mir::Tree,
        functions: Vec<Function>,
        target_by_id: HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
    ) -> Self {
        let environment_layout_by_id = environment_layouts(tree, &target_by_id);
        let signature_by_function_id = function_signatures(tree, &target_by_id);
        let signature_by_type_id = signature_types(tree);

        Self {
            functions,
            target_by_id,
            environment_layout_by_id,
            signature_by_function_id,
            signature_by_type_id,
        }
    }

    /// Resolve the callable target for the given function id.
    pub(crate) fn resolve(&self, func_id: mir::LocalNodeId<mir::Function>) -> Option<CallTarget> {
        self.target_by_id.get(&func_id).copied()
    }

    /// Resolve a lowered function index for the given id.
    pub(crate) fn index_for(&self, func_id: mir::LocalNodeId<mir::Function>) -> Option<u32> {
        match self.resolve(func_id)? {
            CallTarget::Local(index) => Some(index),
            CallTarget::Import => None,
        }
    }

    /// Return a lowered function pointer by dense index.
    pub(crate) fn pointer(&self, index: u32) -> Option<NonNull<Function>> {
        self.functions.get(index as usize).map(NonNull::from)
    }

    /// Resolve one lowered function pointer by function id.
    pub(crate) fn pointer_for(
        &self,
        func_id: mir::LocalNodeId<mir::Function>,
    ) -> Option<NonNull<Function>> {
        let index = self.index_for(func_id)?;
        self.pointer(index)
    }

    /// Return one lowered function by function id.
    pub(crate) fn function_for(
        &self,
        func_id: mir::LocalNodeId<mir::Function>,
    ) -> Option<&Function> {
        let index = self.index_for(func_id)?;

        self.functions.get(index as usize)
    }

    /// Return the callable environment word layout for one function id.
    pub(crate) fn environment_layout(
        &self,
        func_id: mir::LocalNodeId<mir::Function>,
    ) -> Option<WordLayout> {
        self.environment_layout_by_id.get(&func_id).copied()
    }

    /// Require one function to match one bare signature type.
    pub(crate) fn validate_signature(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
        signature: mir::LocalNodeId<mir::Type>,
    ) -> Result<(), Error> {
        let expected = self
            .signature_by_type_id
            .get(&signature)
            .ok_or(Error::InvalidInstruction)?;
        let actual =
            self.signature_by_function_id
                .get(&function_id)
                .ok_or(Error::UndefinedFunction {
                    function: function_id,
                })?;

        if actual == expected {
            return Ok(());
        }

        Err(Error::TypeMismatch {
            expected: format!("function signature {signature:?}"),
            actual: format!("function {function_id:?}"),
        })
    }
}

/// One compiled function call signature.
#[derive(Clone, Debug, PartialEq, Eq)]
struct FunctionSignature {
    /// The parameter types.
    parameters: Box<[mir::TypeReference]>,
    /// The result type.
    result: mir::TypeReference,
}

/// Build callable environment layouts for all callable function ids.
fn environment_layouts(
    tree: &mir::Tree,
    target_by_id: &HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
) -> HashMap<mir::LocalNodeId<mir::Function>, WordLayout> {
    let mut layouts = HashMap::new();

    for function_id in target_by_id.keys().copied() {
        let function = tree.get(function_id);
        let Some(environment) = function.environment else {
            continue;
        };
        let Some(environment_type) = environment.ty() else {
            continue;
        };

        let layout = match word_layout_from_type(tree, environment_type) {
            Some(layout) => layout,
            None => WordLayout::HeapReference,
        };
        layouts.insert(function_id, layout);
    }

    layouts
}

/// Build function signatures for callable function ids.
fn function_signatures(
    tree: &mir::Tree,
    target_by_id: &HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
) -> HashMap<mir::LocalNodeId<mir::Function>, FunctionSignature> {
    let mut signatures = HashMap::new();

    for function_id in target_by_id.keys().copied() {
        let function = tree.get(function_id);
        let parameters = function
            .parameters
            .iter()
            .map(|parameter| parameter.ty)
            .collect::<Vec<_>>()
            .into_boxed_slice();

        signatures.insert(
            function_id,
            FunctionSignature {
                parameters,
                result: function.return_type,
            },
        );
    }

    signatures
}

/// Build bare signature type metadata.
fn signature_types(tree: &mir::Tree) -> HashMap<mir::LocalNodeId<mir::Type>, FunctionSignature> {
    let mut signatures = HashMap::new();

    for (type_id, ty) in tree.iter_nodes::<mir::Type>() {
        let mir::Type::FunctionSignature { parameters, result } = ty else {
            continue;
        };
        let parameters = parameters.clone().into_boxed_slice();

        signatures.insert(
            type_id,
            FunctionSignature {
                parameters,
                result: *result,
            },
        );
    }

    signatures
}

/// Program call target for one function id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CallTarget {
    /// The function id names one imported callable.
    Import,
    /// The function id names one lowered callable.
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
    /// Source MIR boundary for each lowered PC in this block.
    pub source_boundary_by_pc: Vec<u32>,
}

/// Instruction bytes emitted for one block during lowering.
#[derive(Clone, Debug)]
pub(crate) struct BlockCode {
    /// Original MIR block id.
    pub mir_block: mir::LocalNodeId<mir::Block>,
    /// Instructions including terminator.
    pub instructions: Vec<Instruction>,
    /// Source MIR boundary for each lowered PC in this block.
    pub source_boundary_by_pc: Vec<u32>,
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
