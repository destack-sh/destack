use std::fmt::Display;

use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    GlobalStaticId, GlobalSymbolId, GlobalTypeId, StaticArgument, StringId, VarianceModifier,
};

/// Unique identifier for generic templates.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalGenericTemplateId(pub u32);

impl LocalGenericTemplateId {
    /// Wrap an id as a local generic template id.
    pub fn new(id: u32) -> Self {
        Self(id)
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

/// Unique identifier for generic instances.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalGenericInstanceId(pub u32);

impl LocalGenericInstanceId {
    /// Wrap an id as a local generic instance id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Turn into a global generic instance id.
    pub fn into_global(self, module_id: ModuleId) -> GlobalGenericInstanceId {
        GlobalGenericInstanceId {
            module_id,
            local_id: self,
        }
    }
}

/// Global generic instance id across modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GlobalGenericInstanceId {
    /// The module id of the global generic instance.
    pub module_id: ModuleId,
    /// The local generic instance id.
    pub local_id: LocalGenericInstanceId,
}

impl GlobalGenericInstanceId {
    /// Create a new global generic instance id.
    pub fn new(module_id: ModuleId, local_id: LocalGenericInstanceId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Turn into a local generic instance id.
    pub fn into_local(self) -> LocalGenericInstanceId {
        self.local_id
    }
}

impl From<GlobalGenericInstanceId> for LocalGenericInstanceId {
    fn from(id: GlobalGenericInstanceId) -> Self {
        id.local_id
    }
}

impl Display for LocalGenericInstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
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
    /// A transparent type constraint escaped.
    Constraint,
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

/// One owner-level declaration of generic parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenericTemplate {
    /// The symbol that owns this generic template.
    pub owner: GlobalSymbolId,
    /// The generic parameters in declaration order.
    pub parameters: Vec<LocalGenericParameterId>,
}

impl GenericTemplate {
    /// Create an empty generic template for one owner.
    pub fn new(owner: GlobalSymbolId) -> Self {
        Self {
            owner,
            parameters: Vec::new(),
        }
    }
}

/// One declaration-side generic parameter.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GenericParameterBinding {
    /// Type generic parameter.
    Type {
        /// The generic template that owns this parameter.
        template: LocalGenericTemplateId,
        /// The parameter key.
        key: GenericParameterKey,
        /// The parameter variance.
        variance: Option<VarianceModifier>,
        /// The optional type constraint.
        constraint: Option<GlobalTypeId>,
        /// The optional type default.
        default: Option<GlobalTypeId>,
        /// The parameter origin.
        origin: GenericParameterOrigin,
    },
    /// Variadic type generic parameter.
    VariadicType {
        /// The generic template that owns this parameter.
        template: LocalGenericTemplateId,
        /// The parameter key.
        key: GenericParameterKey,
        /// The parameter variance.
        variance: Option<VarianceModifier>,
        /// The optional type constraint.
        constraint: Option<GlobalTypeId>,
        /// The optional type default.
        default: Option<GlobalTypeId>,
        /// The parameter origin.
        origin: GenericParameterOrigin,
    },
    /// Static generic parameter.
    Static {
        /// The generic template that owns this parameter.
        template: LocalGenericTemplateId,
        /// The parameter key.
        key: GenericParameterKey,
        /// The optional static value type constraint.
        constraint: Option<GlobalTypeId>,
        /// The optional static default.
        default: Option<GlobalStaticId>,
        /// The parameter origin.
        origin: GenericParameterOrigin,
    },
    /// Variadic static generic parameter.
    VariadicStatic {
        /// The generic template that owns this parameter.
        template: LocalGenericTemplateId,
        /// The parameter key.
        key: GenericParameterKey,
        /// The optional static value type constraint.
        constraint: Option<GlobalTypeId>,
        /// The optional static default.
        default: Option<GlobalStaticId>,
        /// The parameter origin.
        origin: GenericParameterOrigin,
    },
}

impl GenericParameterBinding {
    /// Return the owner generic template.
    pub fn template(&self) -> LocalGenericTemplateId {
        match self {
            Self::Type { template, .. }
            | Self::VariadicType { template, .. }
            | Self::Static { template, .. }
            | Self::VariadicStatic { template, .. } => *template,
        }
    }

    /// Return the parameter key.
    pub fn key(&self) -> GenericParameterKey {
        match self {
            Self::Type { key, .. }
            | Self::VariadicType { key, .. }
            | Self::Static { key, .. }
            | Self::VariadicStatic { key, .. } => *key,
        }
    }

    /// Return the parameter origin.
    pub fn origin(&self) -> GenericParameterOrigin {
        match self {
            Self::Type { origin, .. }
            | Self::VariadicType { origin, .. }
            | Self::Static { origin, .. }
            | Self::VariadicStatic { origin, .. } => *origin,
        }
    }
}

/// One concrete instance of static arguments to a generic template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericInstance {
    /// The generic template being applied.
    pub template: LocalGenericTemplateId,
    /// The static arguments in declaration order.
    pub arguments: Vec<StaticArgument>,
}

impl GenericInstance {
    /// Create a generic instance.
    pub fn new(template: LocalGenericTemplateId, arguments: Vec<StaticArgument>) -> Self {
        Self {
            template,
            arguments,
        }
    }
}
