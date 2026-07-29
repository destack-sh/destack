use std::collections::HashMap;

use destack_fir::format::{FormatError, FormatResult};

use crate::{Function, LifetimeParameter, LifetimeSlot, Local, LocalNodeId};

/// The lexical state of the node currently being formatted.
#[derive(Default)]
pub(crate) struct Scope {
    /// The current function.
    function: Option<LocalNodeId<Function>>,
    /// Local indices within the current function.
    locals: HashMap<LocalNodeId<Local>, usize>,
    /// Lifetime parameters currently in scope.
    lifetimes: Vec<LifetimeParameter>,
}

impl Scope {
    /// Enter one function scope.
    pub(crate) fn enter(
        &mut self,
        function: LocalNodeId<Function>,
        locals: &[LocalNodeId<Local>],
        lifetimes: &[LifetimeParameter],
    ) {
        self.function = Some(function);

        // index function locals
        self.locals.clear();
        self.locals
            .extend(locals.iter().enumerate().map(|(index, id)| (*id, index)));

        // retain function lifetime parameters
        self.lifetimes.clear();
        self.lifetimes.extend_from_slice(lifetimes);
    }

    /// Leave the current function scope.
    pub(crate) fn leave(&mut self) {
        self.function = None;
        self.locals.clear();
        self.lifetimes.clear();
    }

    /// Return the current function.
    pub(crate) fn function(&self) -> Option<LocalNodeId<Function>> {
        self.function
    }

    /// Replace the lifetime parameters and return the previous parameters.
    pub(crate) fn replace_lifetimes(
        &mut self,
        lifetimes: Vec<LifetimeParameter>,
    ) -> Vec<LifetimeParameter> {
        std::mem::replace(&mut self.lifetimes, lifetimes)
    }

    /// Return one local index.
    pub(crate) fn local(&self, id: LocalNodeId<Local>) -> FormatResult<usize> {
        self.locals
            .get(&id)
            .copied()
            .ok_or(FormatError::SyntaxError {
                message: "missing MIR local index",
            })
    }

    /// Return one lifetime parameter.
    pub(crate) fn lifetime(&self, slot: LifetimeSlot) -> Option<&LifetimeParameter> {
        self.lifetimes.get(slot.0 as usize)
    }
}
