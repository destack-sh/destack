use destack_dir as dir;

use crate::check::{CheckModuleState, ReceiverCapture, ReceiverResolution};

impl CheckModuleState {
    /// Resolve `this` at the current walk point.
    pub(in crate::check) fn resolve_this_receiver(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<ReceiverCapture> {
        let (index, receiver) = self.work.flow.lexical_receiver()?;
        let is_current = self.work.flow.is_current_function(index);

        if is_current {
            self.record_this_receiver_resolution(source, receiver);
        } else {
            self.work.flow.capture_receiver(receiver);
            self.record_name_resolution(source, dir::NameResolution::new(receiver.symbol));
        }

        Some(receiver)
    }

    /// Record one lexical value reference for capture analysis.
    pub(in crate::check) fn capture_symbol_reference(&mut self, symbol: dir::GlobalSymbolId) {
        if symbol.module_id != self.input.module_id {
            return;
        }
        if self.is_import_symbol(symbol) {
            return;
        }
        let Some(function) = self.work.flow.current_function() else {
            return;
        };
        if symbol == function.symbol {
            return;
        }
        if self.symbol_is_module_scoped(symbol) {
            return;
        }
        if self.symbol_is_owned_by_function(symbol, function.symbol) {
            return;
        }

        self.work.flow.capture_symbol(symbol);
    }

    /// Return whether one symbol is declared in the module scope.
    fn symbol_is_module_scoped(&self, symbol: dir::GlobalSymbolId) -> bool {
        let bindings = self.binding_table();
        let symbol = bindings.get_symbol(symbol.local_id);
        let scope = bindings.get_scope(symbol.scope);

        scope.parent.is_none()
    }

    /// Return whether one symbol is declared under one function source node.
    fn symbol_is_owned_by_function(
        &self,
        symbol: dir::GlobalSymbolId,
        function: dir::GlobalSymbolId,
    ) -> bool {
        let Some(symbol_source) = self.symbol_source_node(symbol) else {
            return false;
        };
        let Some(function_source) = self.symbol_source_node(function) else {
            return false;
        };

        self.node_is_descendant_of(symbol_source, function_source)
    }

    /// Return whether one node is inside another source node.
    fn node_is_descendant_of(
        &self,
        node: dir::LocalNodeIdAny,
        ancestor: dir::LocalNodeIdAny,
    ) -> bool {
        let mut current = Some(node);
        let view = self.input.view();

        while let Some(node) = current {
            if node == ancestor {
                return true;
            }

            current = view.get_parent(node.id);
        }

        false
    }

    /// Record the contextual receiver selected by one `this` expression.
    fn record_this_receiver_resolution(
        &mut self,
        source: dir::GlobalNodeIdAny,
        receiver: ReceiverCapture,
    ) {
        let Some(owner) = receiver.owner else {
            self.record_name_resolution(source, dir::NameResolution::new(receiver.symbol));

            return;
        };
        let resolution = ReceiverResolution {
            source,
            kind: dir::ReceiverKind::This,
            owner: Some(owner),
            ty: receiver.ty,
        };

        self.record_receiver_resolution(resolution);
    }
}
