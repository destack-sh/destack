use super::member::MemberResolution;
use crate::Compiler;
use destack_dir::{
    DispatchKey, GlobalNodeIdAny, GlobalSymbolId, LocalInstanceId, LocalTypeId, Resolution,
    ResolutionCandidate, TypeTable,
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
        types: &mut TypeTable,
    ) {
        let candidate = ResolutionCandidate {
            key: None,
            target_symbol,
            instance: instance_id,
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
        candidate_symbols: Vec<GlobalSymbolId>,
        types: &mut TypeTable,
    ) {
        let candidates = candidate_symbols
            .into_iter()
            .map(|symbol| ResolutionCandidate {
                key: None,
                target_symbol: symbol,
                instance: None,
            })
            .collect();
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
                        types,
                    );
                }
                MemberResolution::Dynamic { symbols } => {
                    self.record_dynamic_resolution(node_id, receiver_ty_id, symbols.clone(), types);
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
