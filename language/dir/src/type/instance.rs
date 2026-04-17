use destack_source::AdaptImage;
use std::collections::HashSet;
use std::fmt::Display;

use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, StaticArgument};

/// Unique identifier for Instances.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, AdaptImage,
)]
pub struct LocalInstanceId(pub u32);

impl LocalInstanceId {
    /// Wrap an id as a InstanceId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Turn into a GlobalInstanceId.
    pub fn into_global(self, module_id: ModuleId) -> GlobalInstanceId {
        GlobalInstanceId {
            module_id,
            local_id: self,
        }
    }
}

/// Global instance id across modules.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, AdaptImage,
)]
pub struct GlobalInstanceId {
    /// The module id of the global instance.
    pub module_id: ModuleId,
    /// The local id of the global instance.
    pub local_id: LocalInstanceId,
}

impl GlobalInstanceId {
    /// Create a new global instance id.
    pub fn new(module_id: ModuleId, local_id: LocalInstanceId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Turn into a LocalInstanceId.
    pub fn into_local(self) -> LocalInstanceId {
        self.local_id
    }
}

impl From<GlobalInstanceId> for LocalInstanceId {
    fn from(id: GlobalInstanceId) -> Self {
        id.local_id
    }
}

/// Errors emitted when instance environment metadata is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceEnvironmentError {
    /// Parameter symbol count does not match argument count.
    ParameterArgumentCountMismatch,
    /// Inherited argument count exceeds total argument count.
    InheritedArgumentCountOutOfRange,
    /// Parameter symbols contain duplicates.
    DuplicateParameterSymbol,
    /// Merge metadata parameter symbols do not match instance symbols.
    ParameterSymbolMismatch,
    /// Merge metadata inherited argument count conflicts with existing metadata.
    InheritedArgumentCountConflict,
}

impl Display for LocalInstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// An Instance is a concrete instantiation of a statically parameterized ("generic") declaration.
/// Basically, an Instance is a monomorphized variant of a polymorphic DIR declaration.
///
/// Arguments are stored **flattened**: inherited arguments first (from enclosing generic
/// contexts), then own arguments (declared by this symbol).
/// This matches how Rust and C++ handle monomorphization, where each instance
///  is self-contained with all type arguments it needs.
///
/// ### Example
///
/// ```text
/// struct Container<T> {
///     value: T
///     map<U>(f: (T) => U): Container<U> { ... }
/// }
///
/// let c: Container<int32> = ...;
/// c.map<string>(f)
/// ```
///
/// The instances created are:
/// - `Container<int32>` → `{ symbol: Container, arguments: [int32] }`
/// - `Container<int32>.map<string>` → `{ symbol: map, arguments: [int32, string] }`
///
/// For `map`, `int32` is inherited from `Container<T>` and `string` is `map`'s own `U`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub struct Instance {
    /// The symbol being instantiated.
    pub symbol_id: GlobalSymbolId,
    /// Static arguments, flattened: `[inherited..., own...]`.
    pub generic_arguments: Vec<StaticArgument>,
    /// Parameter symbols aligned one-to-one with `generic_arguments`.
    pub generic_parameter_symbols: Vec<GlobalSymbolId>,
    /// Number of inherited arguments at the head of `generic_arguments`.
    pub inherited_static_argument_count: usize,
}

impl Instance {
    /// Create a new instance with explicit parameter symbols.
    pub fn new(
        symbol_id: GlobalSymbolId,
        arguments: Vec<StaticArgument>,
        parameter_symbols: Vec<GlobalSymbolId>,
    ) -> Result<Self, InstanceEnvironmentError> {
        Self::with_environment(symbol_id, arguments, parameter_symbols, 0)
    }

    /// Create a new instance with explicit substitution-environment metadata.
    pub fn with_environment(
        symbol_id: GlobalSymbolId,
        arguments: Vec<StaticArgument>,
        parameter_symbols: Vec<GlobalSymbolId>,
        inherited_argument_count: usize,
    ) -> Result<Self, InstanceEnvironmentError> {
        if parameter_symbols.len() != arguments.len() {
            return Err(InstanceEnvironmentError::ParameterArgumentCountMismatch);
        }
        if inherited_argument_count > arguments.len() {
            return Err(InstanceEnvironmentError::InheritedArgumentCountOutOfRange);
        }
        if !parameter_symbols_are_unique(&parameter_symbols) {
            return Err(InstanceEnvironmentError::DuplicateParameterSymbol);
        }

        Ok(Self {
            symbol_id,
            generic_arguments: arguments,
            generic_parameter_symbols: parameter_symbols,
            inherited_static_argument_count: inherited_argument_count,
        })
    }

    /// Return inherited arguments from the canonical flattened list.
    pub fn inherited_arguments(&self) -> &[StaticArgument] {
        let split = self.inherited_static_argument_count;
        assert!(
            split <= self.generic_arguments.len(),
            "invalid inherited argument count"
        );
        &self.generic_arguments[..split]
    }

    /// Return own arguments from the canonical flattened list.
    pub fn own_arguments(&self) -> &[StaticArgument] {
        let split = self.inherited_static_argument_count;
        assert!(
            split <= self.generic_arguments.len(),
            "invalid inherited argument count"
        );
        &self.generic_arguments[split..]
    }

    /// Return the argument bound to one parameter symbol when known.
    pub fn argument_for_parameter_symbol(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<&StaticArgument> {
        self.generic_parameter_symbols
            .iter()
            .position(|parameter| *parameter == symbol_id)
            .map(|index| &self.generic_arguments[index])
    }

    /// Merge known environment metadata into this instance.
    pub fn merge_environment_metadata(
        &mut self,
        parameter_symbols: &[GlobalSymbolId],
        inherited_argument_count: usize,
    ) -> Result<(), InstanceEnvironmentError> {
        if parameter_symbols != self.generic_parameter_symbols {
            return Err(InstanceEnvironmentError::ParameterSymbolMismatch);
        }
        if inherited_argument_count > self.generic_arguments.len() {
            return Err(InstanceEnvironmentError::InheritedArgumentCountOutOfRange);
        }
        if inherited_argument_count < self.inherited_static_argument_count {
            return Err(InstanceEnvironmentError::InheritedArgumentCountConflict);
        }

        self.inherited_static_argument_count = inherited_argument_count;
        Ok(())
    }

    /// Split arguments into inherited and own using canonical instance metadata.
    /// Returns `(inherited, own)`.
    pub fn split_arguments(&self) -> (&[StaticArgument], &[StaticArgument]) {
        (self.inherited_arguments(), self.own_arguments())
    }
}

/// Return whether parameter symbols are unique.
fn parameter_symbols_are_unique(parameter_symbols: &[GlobalSymbolId]) -> bool {
    let mut seen = HashSet::with_capacity(parameter_symbols.len());
    parameter_symbols
        .iter()
        .copied()
        .all(|symbol| seen.insert(symbol))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LocalSymbolId, LocalTypeId, StaticExpression, SymbolType};

    /// Build one stable test symbol id.
    fn test_symbol(id: u32) -> GlobalSymbolId {
        LocalSymbolId::new_typed(id, SymbolType::Void).into_global(ModuleId::EPHEMERAL)
    }

    /// Build one stable static type argument.
    fn test_type_argument(id: u32) -> StaticArgument {
        StaticArgument::value(StaticExpression::Type {
            ty: LocalTypeId::new(id),
        })
    }

    /// Reject duplicate parameter symbols during instance construction.
    #[test]
    fn test_instance_rejects_duplicate_parameter_symbols() {
        let symbol_id = test_symbol(1);
        let parameter_symbol = test_symbol(2);
        let arguments = vec![test_type_argument(1), test_type_argument(2)];
        let parameter_symbols = vec![parameter_symbol, parameter_symbol];

        let result = Instance::with_environment(symbol_id, arguments, parameter_symbols, 1);
        assert_eq!(
            result,
            Err(InstanceEnvironmentError::DuplicateParameterSymbol)
        );
    }

    /// Reject mismatched parameter and argument lengths during construction.
    #[test]
    fn test_instance_rejects_parameter_argument_count_mismatch() {
        let symbol_id = test_symbol(1);
        let arguments = vec![test_type_argument(1)];
        let parameter_symbols = vec![test_symbol(2), test_symbol(3)];

        let result = Instance::with_environment(symbol_id, arguments, parameter_symbols, 0);
        assert_eq!(
            result,
            Err(InstanceEnvironmentError::ParameterArgumentCountMismatch)
        );
    }

    /// Split inherited and own arguments using instance-owned metadata.
    #[test]
    fn test_instance_split_arguments_uses_inherited_count() {
        let symbol_id = test_symbol(1);
        let arguments = vec![
            test_type_argument(1),
            test_type_argument(2),
            test_type_argument(3),
        ];
        let parameter_symbols = vec![test_symbol(2), test_symbol(3), test_symbol(4)];
        let instance = Instance::with_environment(symbol_id, arguments, parameter_symbols, 2)
            .expect("expected valid instance");

        let (inherited, own) = instance.split_arguments();
        assert_eq!(inherited.len(), 2);
        assert_eq!(own.len(), 1);
    }

    /// Reject metadata merges that regress inherited-argument count.
    #[test]
    fn test_instance_merge_rejects_inherited_argument_regression() {
        let symbol_id = test_symbol(1);
        let parameter_symbols = vec![test_symbol(2), test_symbol(3)];
        let arguments = vec![test_type_argument(1), test_type_argument(2)];
        let mut instance =
            Instance::with_environment(symbol_id, arguments, parameter_symbols.clone(), 1)
                .expect("expected valid instance");

        let result = instance.merge_environment_metadata(&parameter_symbols, 0);
        assert_eq!(
            result,
            Err(InstanceEnvironmentError::InheritedArgumentCountConflict)
        );
    }
}
