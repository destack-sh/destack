use destack_dir as dir;

/// Interface protocol required by a generated operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct Protocol {
    /// The protocol interface symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The protocol generic arguments.
    pub(in crate::check) arguments: Vec<dir::GlobalTypeId>,
}

impl Protocol {
    /// Return one protocol interface instance.
    pub(in crate::check) fn new(
        symbol: dir::GlobalSymbolId,
        arguments: Vec<dir::GlobalTypeId>,
    ) -> Self {
        Self { symbol, arguments }
    }

    /// Return this protocol as a generic instance.
    pub(in crate::check) fn instance(&self) -> dir::GenericInstance {
        dir::GenericInstance {
            symbol: self.symbol,
            arguments: self.arguments.clone(),
        }
    }
}

/// Selected protocol member.
pub(in crate::check) struct ProtocolMember {
    /// The selected member resolution.
    pub(in crate::check) resolution: dir::MemberResolution,
}

/// Selected protocol call.
pub(in crate::check) struct ProtocolCall {
    /// The selected call resolution.
    pub(in crate::check) resolution: dir::CallResolution,
    /// The selected call return type.
    pub(in crate::check) return_type: dir::GlobalTypeId,
}
