use std::fmt::Display;

use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, StaticArgument};

/// Unique identifier for Instances.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instance {
    /// The symbol being instantiated.
    pub symbol_id: GlobalSymbolId,
    /// Static arguments, flattened: `[inherited..., own...]`.
    pub static_arguments: Vec<StaticArgument>,
    /// Parameter symbols aligned one-to-one with `static_arguments`.
    pub static_parameter_symbols: Vec<GlobalSymbolId>,
    /// Number of inherited arguments at the head of `static_arguments`.
    pub inherited_static_argument_count: usize,
}

impl Instance {
    /// Create a new instance with explicit parameter symbols.
    pub fn new(
        symbol_id: GlobalSymbolId,
        arguments: Vec<StaticArgument>,
        parameter_symbols: Vec<GlobalSymbolId>,
    ) -> Option<Self> {
        Self::with_environment(symbol_id, arguments, parameter_symbols, 0)
    }

    /// Create a new instance with explicit substitution-environment metadata.
    pub fn with_environment(
        symbol_id: GlobalSymbolId,
        arguments: Vec<StaticArgument>,
        parameter_symbols: Vec<GlobalSymbolId>,
        inherited_argument_count: usize,
    ) -> Option<Self> {
        if parameter_symbols.len() != arguments.len() {
            return None;
        }
        if inherited_argument_count > arguments.len() {
            return None;
        }

        Some(Self {
            symbol_id,
            static_arguments: arguments,
            static_parameter_symbols: parameter_symbols,
            inherited_static_argument_count: inherited_argument_count,
        })
    }

    /// Return inherited arguments from the canonical flattened list.
    pub fn inherited_arguments(&self) -> &[StaticArgument] {
        let split = self
            .inherited_static_argument_count
            .min(self.static_arguments.len());
        &self.static_arguments[..split]
    }

    /// Return own arguments from the canonical flattened list.
    pub fn own_arguments(&self) -> &[StaticArgument] {
        let split = self
            .inherited_static_argument_count
            .min(self.static_arguments.len());
        &self.static_arguments[split..]
    }

    /// Return the argument bound to one parameter symbol when known.
    pub fn argument_for_parameter_symbol(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<&StaticArgument> {
        self.static_parameter_symbols
            .iter()
            .position(|parameter| *parameter == symbol_id)
            .map(|index| &self.static_arguments[index])
    }

    /// Merge known environment metadata into this instance.
    pub fn merge_environment_metadata(
        &mut self,
        parameter_symbols: &[GlobalSymbolId],
        inherited_argument_count: usize,
    ) {
        if parameter_symbols != self.static_parameter_symbols {
            return;
        }

        if inherited_argument_count > self.inherited_static_argument_count {
            self.inherited_static_argument_count =
                inherited_argument_count.min(self.static_arguments.len());
        }
    }

    /// Split arguments into inherited and own, given the count of own parameters.
    /// Returns `(inherited, own)`.
    pub fn split_arguments(
        &self,
        own_param_count: usize,
    ) -> (&[StaticArgument], &[StaticArgument]) {
        let split = self.static_arguments.len().saturating_sub(own_param_count);
        (
            &self.static_arguments[..split],
            &self.static_arguments[split..],
        )
    }
}
