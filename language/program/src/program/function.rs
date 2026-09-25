use serde::{Deserialize, Serialize};
use tspp_core::{
    EntryRange, EntryStore, Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice,
    StringId,
};
use tspp_serde::Reflect;

use crate::Error;

use super::{Symbol, TypeId, Word};

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
    SectionEntry,
)]
pub struct FunctionId(pub u32);

impl FunctionId {
    /// The callable word bias after null and undefined.
    pub const WORD_BIAS: u64 = 2;

    /// Return this id as a dense table index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// Decode one non-null callable value.
    pub const fn from_word(word: Word) -> Option<Self> {
        let Some(id) = word.bits().checked_sub(Self::WORD_BIAS) else {
            return None;
        };
        if id > u32::MAX as u64 {
            return None;
        }

        Some(Self(id as u32))
    }

    /// Encode this id as one non-null callable value.
    pub const fn word(self) -> Word {
        Word::from_bits(self.0 as u64 + Self::WORD_BIAS)
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

impl From<FunctionId> for Word {
    /// Encode one callable identity word.
    fn from(function: FunctionId) -> Self {
        function.word()
    }
}

/// Function table carried by one durable program.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct FunctionTable {
    /// Stable symbols keyed by program function id.
    symbols: SectionSlice<Symbol>,
    /// Dense signature entries keyed by signature id.
    signatures: SectionSlice<SignatureEntry>,
    /// Dense function entries keyed by program function id.
    functions: SectionSlice<Function>,
    /// Flattened function parameter types.
    parameters: SectionSlice<TypeId>,
    /// Exported function names.
    exports: SectionSlice<FunctionExport>,
}

impl FunctionTable {
    /// Return one stable function symbol.
    pub fn symbol(&self, sections: SectionImage<'_>, function: FunctionId) -> Option<Symbol> {
        sections
            .entries(self.symbols)
            .get(function.index())
            .copied()
    }

    /// Return one function entry.
    pub fn get<'a>(
        &self,
        sections: SectionImage<'a>,
        function: FunctionId,
    ) -> Option<&'a Function> {
        self.entries(sections).get(function.index())
    }

    /// Return one callable signature entry.
    pub fn signature<'a>(
        &self,
        sections: SectionImage<'a>,
        signature: SignatureId,
    ) -> Option<&'a SignatureEntry> {
        sections.entries(self.signatures).get(signature.index())
    }

    /// Expand one callable signature into its owned form.
    pub fn expand_signature(
        &self,
        sections: SectionImage<'_>,
        signature: SignatureId,
    ) -> Option<Signature> {
        let signature = self.signature(sections, signature)?;

        Some(Signature {
            parameters: self.parameters(sections, signature).to_vec(),
            result: signature.result,
        })
    }

    /// Resolve one exported function id by name.
    pub fn id_by_name(&self, sections: SectionImage<'_>, name: StringId) -> Option<FunctionId> {
        sections
            .entries(self.exports)
            .iter()
            .find_map(|export| (export.name == name).then_some(export.function))
    }

    /// Check that one function matches one call signature.
    pub fn check_signature<I>(
        &self,
        sections: SectionImage<'_>,
        function: FunctionId,
        result: TypeId,
        parameters: I,
    ) -> Result<(), Error>
    where
        I: Clone + Iterator<Item = TypeId>,
    {
        let function_id = function;
        let function = self
            .get(sections, function)
            .ok_or_else(|| Error::undefined_function(function))?;
        let signature = self
            .signature(sections, function.signature)
            .ok_or_else(|| Error::undefined_signature(function.signature))?;
        let actual_parameters = self.parameters(sections, signature);

        if signature.result == result && actual_parameters.iter().copied().eq(parameters.clone()) {
            return Ok(());
        }

        let expected = Signature {
            parameters: parameters.collect(),
            result,
        };
        let actual = Signature {
            parameters: actual_parameters.to_vec(),
            result: signature.result,
        };

        Err(Error::function_signature_mismatch(
            function_id,
            expected,
            actual,
        ))
    }

    /// Return all function entries.
    pub fn entries<'a>(&self, sections: SectionImage<'a>) -> &'a [Function] {
        sections.entries(self.functions)
    }

    /// Return one signature's parameter types.
    pub fn parameters<'a>(
        &self,
        sections: SectionImage<'a>,
        signature: &SignatureEntry,
    ) -> &'a [TypeId] {
        signature
            .parameters
            .slice(sections.entries(self.parameters))
    }

    /// Return whether every signature range fits the parameter column.
    pub(super) fn ranges_fit(&self, sections: SectionImage<'_>) -> bool {
        let parameters = sections.entries(self.parameters).len();

        sections
            .entries(self.signatures)
            .iter()
            .all(|signature| signature.parameters.fits(parameters))
    }
}

/// Exported function name.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FunctionExport {
    /// Exported function name.
    pub name: StringId,
    /// Exported function id.
    pub function: FunctionId,
    /// Reserved export word.
    reserved: u32,
}

impl FunctionExport {
    /// Create one exported function name.
    pub const fn new(name: StringId, function: FunctionId) -> Self {
        Self {
            name,
            function,
            reserved: 0,
        }
    }
}

/// Program function entry.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Function {
    /// The source-facing function name.
    pub name: StringId,
    /// Captured closure environment type when one exists.
    pub environment: Optional<TypeId>,
    /// Function call signature.
    pub signature: SignatureId,
    /// Reserved function bytes.
    reserved: [u8; 4],
}

impl Function {
    /// Return the closure environment type when one exists.
    pub fn environment(&self) -> Option<TypeId> {
        self.environment.get()
    }
}

/// Durable id for one callable signature.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct SignatureId(pub u32);

impl SignatureId {
    /// Return this id as a dense table index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Packed callable signature entry.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct SignatureEntry {
    /// Parameter types.
    pub parameters: EntryRange<TypeId>,
    /// Return type.
    pub result: TypeId,
    /// Reserved signature word.
    reserved: u32,
}

/// Owned logical runtime signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Signature {
    /// Parameter types.
    pub parameters: Vec<TypeId>,
    /// Return type.
    pub result: TypeId,
}

/// Build-time function table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FunctionTableBuilder {
    /// Stable symbols in dense function id order.
    symbols: Vec<Symbol>,
    /// Callable signatures in dense id order.
    signatures: Vec<Signature>,
    /// Function entries in dense id order.
    functions: Vec<FunctionBuilder>,
    /// Exported function names.
    exports: Vec<FunctionExport>,
}

impl FunctionTableBuilder {
    /// Create an empty function table builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set callable signatures in dense id order.
    pub fn signatures(mut self, signatures: impl IntoIterator<Item = Signature>) -> Self {
        self.signatures = signatures.into_iter().collect();

        self
    }

    /// Set function entries in dense id order.
    pub fn functions(
        mut self,
        functions: impl IntoIterator<Item = (Symbol, FunctionBuilder)>,
    ) -> Self {
        (self.symbols, self.functions) = functions.into_iter().unzip();

        self
    }

    /// Set exported function names.
    pub fn exports(mut self, exports: impl IntoIterator<Item = FunctionExport>) -> Self {
        self.exports = exports.into_iter().collect();

        self
    }

    /// Build this function table into program sections.
    pub(crate) fn build(self, sections: &mut SectionBuilder) -> FunctionTable {
        let mut signatures = Vec::with_capacity(self.signatures.len());
        let mut functions = Vec::with_capacity(self.functions.len());
        let mut parameters = EntryStore::new();

        // flatten variable signature payloads
        for signature in self.signatures {
            signatures.push(SignatureEntry {
                parameters: parameters.append(signature.parameters),
                result: signature.result,
                reserved: 0,
            });
        }

        // build fixed function entries
        for function in self.functions {
            functions.push(Function {
                name: function.name,
                environment: function.environment.into(),
                signature: function.signature,
                reserved: [0; 4],
            });
        }

        FunctionTable {
            symbols: sections.insert(self.symbols),
            signatures: sections.insert(signatures),
            functions: sections.insert(functions),
            parameters: sections.insert(parameters.into_entries()),
            exports: sections.insert(self.exports),
        }
    }
}

const _: () = assert!(size_of::<FunctionTable>() == 80);
const _: () = assert!(size_of::<FunctionExport>() == 16);
const _: () = assert!(size_of::<Function>() == 24);
const _: () = assert!(size_of::<SignatureId>() == 4);
const _: () = assert!(size_of::<SignatureEntry>() == 16);

/// Build-time function entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionBuilder {
    /// The source-facing function name.
    name: StringId,
    /// Function call signature.
    signature: SignatureId,
    /// Captured closure environment type when one exists.
    environment: Option<TypeId>,
}

impl FunctionBuilder {
    /// Create one function entry builder.
    pub fn new(name: StringId, signature: SignatureId) -> Self {
        Self {
            name,
            signature,
            environment: None,
        }
    }

    /// Set the captured closure environment type.
    pub fn environment(mut self, environment: TypeId) -> Self {
        self.environment = Some(environment);

        self
    }
}
