use std::collections::HashMap;

use destack_source::Span;

use crate::{
    ConstantId, FunctionId, FunctionTypeId, GlobalId, ParseError, ParseResult, TypeId, ValueType,
};

/// Symbols indexed before bytecode declarations are parsed.
#[derive(Debug, Default)]
pub(super) struct SymbolTable {
    /// Runtime type symbols.
    pub(super) types: HashMap<String, TypeId>,
    /// Named callable types.
    pub(super) function_types: HashMap<String, FunctionTypeId>,
    /// Indexed function declarations in object order.
    pub(super) function_declarations: Vec<FunctionDeclaration>,
    /// Logical values selected by every callable type.
    pub(super) function_type_definitions: Vec<FunctionTypeDefinition>,
    /// Globals declared by this object.
    pub(super) globals: HashMap<String, GlobalId>,
    /// Immutable constants declared by this object.
    pub(super) constants: HashMap<String, ConstantId>,
    /// Functions declared by this object.
    pub(super) functions: HashMap<String, FunctionId>,
}

/// One function type collected before function bodies are parsed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FunctionTypeDefinition {
    /// The logical parameter types in call order.
    pub(super) parameters: Vec<ValueType>,
    /// The logical result types in return order.
    pub(super) results: Vec<ValueType>,
}

/// One indexed function awaiting body parsing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FunctionDeclaration {
    /// The canonical callable type.
    pub(super) function_type: FunctionTypeId,
    /// The values delivered when the function resumes.
    pub(super) resume: Vec<ValueType>,
}

impl SymbolTable {
    /// Insert one dense symbol and reject duplicate declarations.
    pub(super) fn insert<Id: Copy>(
        symbols: &mut HashMap<String, Id>,
        name: String,
        create: impl FnOnce(u32) -> Id,
        span: Span,
    ) -> ParseResult<()> {
        if symbols.contains_key(&name) {
            return Err(ParseError::new("duplicate symbol", span));
        }

        let id = create(symbols.len() as u32);
        symbols.insert(name, id);

        Ok(())
    }

    /// Return whether two function types have identical parameters and results.
    pub(super) fn function_types_match(&self, left: FunctionTypeId, right: FunctionTypeId) -> bool {
        self.function_type_definitions.get(left.index())
            == self.function_type_definitions.get(right.index())
    }
}
