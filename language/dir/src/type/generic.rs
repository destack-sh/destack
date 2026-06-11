use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, StringId, VarianceModifier};

/// Unique identifier for generic templates.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalGenericTemplateId(pub u32);

impl LocalGenericTemplateId {
    /// Wrap an id as a local generic template id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Turn into a global generic template id.
    pub fn into_global(self, module_id: ModuleId) -> GlobalGenericTemplateId {
        GlobalGenericTemplateId {
            module_id,
            local_id: self,
        }
    }
}

/// Global generic template id across modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GlobalGenericTemplateId {
    /// The module id of the global generic template.
    pub module_id: ModuleId,
    /// The local generic template id.
    pub local_id: LocalGenericTemplateId,
}

impl GlobalGenericTemplateId {
    /// Create a new global generic template id.
    pub fn new(module_id: ModuleId, local_id: LocalGenericTemplateId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Turn into a local generic template id.
    pub fn into_local(self) -> LocalGenericTemplateId {
        self.local_id
    }
}

impl From<GlobalGenericTemplateId> for LocalGenericTemplateId {
    fn from(id: GlobalGenericTemplateId) -> Self {
        id.local_id
    }
}

/// Unique identifier for generic parameters.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalGenericParameterId(pub u32);

impl LocalGenericParameterId {
    /// Wrap an id as a local generic parameter id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Turn into a global generic parameter id.
    pub fn into_global(self, module_id: ModuleId) -> GlobalGenericParameterId {
        GlobalGenericParameterId {
            module_id,
            local_id: self,
        }
    }
}

/// Global generic parameter id across modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GlobalGenericParameterId {
    /// The module id of the global generic parameter.
    pub module_id: ModuleId,
    /// The local generic parameter id.
    pub local_id: LocalGenericParameterId,
}

impl GlobalGenericParameterId {
    /// Create a new global generic parameter id.
    pub fn new(module_id: ModuleId, local_id: LocalGenericParameterId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Turn into a local generic parameter id.
    pub fn into_local(self) -> LocalGenericParameterId {
        self.local_id
    }
}

impl From<GlobalGenericParameterId> for LocalGenericParameterId {
    fn from(id: GlobalGenericParameterId) -> Self {
        id.local_id
    }
}

/// Source that introduced one generic parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenericParameterOrigin {
    /// The parameter was written in source.
    Explicit,
    /// The parameter was induced by check.
    Induced(GenericParameterInduction),
}

/// Reason one generic parameter was induced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenericParameterInduction {
    /// A parameter type induced a hidden constrained type parameter.
    ParameterConstraint,
    /// A storage type induced a hidden constrained type parameter.
    StorageConstraint,
    /// A type form parameter escaped.
    Form,
    /// A comptime runtime parameter was lifted.
    Comptime,
}

/// User-visible key of one generic parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GenericParameterKey {
    /// Explicit source symbol.
    Symbol(GlobalSymbolId),
    /// Generated checked parameter key.
    Generated(StringId),
}

/// One declaration of generic parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenericTemplate {
    /// The source node that declares this template.
    pub source: GlobalNodeIdAny,
    /// The immediately enclosing generic template.
    pub parent: Option<LocalGenericTemplateId>,
    /// The generic parameters in declaration order.
    pub parameters: Vec<LocalGenericParameterId>,
}

impl GenericTemplate {
    /// Create an empty generic template for one source node.
    pub fn new(source: GlobalNodeIdAny, parent: Option<LocalGenericTemplateId>) -> Self {
        Self {
            source,
            parent,
            parameters: Vec::new(),
        }
    }
}

/// One declaration-side generic parameter.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GenericParameterBinding {
    /// The generic template that owns this parameter.
    pub template: LocalGenericTemplateId,
    /// The parameter key.
    pub key: GenericParameterKey,
    /// The parameter variance, rejected on comptime parameters.
    pub variance: Option<VarianceModifier>,
    /// The optional constraint.
    pub constraint: Option<GlobalTypeId>,
    /// The optional default.
    pub default: Option<GlobalTypeId>,
    /// The parameter origin.
    pub origin: GenericParameterOrigin,
    /// Whether the parameter captures remaining arguments.
    pub is_variadic: bool,
    /// Whether arguments must solve to singleton types.
    pub is_comptime: bool,
}
