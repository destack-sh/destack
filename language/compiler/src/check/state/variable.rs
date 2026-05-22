use destack_dir as dir;
use destack_source::ModuleId;

/// Component-valid id for one check variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in crate::check) struct VariableId {
    /// The module that owns the variable.
    pub(in crate::check) module: ModuleId,
    /// The variable index inside the owning module.
    pub(in crate::check) index: u32,
}

impl VariableId {
    /// Create one variable id.
    pub(in crate::check) fn new(module: ModuleId, index: u32) -> Self {
        Self { module, index }
    }
}

/// One check variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct Variable {
    /// The variable id.
    pub(in crate::check) id: VariableId,
    /// The variable kind.
    pub(in crate::check) kind: VariableKind,
    /// The source that produced the variable.
    pub(in crate::check) origin: VariableOrigin,
    /// The solved value, when known.
    pub(in crate::check) value: Option<VariableValue>,
}

impl Variable {
    /// Create one unsolved variable.
    pub(in crate::check) fn new(
        id: VariableId,
        kind: VariableKind,
        origin: VariableOrigin,
    ) -> Self {
        Self {
            id,
            kind,
            origin,
            value: None,
        }
    }
}

/// The value space of one check variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum VariableKind {
    /// Type variable.
    Type,
    /// Static value variable.
    Static,
    /// Layout variable.
    Layout,
    /// Resolution variable.
    Resolution,
}

/// Solved value for one check variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum VariableValue {
    /// Solved type id.
    Type(dir::LocalTypeId),
    /// Solved static value id.
    Static(dir::LocalStaticId),
    /// Solved layout id.
    Layout(dir::LocalLayoutId),
    /// Solved resolution value.
    Resolution(ResolutionValue),
}

/// Solved resolution payload.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ResolutionValue {
    /// Resolved symbol.
    Symbol(dir::GlobalSymbolId),
    /// Resolved name.
    Name(dir::NameResolution),
    /// Resolved call.
    Call(dir::CallResolution),
    /// Resolved member.
    Member(dir::MemberResolution),
}

/// Source that produced one variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum VariableOrigin {
    /// Variable attached to a source node.
    Node(dir::GlobalNodeIdAny),
    /// Variable attached to a source symbol.
    Symbol(dir::GlobalSymbolId),
    /// Variable for a source type expression.
    TypeExpression(dir::GlobalNodeId<dir::TypeExpression>),
    /// Variable for a source static expression.
    StaticExpression(dir::GlobalNodeId<dir::Expression>),
    /// Variable for a layout request.
    Layout { ty: dir::LocalTypeId },
    /// Variable introduced by solver rules.
    Synthetic,
}
