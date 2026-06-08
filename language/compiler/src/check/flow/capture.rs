use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Receiver, ReceiverBinding, ReceiverResolution, WalkState};

impl WalkState<'_, '_> {
    /// Resolve `this` at the current walk point.
    pub(in crate::check) fn resolve_this_receiver(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<Receiver>> {
        // prefer receiver from an active function frame
        if let Some((index, receiver)) = self.flow().lexical_receiver() {
            let is_current = self.flow().is_current_function(index);

            // select local receiver directly
            if is_current {
                self.select_this_receiver_binding(source, receiver)?;
            }
            // capture receiver from an outer function
            else {
                self.flow_mut().capture_receiver(receiver);
                let resolution = dir::NameResolution::new(receiver.symbol);

                self.check.inference.select_name(source, resolution)?;
            }

            return Ok(Some(receiver.receiver));
        }

        // fall back to contextual receiver outside function bodies
        if let Some(receiver) = self.flow().current_receiver() {
            self.select_this_receiver(source, receiver)?;

            return Ok(Some(receiver));
        }

        Ok(None)
    }

    /// Capture one lexical value reference when required.
    pub(in crate::check) fn capture_symbol_reference(&mut self, symbol: dir::GlobalSymbolId) {
        // ignore references outside function bodies
        let Some(function) = self.flow().current_function() else {
            return;
        };

        // ignore references that do not cross a boundary
        if !self.is_captured_symbol_reference(symbol, function.symbol) {
            return;
        }

        // capture the outer symbol
        self.flow_mut().capture_symbol(symbol);
    }

    /// Return whether one value reference crosses a function boundary.
    fn is_captured_symbol_reference(
        &self,
        symbol: dir::GlobalSymbolId,
        function: dir::GlobalSymbolId,
    ) -> bool {
        symbol.module_id == self.module
            && !self
                .check
                .module(self.module)
                .is_import_alias(symbol.local_id)
            && symbol != function
            && !self.is_module_scoped_symbol(symbol)
            && !self.is_symbol_owned_by_function(symbol, function)
    }

    /// Return whether one symbol is declared in the module scope.
    fn is_module_scoped_symbol(&self, symbol: dir::GlobalSymbolId) -> bool {
        // read lexical scope for the symbol
        let bindings = self.check.module(self.module).binding_table();
        let symbol = bindings.get_symbol(symbol.local_id);
        let scope = bindings.get_scope(symbol.scope);

        scope.parent.is_none()
    }

    /// Return whether one symbol is declared under one function source node.
    fn is_symbol_owned_by_function(
        &self,
        symbol: dir::GlobalSymbolId,
        function: dir::GlobalSymbolId,
    ) -> bool {
        assert_eq!(
            function.module_id, self.module,
            "flow function {function:?} must belong to active module {:?}",
            self.module
        );

        // start from the symbol scope
        let bindings = self.check.module(self.module).binding_table();
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

    /// Select the contextual receiver for one `this` expression.
    fn select_this_receiver_binding(
        &mut self,
        source: dir::GlobalNodeIdAny,
        receiver: ReceiverBinding,
    ) -> CompilerResult<()> {
        // select bare receiver symbols directly
        let Some(owner) = receiver.receiver.owner else {
            let resolution = dir::NameResolution::new(receiver.symbol);

            return self.check.inference.select_name(source, resolution);
        };

        self.select_this_receiver(
            source,
            Receiver {
                owner: Some(owner),
                ty: receiver.receiver.ty,
            },
        )
    }

    /// Select the contextual receiver type for one `this` expression.
    fn select_this_receiver(
        &mut self,
        source: dir::GlobalNodeIdAny,
        receiver: Receiver,
    ) -> CompilerResult<()> {
        let Some(owner) = receiver.owner else {
            return Ok(());
        };

        // select receiver with owner metadata
        let resolution = ReceiverResolution {
            source,
            kind: dir::ReceiverKind::This,
            owner,
            ty: receiver.ty,
        };

        self.check.inference.select_receiver(resolution)
    }
}
