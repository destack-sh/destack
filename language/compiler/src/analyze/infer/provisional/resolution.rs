use crate::Compiler;
use crate::analyze::infer::member::MemberResolution;
use destack_dir::{
    DispatchKey, GlobalNodeIdAny, GlobalSymbolId, InferTable, LocalInstanceId, LocalTypeId,
    Resolution, ResolutionCandidate, ResolvedSignature, TypeTable,
};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Record a builtin resolution for a node.
    pub(crate) fn record_provisional_builtin_resolution(
        &self,
        node_id: GlobalNodeIdAny,
        receiver_ty_id: Option<LocalTypeId>,
        infer: &mut InferTable,
        _types: &mut TypeTable,
    ) {
        let resolution = Resolution::Builtin {
            receiver: receiver_ty_id,
        };
        infer.set_provisional_resolution_for_node(node_id, resolution);
    }

    /// Record a static resolution for a node.
    pub(crate) fn record_provisional_static_resolution(
        &self,
        node_id: GlobalNodeIdAny,
        receiver_ty_id: Option<LocalTypeId>,
        target_symbol: GlobalSymbolId,
        instance_id: Option<LocalInstanceId>,
        resolved_signature: Option<ResolvedSignature>,
        infer: &mut InferTable,
        _types: &mut TypeTable,
    ) {
        let candidate = ResolutionCandidate {
            key: None,
            target_symbol,
            instance: instance_id,
            resolved_signature,
        };
        let resolution = Resolution::Static {
            receiver: receiver_ty_id,
            candidate,
        };
        infer.set_provisional_resolution_for_node(node_id, resolution);
    }

    /// Record a dynamic resolution for a node.
    pub(crate) fn record_provisional_dynamic_resolution(
        &self,
        node_id: GlobalNodeIdAny,
        receiver_ty_id: Option<LocalTypeId>,
        candidates: Vec<ResolutionCandidate>,
        infer: &mut InferTable,
        _types: &mut TypeTable,
    ) {
        let resolution = Resolution::Dynamic {
            receiver: receiver_ty_id,
            candidates,
        };
        infer.set_provisional_resolution_for_node(node_id, resolution);
    }

    /// Record the resolution for a member lookup.
    pub(crate) fn record_provisional_member_resolution(
        &self,
        node_id: GlobalNodeIdAny,
        receiver_ty_id: Option<LocalTypeId>,
        resolution: &MemberResolution,
        instance_id: Option<LocalInstanceId>,
        resolved_signature: Option<ResolvedSignature>,
        has_member: bool,
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) {
        // TODO #Architecture: store instantiation context on resolution entries
        if has_member {
            match resolution {
                MemberResolution::Static { symbol } => {
                    self.record_provisional_static_resolution(
                        node_id,
                        receiver_ty_id,
                        *symbol,
                        instance_id,
                        resolved_signature,
                        infer,
                        types,
                    );
                }
                MemberResolution::Dynamic { candidates } => {
                    let candidates = candidates
                        .iter()
                        .map(|candidate| ResolutionCandidate {
                            key: Some(DispatchKey::single(candidate.receiver_ty_id)),
                            target_symbol: candidate.symbol,
                            instance: None,
                            resolved_signature: None,
                        })
                        .collect();
                    self.record_provisional_dynamic_resolution(
                        node_id,
                        receiver_ty_id,
                        candidates,
                        infer,
                        types,
                    );
                }
                MemberResolution::Unresolved | MemberResolution::None => {
                    self.record_provisional_unresolved_resolution(
                        node_id,
                        receiver_ty_id,
                        Vec::new(),
                        Vec::new(),
                        infer,
                        types,
                    );
                }
            }
        } else {
            self.record_provisional_unresolved_resolution(
                node_id,
                receiver_ty_id,
                Vec::new(),
                Vec::new(),
                infer,
                types,
            );
        }
    }

    /// Record an unresolved resolution for a node.
    pub(crate) fn record_provisional_unresolved_resolution(
        &self,
        node_id: GlobalNodeIdAny,
        receiver_ty_id: Option<LocalTypeId>,
        missing_keys: Vec<DispatchKey>,
        candidates: Vec<ResolutionCandidate>,
        infer: &mut InferTable,
        _types: &mut TypeTable,
    ) {
        let resolution = Resolution::Unresolved {
            receiver: receiver_ty_id,
            missing_keys,
            candidates,
        };
        infer.set_provisional_resolution_for_node(node_id, resolution);
    }
}
