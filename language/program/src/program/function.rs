use destack_serde::Reflect;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::vm::Error;

use super::TypeId;

/// Durable runtime function id inside one program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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

/// Executable function table carried by one durable program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionTable {
    /// Dense function records keyed by program function id.
    functions: Vec<Option<Function>>,
    /// Function ids keyed by exported source name.
    function_by_name: HashMap<String, FunctionId>,
}

impl FunctionTable {
    /// Create one executable function table.
    pub fn new(
        functions: Vec<Option<Function>>,
        function_by_name: HashMap<String, FunctionId>,
    ) -> Self {
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

    /// Require one function to match one bare signature type.
    pub fn validate_signature(
        &self,
        function: FunctionId,
        signature: &Signature,
    ) -> Result<(), Error> {
        let function = self
            .get(function)
            .ok_or_else(|| Error::undefined_function(function))?;

        if function.matches_signature(signature) {
            return Ok(());
        }

        Err(Error::type_mismatch(
            format!("function signature {signature:?}"),
            format!("function {function:?}"),
        ))
    }
}

/// Executable function record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Function {
    /// The source-facing function name.
    pub name: String,
    /// Function call signature.
    pub signature: Signature,
    /// Captured closure environment type when one exists.
    pub environment: Option<TypeId>,
}

impl Function {
    /// Return whether this function matches one function signature type.
    fn matches_signature(&self, signature: &Signature) -> bool {
        if self.signature.parameters.len() != signature.parameters.len() {
            return false;
        }

        let parameters_match = self
            .signature
            .parameters
            .iter()
            .zip(signature.parameters.iter())
            .all(|(actual, expected)| actual == expected);

        parameters_match && self.signature.result == signature.result
    }
}

/// Executable callable signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Signature {
    /// Parameter types.
    pub parameters: Vec<TypeId>,
    /// Return type.
    pub result: TypeId,
}
