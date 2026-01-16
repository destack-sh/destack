use destack_base::StringId;
use destack_dir::{ExportKind, GlobalSymbolId, StaticKey, SymbolKind, SymbolSpace, SymbolType};
use destack_source::{ModuleId, ModuleVersion};
use indexmap::IndexMap;

use crate::{ProfileId, ProfileVersion};

/// Unique key for a module signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleSignatureKey {
    /// The module id.
    pub module_id: ModuleId,
    /// The profile id.
    pub profile_id: ProfileId,
}

impl ModuleSignatureKey {
    /// Create a new module signature key.
    pub fn new(module_id: ModuleId, profile_id: ProfileId) -> Self {
        Self {
            module_id,
            profile_id,
        }
    }
}

/// Summary of a module's exported surface for a profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleSignature {
    /// The module id.
    pub module_id: ModuleId,
    /// The profile id.
    pub profile_id: ProfileId,
    /// The module version used to compute this signature.
    pub module_version: ModuleVersion,
    /// The profile version used to compute this signature.
    pub profile_version: ProfileVersion,
    /// The stable hash for the signature.
    pub hash: u64,
    /// Export signatures in stable order.
    pub exports: Vec<ModuleSignatureExport>,
    /// Export assignment signature when present.
    pub export_assignment_hash: Option<u64>,
    /// Signature hashes for exportable symbols.
    pub symbol_signatures: IndexMap<GlobalSymbolId, u64>,
    /// Global augmentation signatures.
    pub global_augmentations: Vec<ModuleSignatureAugmentation>,
    /// Module binding signatures.
    pub module_bindings: Vec<ModuleSignatureBinding>,
}

#[allow(clippy::too_many_arguments)]
impl ModuleSignature {
    /// Create a new module signature with the provided data.
    pub fn new(
        module_id: ModuleId,
        profile_id: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
        hash: u64,
        exports: Vec<ModuleSignatureExport>,
        export_assignment_hash: Option<u64>,
        symbol_signatures: IndexMap<GlobalSymbolId, u64>,
        global_augmentations: Vec<ModuleSignatureAugmentation>,
        module_bindings: Vec<ModuleSignatureBinding>,
    ) -> Self {
        Self {
            module_id,
            profile_id,
            module_version,
            profile_version,
            hash,
            exports,
            export_assignment_hash,
            symbol_signatures,
            global_augmentations,
            module_bindings,
        }
    }
}

/// Export signature data for a single export entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleSignatureExport {
    /// The export key.
    pub key: StaticKey,
    /// The export symbol space.
    pub space: SymbolSpace,
    /// The export kind.
    pub kind: ExportKind,
    /// The resolved target symbol, if any.
    pub target: Option<GlobalSymbolId>,
    /// The declared symbol kind, if any.
    pub symbol_kind: Option<SymbolKind>,
    /// The declared symbol type, if any.
    pub symbol_type: Option<SymbolType>,
    /// The hashed signature for the export's shape.
    pub signature_hash: Option<u64>,
}

/// Signature data for a global augmentation symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleSignatureAugmentation {
    /// The symbol key when available.
    pub key: Option<StaticKey>,
    /// The symbol space.
    pub space: SymbolSpace,
    /// The symbol kind.
    pub symbol_kind: SymbolKind,
    /// The symbol type.
    pub symbol_type: SymbolType,
    /// The hashed signature for the symbol shape.
    pub signature_hash: Option<u64>,
}

/// Signature data for a `declare module` binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleSignatureBinding {
    /// The module specifier string.
    pub specifier: StringId,
    /// Export signatures for the binding.
    pub exports: Vec<ModuleSignatureExport>,
    /// Export assignment signature hash when present.
    pub export_assignment_hash: Option<u64>,
}
