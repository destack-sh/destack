use std::fmt::Display;

use destack_source::ModuleId;

use crate::{GlobalSymbolId, StaticArgument};

/// Unique identifier for Instances.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    /// The symbol being instantiated.
    pub symbol_id: GlobalSymbolId,
    /// Static arguments, flattened: `[inherited..., own...]`.
    pub static_arguments: Vec<StaticArgument>,
}

impl Instance {
    /// Create a new instance with the given symbol and arguments.
    pub fn new(symbol_id: GlobalSymbolId, arguments: Vec<StaticArgument>) -> Self {
        Self {
            symbol_id,
            static_arguments: arguments,
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
