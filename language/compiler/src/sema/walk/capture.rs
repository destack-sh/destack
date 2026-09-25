use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, Receiver, ReceiverBinding};

impl CheckState<'_> {
    /// Commit the receiver decision one receiver kind makes at the current walk point.
    pub(in crate::sema) fn commit_active_receiver_decision(
        &mut self,
        source: dir::GlobalNodeIdAny,
        kind: dir::ReceiverKind,
    ) -> CompilerResult<Option<Receiver>> {
        // prefer receiver from an active function frame
        if let Some((is_own, receiver)) = self.flow.lexical_receiver() {
            // capture the receiver of an outer function
            if !is_own {
                self.flow.capture_receiver(receiver);

                // name the captured binding `this` reads
                if kind == dir::ReceiverKind::This {
                    let resolution = dir::NameResolution::new(receiver.symbol);
                    self.commit_name(source, resolution)?;
                }
            }

            // select the receiver the frame binds
            self.commit_receiver_binding_decision(source, receiver, kind)?;

            return Ok(Some(receiver.receiver));
        }

        // stop declaration receivers at the function boundary
        if self.flow.current_function().is_some() {
            return Ok(None);
        }

        // use contextual receiver outside function bodies
        if let Some(receiver) = self.flow.current_receiver() {
            self.commit_receiver_decision(source, receiver, kind)?;

            return Ok(Some(receiver));
        }

        Ok(None)
    }

    /// Capture one lexical value reference when required.
    pub(in crate::sema) fn capture_symbol_reference(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        // ignore references outside function bodies
        let Some(function) = self.flow.current_function_symbol() else {
            return Ok(());
        };

        // keep references crossing into an outer function
        if !self.is_captured_symbol_reference(symbol, function) {
            return Ok(());
        }

        // capture the outer symbol
        self.flow.capture_symbol(symbol);
        self.module_mut(source.module_id).flows.commit_binding_use(
            source.local_id,
            symbol,
            dir::BindingUse::CAPTURE,
        );

        Ok(())
    }

    /// Return whether one value reference crosses into an outer function.
    pub(in crate::sema) fn is_captured_symbol_reference(
        &self,
        symbol: dir::GlobalSymbolId,
        function: dir::GlobalSymbolId,
    ) -> bool {
        symbol.module_id == self.module_id
            && self
                .own_symbol_kind(symbol)
                .is_some_and(dir::SymbolKind::is_binding)
            && !self.is_lexical_receiver_symbol(symbol)
            && !self.module(self.module_id).is_import_alias(symbol.local_id)
            && symbol != function
            && !self.is_symbol_owned_by_function(symbol, function)
            && self.is_function_scoped_symbol(symbol)
    }

    /// Return whether one symbol lives inside some function body.
    ///
    /// Module and namespace bindings are static storage.
    fn is_function_scoped_symbol(&self, symbol: dir::GlobalSymbolId) -> bool {
        // start at the scope declaring the symbol
        let bindings = self.module(self.module_id).binding_table();
        let symbol = bindings.get_symbol(symbol.local_id);
        let mut scope = Some(symbol.scope.id);

        // climb to the nearest function scope
        while let Some(scope_id) = scope {
            let current = bindings.get_scope_by_id(scope_id);
            if current.kind == dir::ScopeKind::Function {
                return true;
            }

            scope = current.parent.map(|parent| parent.id);
        }

        false
    }

    /// Return whether one symbol is the active lexical receiver.
    fn is_lexical_receiver_symbol(&self, symbol: dir::GlobalSymbolId) -> bool {
        self.flow
            .lexical_receiver()
            .is_some_and(|(_, receiver)| receiver.symbol == symbol)
    }

    /// Commit a receiver decision from one lexical receiver binding.
    fn commit_receiver_binding_decision(
        &mut self,
        source: dir::GlobalNodeIdAny,
        receiver: ReceiverBinding,
        kind: dir::ReceiverKind,
    ) -> CompilerResult<()> {
        // select bare receiver symbols directly
        let Some(declaration) = receiver.receiver.declaration else {
            let resolution = dir::NameResolution::new(receiver.symbol);
            self.commit_name(source, resolution)?;

            return self.commit_access(source, dir::AccessPath::receiver());
        };

        // commit the receiver the capture reads
        self.commit_receiver_decision(
            source,
            Receiver {
                declaration: Some(declaration),
                ownership: receiver.receiver.ownership,
                ty: receiver.receiver.ty,
                super_ty: receiver.receiver.super_ty,
            },
            kind,
        )
    }

    /// Commit one contextual receiver decision.
    fn commit_receiver_decision(
        &mut self,
        source: dir::GlobalNodeIdAny,
        receiver: Receiver,
        kind: dir::ReceiverKind,
    ) -> CompilerResult<()> {
        // name the instance for `this` and the superclass above it for `super`
        let ty = match kind {
            dir::ReceiverKind::This => Some(receiver.ty),
            dir::ReceiverKind::Super => receiver.super_ty,
        };

        // skip a receiver the context leaves without a declaration or a type
        let (Some(declaration), Some(ty)) = (receiver.declaration, ty) else {
            return Ok(());
        };

        // commit receiver with declaration context
        let resolution = dir::ReceiverDecision {
            kind,
            declaration,
            ty,
        };

        self.commit_decision(source, dir::Decision::Receiver(resolution))?;

        self.commit_access(source, dir::AccessPath::receiver())
    }
}

impl CheckState<'_> {
    /// Return whether one symbol is declared under one function source node.
    pub(in crate::sema) fn is_symbol_owned_by_function(
        &self,
        symbol: dir::GlobalSymbolId,
        function: dir::GlobalSymbolId,
    ) -> bool {
        assert_eq!(
            function.module_id, self.module_id,
            "flow function {function:?} must belong to active module {:?}",
            self.module_id
        );

        // start from the symbol scope
        let bindings = self.module(self.module_id).binding_table();
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
}
