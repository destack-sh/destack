use serde::{Deserialize, Serialize};
use tspp_core::StringId;
use tspp_mir as mir;
use tspp_serde::Reflect;

/// One object-local function declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Function {
    /// The module-local MIR function identity.
    pub id: mir::FunctionId,
    /// The source-facing function name.
    pub name: StringId,
    /// The persistent linkable function identity.
    pub symbol: mir::Symbol,
    /// The function linkage.
    pub linkage: mir::Linkage,
    /// The function lifetime parameters in declaration order.
    pub lifetimes: Vec<mir::LifetimeParameter>,
    /// The function parameter types in declaration order.
    pub parameters: Vec<mir::SignatureParameter>,
    /// The function result type.
    pub result: mir::TypeId,
    /// The hidden environment type when present.
    pub environment: Option<mir::TypeId>,
    /// The runtime binding declaration when present.
    pub binding: Option<Box<mir::Binding>>,
}

impl Function {
    /// Return whether this function is imported.
    pub fn is_import(&self) -> bool {
        self.linkage.is_import()
    }

    /// Return the runtime binding name when present.
    pub const fn binding_name(&self) -> Option<StringId> {
        match &self.binding {
            Some(binding) => Some(binding.name),
            None => None,
        }
    }
}
