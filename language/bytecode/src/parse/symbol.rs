use std::collections::HashMap;

use crate::{ConstantId, FunctionId, GlobalId, TypeId, ValueType};

/// Symbols indexed before bytecode declarations are parsed.
#[derive(Debug, Default)]
pub(super) struct SymbolTable {
    /// Runtime type symbols.
    pub(super) types: HashMap<String, TypeId>,
    /// Indexed function declarations in object order.
    pub(super) function_declarations: Vec<FunctionDeclaration>,
    /// Globals declared by this object.
    pub(super) globals: HashMap<String, GlobalId>,
    /// Immutable constants declared by this object.
    pub(super) constants: HashMap<String, ConstantId>,
    /// Functions declared by this object.
    pub(super) functions: HashMap<String, FunctionId>,
}

/// One indexed function awaiting body parsing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FunctionDeclaration {
    /// The physical parameter types.
    pub(super) parameters: Vec<ValueType>,
    /// The physical result types.
    pub(super) results: Vec<ValueType>,
    /// The hidden closure environment type when present.
    pub(super) environment: Option<ValueType>,
    /// The parameters delivered when the function resumes.
    pub(super) resume_parameters: Vec<ValueType>,
}
