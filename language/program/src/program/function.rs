use std::collections::HashMap;

use destack_core::StringPool;
use destack_mir as mir;
use serde::{Deserialize, Serialize};

use crate::vm::{CellLayout, Error};

use super::{ProgramIndex, TypeId, TypeTable};

/// Durable runtime function id inside one program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FunctionId(pub u32);

impl FunctionId {
    /// Return this id as a dense table index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl From<u32> for FunctionId {
    /// Convert one raw program function id.
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<FunctionId> for u32 {
    /// Convert one program function id into its raw value.
    fn from(id: FunctionId) -> Self {
        id.0
    }
}

/// Runtime function metadata carried by one durable program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionTable {
    /// Dense function records keyed by program function id.
    functions: Vec<Option<Function>>,
    /// Function ids keyed by exported source name.
    function_by_name: HashMap<String, FunctionId>,
}

impl FunctionTable {
    /// Build one runtime function table from lowered MIR metadata.
    pub fn lower(tree: &mir::Tree, strings: &StringPool, index: &ProgramIndex) -> Self {
        let mut functions = Vec::new();
        let mut function_by_name = HashMap::new();

        // copy function records into dense runtime slots
        for (mir_function, function) in tree.iter_nodes::<mir::Function>() {
            let function_id = index.function_id(mir_function);
            let slot = function_id.index();
            if slot >= functions.len() {
                functions.resize_with(slot + 1, || None);
            }

            let name = strings.get(function.name).to_string();
            let record = Function::lower(function, name.clone(), index);
            functions[slot] = Some(record);
            function_by_name.entry(name).or_insert(function_id);
        }

        Self {
            functions,
            function_by_name,
        }
    }

    /// Return one function record.
    pub fn get(&self, function: FunctionId) -> Option<&Function> {
        self.functions
            .get(function.index())
            .and_then(Option::as_ref)
    }

    /// Resolve one function id by source name.
    pub fn id_by_name(&self, name: &str) -> Option<FunctionId> {
        self.function_by_name.get(name).copied()
    }

    /// Return the function environment cell layout for one function id.
    pub fn environment_layout(
        &self,
        types: &TypeTable,
        pointer_bytes: u8,
        function: FunctionId,
    ) -> Option<CellLayout> {
        let function = self.get(function)?;
        let environment = function.environment?;

        types.cell_layout(environment, pointer_bytes)
    }

    /// Require one function to match one bare signature type.
    pub fn validate_signature(
        &self,
        types: &TypeTable,
        function: FunctionId,
        signature: TypeId,
    ) -> Result<(), Error> {
        let function = self
            .get(function)
            .ok_or_else(|| Error::undefined_function(function))?;

        let Some(mir::Type::FunctionSignature {
            parameters, result, ..
        }) = types.get(signature)
        else {
            return Err(Error::invalid_instruction());
        };

        if function.matches_signature(types, parameters, types.type_id(*result)) {
            return Ok(());
        }

        Err(Error::type_mismatch(
            format!("function signature {signature:?}"),
            format!("function {function:?}"),
        ))
    }
}

/// Runtime function metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Function {
    /// The source-facing function name.
    pub name: String,
    /// Function parameter types.
    pub parameters: Vec<TypeId>,
    /// Function return type.
    pub return_type: TypeId,
    /// Captured closure environment type when one exists.
    pub environment: Option<TypeId>,
}

impl Function {
    /// Build one runtime function record from MIR.
    fn lower(function: &mir::Function, name: String, index: &ProgramIndex) -> Self {
        let parameters = function
            .parameters
            .iter()
            .map(|parameter| index.type_id(parameter.ty))
            .collect();

        Self {
            name,
            parameters,
            return_type: index.type_id(function.return_type),
            environment: function.environment.map(|ty| index.type_id(ty)),
        }
    }

    /// Return whether this function matches one function signature type.
    fn matches_signature(
        &self,
        types: &TypeTable,
        parameters: &[mir::SignatureParameter],
        result: TypeId,
    ) -> bool {
        if self.parameters.len() != parameters.len() {
            return false;
        }

        let parameters_match = self
            .parameters
            .iter()
            .zip(parameters.iter())
            .all(|(actual, expected)| *actual == types.type_id(expected.ty));

        parameters_match && self.return_type == result
    }
}
