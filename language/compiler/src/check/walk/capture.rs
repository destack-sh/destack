use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Decision, Receiver, ReceiverBinding, WalkState};

impl WalkState<'_, '_> {
    /// Commit the receiver decision visible at the current walk point.
    pub(in crate::check) fn commit_active_receiver_decision(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<Receiver>> {
        // prefer receiver from an active function frame
        if let Some((index, receiver)) = self.flow().lexical_receiver() {
            let is_current = self.flow().is_current_function(index);

            // select local receiver directly
            if is_current {
                self.commit_receiver_binding_decision(source, receiver)?;
            }
            // capture receiver from an outer function
            else {
                self.flow_mut().capture_receiver(receiver);
                let resolution = dir::NameResolution::new(receiver.symbol);
                self.check
                    .commit_decision(source, Decision::Name(resolution))?;
                self.check
                    .commit_access(source, dir::AccessPath::symbol(receiver.symbol))?;
            }

            return Ok(Some(receiver.receiver));
        }

        // do not inherit declaration receivers into function bodies
        if self.flow().current_function().is_some() {
            return Ok(None);
        }

        // use contextual receiver outside function bodies
        if let Some(receiver) = self.flow().current_receiver() {
            self.commit_receiver_decision(source, receiver)?;

            return Ok(Some(receiver));
        }

        Ok(None)
    }

    /// Capture one lexical value reference when required.
    pub(in crate::check) fn capture_symbol_reference(&mut self, symbol: dir::GlobalSymbolId) {
        // ignore references outside function bodies
        let Some(function) = self.flow().current_function_symbol() else {
            return;
        };

        // ignore references that do not cross into an outer function
        if !self.is_captured_symbol_reference(symbol, function) {
            return;
        }

        // capture the outer symbol
        self.flow_mut().capture_symbol(symbol);
    }

    /// Return whether one value reference crosses into an outer function.
    pub(in crate::check) fn is_captured_symbol_reference(
        &self,
        symbol: dir::GlobalSymbolId,
        function: dir::GlobalSymbolId,
    ) -> bool {
        symbol.module_id == self.module
            && self
                .check
                .own_symbol_kind(symbol)
                .is_some_and(dir::SymbolKind::is_binding)
            && !self.is_lexical_receiver_symbol(symbol)
            && !self
                .check
                .module(self.module)
                .is_import_alias(symbol.local_id)
            && symbol != function
            && !self.is_symbol_owned_by_function(symbol, function)
    }

    /// Return whether one symbol is the active lexical receiver.
    fn is_lexical_receiver_symbol(&self, symbol: dir::GlobalSymbolId) -> bool {
        self.flow()
            .lexical_receiver()
            .is_some_and(|(_, receiver)| receiver.symbol == symbol)
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

    /// Commit a receiver decision from one lexical receiver binding.
    fn commit_receiver_binding_decision(
        &mut self,
        source: dir::GlobalNodeIdAny,
        receiver: ReceiverBinding,
    ) -> CompilerResult<()> {
        // select bare receiver symbols directly
        let Some(declaration) = receiver.receiver.declaration else {
            let resolution = dir::NameResolution::new(receiver.symbol);
            self.check
                .commit_decision(source, Decision::Name(resolution))?;

            return self
                .check
                .commit_access(source, dir::AccessPath::symbol(receiver.symbol));
        };

        self.commit_receiver_decision(
            source,
            Receiver {
                declaration: Some(declaration),
                ownership: receiver.receiver.ownership,
                ty: receiver.receiver.ty,
                super_ty: receiver.receiver.super_ty,
            },
        )
    }

    /// Commit one contextual receiver decision.
    fn commit_receiver_decision(
        &mut self,
        source: dir::GlobalNodeIdAny,
        receiver: Receiver,
    ) -> CompilerResult<()> {
        let Some(declaration) = receiver.declaration else {
            return Ok(());
        };

        // commit receiver with declaration context
        let resolution = dir::ReceiverResolution {
            kind: dir::ReceiverKind::This,
            declaration,
            ty: receiver.ty,
        };

        self.check
            .commit_decision(source, Decision::Receiver(resolution))?;

        self.check
            .commit_access(source, dir::AccessPath::receiver(dir::ReceiverKind::This))
    }
}
