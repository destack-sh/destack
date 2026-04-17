use super::*;

/// Describe the resolution outcome for a member lookup.
#[derive(Debug, Clone)]
pub(crate) enum MemberResolution {
    /// No resolution is recorded for this lookup.
    None,
    /// A single target symbol is selected.
    Static { symbol: GlobalSymbolId },
    /// Multiple target symbols must be dispatched at runtime.
    Dynamic {
        candidates: Vec<MemberResolutionCandidate>,
    },
    /// A nominal lookup failed, but some candidates exist.
    Unresolved,
}

/// Candidate member target for dynamic union dispatch.
#[derive(Debug, Clone)]
pub(crate) struct MemberResolutionCandidate {
    /// The receiver type to dispatch against.
    pub(crate) receiver_ty_id: LocalTypeId,
    /// The resolved member symbol for this receiver type.
    pub(crate) symbol: GlobalSymbolId,
}

/// Resolved extension metadata for a member lookup.
#[derive(Debug, Clone)]
pub(crate) struct ExtensionMemberContext {
    /// The resolved static arguments for the extension parameters.
    pub(crate) arguments: Vec<StaticArgument>,
    /// The substitutions for extension type parameters.
    pub(crate) substitutions: HashMap<GlobalSymbolId, LocalTypeId>,
}

/// Select which members are visible during lookup.
#[derive(Debug, Copy, Clone)]
pub(crate) enum MemberLookupMode {
    /// Look up instance members only.
    Instance,
    /// Look up static members only.
    Value,
    /// Look up members without filtering.
    Any,
}

/// Visibility metadata for a member symbol.
#[derive(Debug, Clone)]
pub(crate) struct MemberVisibilityContext {
    /// The visibility of the member symbol.
    pub(crate) visibility: Visibility,
    /// The owner symbol of the member symbol.
    pub(crate) owner_symbol: GlobalSymbolId,
}

/// Metadata for a constructor parameter-property projected as a member.
#[derive(Debug, Clone)]
pub(crate) struct ParameterPropertyMemberContext {
    /// The visibility of the projected property.
    pub(crate) visibility: Visibility,
    /// Whether the projected property is readonly.
    pub(crate) is_readonly: bool,
    /// The class symbol that declares the property.
    pub(crate) owner_symbol: GlobalSymbolId,
}

/// Classification metadata for a member receiver expression.
#[derive(Debug, Copy, Clone)]
pub(crate) struct MemberReceiverContext {
    /// The nominal symbol when the receiver is a type-like value.
    pub(crate) nominal_symbol: Option<GlobalSymbolId>,
    /// Whether the receiver expression carries explicit static arguments.
    pub(crate) has_static_arguments: bool,
    /// Whether the receiver is context-sensitive because it carries `this`.
    pub(crate) has_this_receiver: bool,
    /// The lookup mode to use for type-driven member inference.
    pub(crate) lookup_mode: MemberLookupMode,
}

/// Resolved function signature instantiation for member static arguments.
#[derive(Debug, Clone)]
pub(crate) struct InstantiatedMemberSignature {
    /// The instantiated function type.
    pub(crate) type_id: LocalTypeId,
    /// The static arguments selected for the signature.
    pub(crate) generic_arguments: Vec<StaticArgument>,
    /// The signature static parameter symbols in resolved order.
    pub(crate) generic_parameter_symbols: Vec<GlobalSymbolId>,
}

/// Resolved member access type for one symbol lookup path.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedMemberAccessType {
    /// The resolved member type id after substitution and static-argument application.
    pub(crate) type_id: LocalTypeId,
    /// The resolved static arguments applied to this member access.
    pub(crate) generic_arguments: Vec<StaticArgument>,
    /// The signature static parameter symbols in resolved order.
    pub(crate) generic_parameter_symbols: Vec<GlobalSymbolId>,
}

/// Prepared receiver metadata for member access inference.
#[derive(Debug, Clone)]
pub(crate) struct MemberAccessReceiver {
    /// The effective receiver expression after optional-chain normalization.
    pub(crate) receiver_id: LocalNodeId<Expression>,
    /// The inferred receiver type after materialization and normalization.
    pub(crate) receiver_ty_id: LocalTypeId,
    /// The resolved receiver type.
    pub(crate) receiver_ty: Type,
    /// Receiver classification used for downstream member lookup and diagnostics.
    pub(crate) receiver_context: MemberReceiverContext,
    /// Whether the receiver type still depends on infer convergence.
    pub(crate) receiver_requires_infer_convergence: bool,
    /// Whether missing-member diagnostics may defer while inference converges.
    pub(crate) allow_missing_member_deferral: bool,
    /// Whether indeterminate receiver member checks should emit unknown-based diagnostics immediately.
    pub(crate) force_unknown_receiver_diagnostic: bool,
    /// Whether optional chaining introduced nullish receivers.
    pub(crate) has_optional_nullish: bool,
}

/// Receiver query result for member access inference.
#[derive(Debug, Clone)]
pub(crate) enum MemberAccessReceiverQuery {
    /// The receiver short-circuits to a concrete type.
    EarlyType(LocalTypeId),
    /// The receiver is ready for normal member lookup.
    Receiver(MemberAccessReceiver),
}

/// Resolved lookup state for member access inference.
#[derive(Debug, Clone)]
pub(crate) struct MemberAccessLookup {
    /// The resolved member lookup key.
    pub(crate) member_key: StaticKey,
    /// Receiver classification used for member lookup.
    pub(crate) receiver_context: MemberReceiverContext,
    /// Resolved member dispatch mode.
    pub(crate) resolution: MemberResolution,
    /// The resolved concrete member symbol when one exists.
    pub(crate) member_symbol: Option<GlobalSymbolId>,
    /// The enum field value type for enum member symbols.
    pub(crate) enum_field_value_ty_id: Option<LocalTypeId>,
    /// Inherited static argument metadata from the receiver.
    pub(crate) inherited: InheritedStaticArguments,
    /// Resolved extension metadata for the member symbol.
    pub(crate) extension_context: Option<ExtensionMemberContext>,
    /// Effective substitutions merged from inherited and extension contexts.
    pub(crate) substitutions: HashMap<GlobalSymbolId, LocalTypeId>,
}
