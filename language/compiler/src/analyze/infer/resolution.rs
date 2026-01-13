use super::member::MemberResolution;
use crate::Compiler;
use destack_dir::{
    DispatchKey, GlobalNodeIdAny, GlobalSymbolId, LocalInstanceId, LocalTypeId, Resolution,
    ResolutionCandidate, ResolvedSignature, TypeTable,
};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Record a builtin resolution for a node.
    pub(super) fn record_builtin_resolution(
        &self,
        node_id: GlobalNodeIdAny,
        receiver_ty_id: Option<LocalTypeId>,
        types: &mut TypeTable,
    ) {
        let resolution = Resolution::Builtin {
            receiver: receiver_ty_id,
        };

        let resolution_id = types.insert_resolution(resolution);
        types.set_resolution_for_node(node_id, resolution_id);
    }

    /// Record a static resolution for a node.
    pub(super) fn record_static_resolution(
        &self,
        node_id: GlobalNodeIdAny,
        receiver_ty_id: Option<LocalTypeId>,
        target_symbol: GlobalSymbolId,
        instance_id: Option<LocalInstanceId>,
        resolved_signature: Option<ResolvedSignature>,
        types: &mut TypeTable,
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

        let resolution_id = types.insert_resolution(resolution);
        types.set_resolution_for_node(node_id, resolution_id);
    }

    /// Record a dynamic resolution for a node.
    pub(super) fn record_dynamic_resolution(
        &self,
        node_id: GlobalNodeIdAny,
        receiver_ty_id: Option<LocalTypeId>,
        candidates: Vec<ResolutionCandidate>,
        types: &mut TypeTable,
    ) {
        let resolution = Resolution::Dynamic {
            receiver: receiver_ty_id,
            candidates,
        };

        let resolution_id = types.insert_resolution(resolution);
        types.set_resolution_for_node(node_id, resolution_id);
    }

    /// Record the resolution for a member lookup.
    pub(super) fn record_member_resolution(
        &self,
        node_id: GlobalNodeIdAny,
        receiver_ty_id: Option<LocalTypeId>,
        resolution: &MemberResolution,
        instance_id: Option<LocalInstanceId>,
        resolved_signature: Option<ResolvedSignature>,
        has_member: bool,
        types: &mut TypeTable,
    ) {
        if has_member {
            match resolution {
                MemberResolution::Static { symbol } => {
                    self.record_static_resolution(
                        node_id,
                        receiver_ty_id,
                        *symbol,
                        instance_id,
                        resolved_signature,
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
                    self.record_dynamic_resolution(node_id, receiver_ty_id, candidates, types);
                }
                MemberResolution::Unresolved | MemberResolution::None => {}
            }
        } else {
            self.record_unresolved_resolution(
                node_id,
                receiver_ty_id,
                Vec::new(),
                Vec::new(),
                types,
            );
        }
    }

    /// Record an unresolved resolution for a node.
    pub(super) fn record_unresolved_resolution(
        &self,
        node_id: GlobalNodeIdAny,
        receiver_ty_id: Option<LocalTypeId>,
        missing_keys: Vec<DispatchKey>,
        candidates: Vec<ResolutionCandidate>,
        types: &mut TypeTable,
    ) {
        let resolution = Resolution::Unresolved {
            receiver: receiver_ty_id,
            missing_keys,
            candidates,
        };

        let resolution_id = types.insert_resolution(resolution);
        types.set_resolution_for_node(node_id, resolution_id);
    }
}
