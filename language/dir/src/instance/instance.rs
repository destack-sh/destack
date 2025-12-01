use std::fmt::Display;

use destack_source::StringId;

use crate::{GlobalSymbolId, LocalTypeId, ModuleId, ScalarLiteral, SymbolKey};

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

/// An Instance is an instantiation of a statically parameterized type.
#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    /// The id of the Instance.
    pub id: LocalInstanceId,
    /// The symbol we're instantiating.
    pub symbol_id: GlobalSymbolId,
    /// The static arguments to the instance.
    pub static_arguments: Vec<StaticArgument>,
}

/// Static form of an expression.
#[derive(Debug, Clone, PartialEq)]
pub enum StaticExpression {
    /// Type expression.
    Type { type_id: LocalTypeId },
    /// Scalar literal.
    ScalarLiteral { value: ScalarLiteral },
    /// Range literal.
    RangeLiteral {
        start: Box<StaticExpression>,
        end: Box<StaticExpression>,
        is_inclusive: bool,
    },
    /// Array literal.
    ArrayLiteral { elements: Vec<Box<StaticArgument>> },
    /// Tuple literal.
    TupleLiteral { elements: Vec<Box<StaticArgument>> },
}

/// Static argument.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticArgument {
    /// The name.
    pub name: Option<StringId>,
    /// The target symbol.
    pub target_symbol: GlobalSymbolId,
    /// The static expression.
    pub value: Box<StaticExpression>,
}

/// Static property.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticProperty {
    /// The name.
    pub key: SymbolKey,
}
