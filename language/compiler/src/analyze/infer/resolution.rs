use crate::Compiler;
use destack_dir::{
    DispatchKey, GlobalNodeIdAny, GlobalSymbolId, LocalInstanceId, LocalTypeId, NodeTree,
    Resolution, ResolutionCandidate, StaticKey, SymbolTable, Type, TypeTable,
};
use destack_workspace::Module;

/// Describe the resolution outcome for a member lookup.
#[derive(Debug, Clone)]
pub(super) enum MemberResolution {
    /// No resolution is recorded for this lookup.
    None,
    /// A single target symbol is selected.
    Static { symbol: GlobalSymbolId },
    /// Multiple target symbols must be dispatched at runtime.
    Dynamic { symbols: Vec<GlobalSymbolId> },
    /// A nominal lookup failed, but some candidates exist.
    Unresolved,
}

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

    /// Resolve member symbols for a receiver type when nominal dispatch is possible.
    pub(super) fn resolve_member_resolution(
        &self,
        module: &Module,
        receiver_ty: &Type,
        member_key: &StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> MemberResolution {
        match receiver_ty {
            Type::Reference { .. } => {
                let mut visited = Vec::new();
                let member_symbol = self.resolve_member_symbol_for_type(
                    module,
                    receiver_ty,
                    member_key,
                    tree,
                    symbols,
                    types,
                    &mut visited,
                );

                if let Some(symbol) = member_symbol {
                    MemberResolution::Static { symbol }
                } else {
                    MemberResolution::Unresolved
                }
            }
            Type::Union { elements } => {
                let mut symbols_for_union = Vec::new();

                for element_id in elements {
                    let element_ty = types.get_type(*element_id).clone();
                    let mut visited = Vec::new();
                    let member_symbol = self.resolve_member_symbol_for_type(
                        module,
                        &element_ty,
                        member_key,
                        tree,
                        symbols,
                        types,
                        &mut visited,
                    );

                    let Some(member_symbol) = member_symbol else {
                        return MemberResolution::None;
                    };

                    if !symbols_for_union.contains(&member_symbol) {
                        symbols_for_union.push(member_symbol);
                    }
                }

                match symbols_for_union.len() {
                    0 => MemberResolution::None,
                    1 => MemberResolution::Static {
                        symbol: symbols_for_union[0],
                    },
                    _ => MemberResolution::Dynamic {
                        symbols: symbols_for_union,
                    },
                }
            }
            _ => MemberResolution::None,
        }
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
