use destack_dir as dir;

use crate::check::{CheckState, ReceiverCapture, ReceiverResolution};

impl CheckState<'_> {
    /// Resolve `this` at the current walk point.
    pub(in crate::check) fn resolve_this_receiver(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<ReceiverCapture> {
        let module = source.module_id;
        if let Some((index, receiver)) = self.flow(module).lexical_receiver() {
            let is_current = self.flow(module).is_current_function(index);

            if is_current {
                self.record_this_receiver_resolution(source, receiver);
            } else {
                self.flow_mut(module).capture_receiver(receiver);
                self.record_name_resolution(source, receiver.symbol);
            }

            return Some(receiver);
        }

        if let Some(receiver) = self.flow(module).current_receiver() {
            self.record_this_receiver_resolution(source, receiver);

            return Some(receiver);
        }

        None
    }

    /// Record one lexical value reference for capture analysis.
    pub(in crate::check) fn capture_symbol_reference(
        &mut self,
        module: destack_source::ModuleId,
        symbol: dir::GlobalSymbolId,
    ) {
        if symbol.module_id != module {
            return;
        }
        if !self.inputs.contains_key(&module) {
            return;
        }
        if self.is_import_symbol(module, symbol) {
            return;
        }
        let Some(function) = self.flow(module).current_function() else {
            return;
        };
        if symbol == function.symbol {
            return;
        }
        if self.symbol_is_module_scoped(module, symbol) {
            return;
        }
        if self.symbol_is_owned_by_function(module, symbol, function.symbol) {
            return;
        }

        self.flow_mut(module).capture_symbol(symbol);
    }

    /// Return whether one symbol is declared in the module scope.
    fn symbol_is_module_scoped(
        &self,
        module: destack_source::ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> bool {
        let bindings = self.input(module).binding_table();
        let symbol = bindings.get_symbol(symbol.local_id);
        let scope = bindings.get_scope(symbol.scope);

        scope.parent.is_none()
    }

    /// Return whether one symbol is declared under one function source node.
    fn symbol_is_owned_by_function(
        &self,
        module: destack_source::ModuleId,
        symbol: dir::GlobalSymbolId,
        function: dir::GlobalSymbolId,
    ) -> bool {
        if symbol.module_id != module || function.module_id != module {
            return false;
        }
        let bindings = self.input(module).binding_table();
        let symbol = bindings.get_symbol(symbol.local_id);
        let mut scope = Some(symbol.scope.id);

        // walk lexical scope owners
        while let Some(scope_id) = scope {
            let current = bindings.get_scope_by_id(scope_id);
            if current.owner == Some(function.local_id) {
                return true;
            }

            scope = current.parent.map(|parent| parent.id);
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
            self.record_name_resolution(source, receiver.symbol);

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
