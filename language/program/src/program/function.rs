use destack_core::{
    EntryRange, EntryStore, Optional, SectionEntry, SectionImage, SectionPacker, SectionSlice,
    StringId,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::vm::Error;

use super::TypeId;

/// Durable runtime function id inside one program.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
)]
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

/// Function table carried by one durable program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionTable {
    /// Dense function entries keyed by program function id.
    functions: SectionSlice<Optional<Function>>,
    /// Flattened function parameter types.
    parameters: SectionSlice<TypeId>,
    /// Exported function names.
    exports: SectionSlice<FunctionExport>,
}

impl FunctionTable {
    /// Pack one function table.
    pub fn pack(
        sections: &mut SectionPacker,
        functions: Vec<Option<FunctionBuilder>>,
        exports: Vec<FunctionExport>,
    ) -> Self {
        let mut entries = Vec::with_capacity(functions.len());
        let mut parameters = EntryStore::new();

        // flatten variable function payloads
        for function in functions {
            let entry = match function {
                Some(function) => {
                    let parameters = parameters.append(function.signature.parameters);
                    let function = Function {
                        name: function.name,
                        signature: FunctionSignature {
                            parameters,
                            result: function.signature.result,
                        },
                        environment: function.environment.into(),
                    };

                    Optional::some(function)
                }
                None => Optional::none(),
            };

            entries.push(entry);
        }

        let functions = sections.insert(entries);
        let parameters = sections.insert(parameters.into_entries());
        let exports = sections.insert(exports);

        Self {
            functions,
            parameters,
            exports,
        }
    }

    /// Return one function record.
    pub fn get<'a>(
        &self,
        sections: SectionImage<'a>,
        function: FunctionId,
    ) -> Option<&'a Function> {
        self.entries(sections)
            .get(function.index())
            .and_then(Optional::as_ref)
    }

    /// Resolve one exported function id by name.
    pub fn id_by_name(&self, sections: SectionImage<'_>, name: StringId) -> Option<FunctionId> {
        sections
            .entries(self.exports)
            .iter()
            .find_map(|export| (export.name == name).then_some(export.function))
    }

    /// Check that one function matches one bare signature type.
    pub fn check_signature(
        &self,
        sections: SectionImage<'_>,
        function: FunctionId,
        signature: &Signature,
    ) -> Result<(), Error> {
        let function_id = function;
        let function = self
            .get(sections, function)
            .ok_or_else(|| Error::undefined_function(function))?;

        if function.matches_signature(signature, sections.entries(self.parameters)) {
            return Ok(());
        }

        let actual = function.signature(sections.entries(self.parameters));

        Err(Error::function_signature_mismatch(
            function_id,
            signature.clone(),
            actual,
        ))
    }

    /// Check that one function matches one packed signature entry.
    pub fn check_signature_entry(
        &self,
        sections: SectionImage<'_>,
        function: FunctionId,
        signature: FunctionSignature,
        signature_parameters: &[TypeId],
    ) -> Result<(), Error> {
        let function_id = function;
        let function = self
            .get(sections, function)
            .ok_or_else(|| Error::undefined_function(function))?;

        if function.matches_signature_entry(
            signature,
            signature_parameters,
            sections.entries(self.parameters),
        ) {
            return Ok(());
        }

        let expected = Signature {
            parameters: signature_parameters.to_vec(),
            result: signature.result,
        };
        let actual = function.signature(sections.entries(self.parameters));

        Err(Error::function_signature_mismatch(
            function_id,
            expected,
            actual,
        ))
    }

    /// Return all function entries.
    pub fn entries<'a>(&self, sections: SectionImage<'a>) -> &'a [Optional<Function>] {
        sections.entries(self.functions)
    }

    /// Return this function's parameter types.
    pub fn parameters<'a>(&self, sections: SectionImage<'a>, function: &Function) -> &'a [TypeId] {
        function
            .signature
            .parameters
            .slice(sections.entries(self.parameters))
    }
}

/// Exported function name.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionExport {
    /// Exported function name.
    pub name: StringId,
    /// Exported function id.
    pub function: FunctionId,
}

/// Program function record.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Function {
    /// The source-facing function name.
    pub name: StringId,
    /// Function call signature.
    pub signature: FunctionSignature,
    /// Captured closure environment type when one exists.
    pub environment: Optional<TypeId>,
}

impl Function {
    /// Return this function's expanded signature.
    pub fn signature(&self, parameters: &[TypeId]) -> Signature {
        Signature {
            parameters: self.signature.parameters.slice(parameters).to_vec(),
            result: self.signature.result,
        }
    }

    /// Return whether this function matches one function signature type.
    fn matches_signature(&self, signature: &Signature, parameters: &[TypeId]) -> bool {
        let actual = self.signature(parameters);

        actual == *signature
    }

    /// Return whether this function matches one packed signature entry.
    fn matches_signature_entry(
        &self,
        signature: FunctionSignature,
        expected_parameters: &[TypeId],
        parameters: &[TypeId],
    ) -> bool {
        let actual = self.signature(parameters);

        actual.parameters == expected_parameters && actual.result == signature.result
    }

    /// Return the closure environment type when one exists.
    pub fn environment(&self) -> Option<TypeId> {
        self.environment.get()
    }
}

/// Packed function signature entry.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionSignature {
    /// Parameter types.
    pub parameters: EntryRange<TypeId>,
    /// Return type.
    pub result: TypeId,
}

/// Build-time callable signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Signature {
    /// Parameter types.
    pub parameters: Vec<TypeId>,
    /// Return type.
    pub result: TypeId,
}

/// Build-time function record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionBuilder {
    /// The source-facing function name.
    pub name: StringId,
    /// Function call signature.
    pub signature: Signature,
    /// Captured closure environment type when one exists.
    pub environment: Option<TypeId>,
}

// SAFETY: function ids, exports, signatures, and entries are fixed-width.
unsafe impl SectionEntry for FunctionId {}
unsafe impl SectionEntry for FunctionExport {}
unsafe impl SectionEntry for Function {}
unsafe impl SectionEntry for FunctionSignature {}
