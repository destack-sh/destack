use serde::{Deserialize, Serialize};
use tspp_core::StringId;
use tspp_serde::Reflect;
use tspp_source::ModuleId;

use crate::{GenericArgument, GenericParameter, Tree, TypeId};

use super::fingerprint::TypeHasher;

/// Persistent identity of a function, global, or type.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct Symbol {
    /// The module declaring the symbol, none for a declaration of the language.
    module: Option<ModuleId>,
    /// The stable bits of the mangled name.
    hash: u64,
}

impl Symbol {
    /// Create the symbol of one resolved declaration name.
    pub fn named(module: ModuleId, name: StringId) -> Self {
        Self {
            module: Some(module),
            hash: name.raw(),
        }
    }

    /// Create the symbol of one declaration the language makes in every module.
    pub fn language(name: StringId) -> Self {
        Self {
            module: None,
            hash: name.raw(),
        }
    }

    /// Create the symbol of one declaration, distinguished by its declaring identity.
    pub fn declared(module: ModuleId, name: StringId, identity: u64) -> Self {
        TypeHasher::declared(module, name, identity)
    }

    /// Derive one generic instance symbol from its concrete arguments.
    pub fn instantiate(self, arguments: &[GenericArgument], tree: &Tree) -> Self {
        TypeHasher::symbol(self, arguments, tree)
    }

    /// Create a generated function symbol from its signature and selected application.
    pub fn generated(
        base: Self,
        signature: TypeId,
        parameters: &[GenericParameter],
        receiver: Option<&GenericArgument>,
        arguments: &[GenericArgument],
        tree: &Tree,
    ) -> Self {
        TypeHasher::generated(base, signature, parameters, receiver, arguments, tree)
    }

    /// Return the module declaring the symbol, none for a declaration of the language.
    pub fn module(self) -> Option<ModuleId> {
        self.module
    }

    /// Return the module declaring one symbol outside the language's own declarations.
    pub fn declaring_module(self) -> ModuleId {
        self.module
            .unwrap_or_else(|| unreachable!("a declared symbol names its declaring module"))
    }

    /// Return whether one module's tree defines the symbol itself.
    pub fn is_defined_in(self, module: ModuleId) -> bool {
        self.module.is_none_or(|declaring| declaring == module)
    }

    /// Return the stable symbol bits.
    pub fn raw(self) -> u64 {
        self.hash
    }

    /// Restore a symbol from its declaring module and persistent bits.
    pub const fn from_raw(module: Option<ModuleId>, raw: u64) -> Self {
        Self { module, hash: raw }
    }
}
