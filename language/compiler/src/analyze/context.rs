use destack_dir::{GlobalSymbolId, LocalTypeId};

/// InferContext holds contextual, flow-sensitive information during type analysis.
#[derive(Debug)]
pub struct InferContext {
    /// Type narrowings currently in scope.
    pub narrowings: Vec<(GlobalSymbolId, LocalTypeId)>,
    /// Whether we're in an unreachable code region (after `return`, `throw`, etc.).
    pub is_unreachable: bool,
    /// Whether we're inside a loop (break and continue both valid).
    pub in_loop: bool,
    /// Whether we're inside a switch (break valid, continue not).
    pub in_switch: bool,
}

impl InferContext {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self {
            narrowings: Vec::new(),
            is_unreachable: false,
            in_loop: false,
            in_switch: false,
        }
    }

    /// Fork the context for a code region (e.g., then/else of an if).
    pub fn fork(&self) -> Self {
        Self {
            narrowings: self.narrowings.clone(),
            is_unreachable: self.is_unreachable,
            in_loop: self.in_loop,
            in_switch: self.in_switch,
        }
    }

    /// Reset flow context (for function boundaries).
    pub fn reset(&self) -> Self {
        Self {
            narrowings: self.narrowings.clone(),
            is_unreachable: false,
            in_loop: false,
            in_switch: false,
        }
    }

    /// Enter a loop context.
    pub fn in_loop(mut self) -> Self {
        self.in_loop = true;
        self
    }

    /// Enter a switch context.
    pub fn in_switch(mut self) -> Self {
        self.in_switch = true;
        self
    }

    /// Check if we can break (in loop or switch).
    pub fn can_break(&self) -> bool {
        self.in_loop || self.in_switch
    }

    /// Check if we can continue (in loop only).
    pub fn can_continue(&self) -> bool {
        self.in_loop
    }

    /// Mark this context as unreachable.
    pub fn mark_unreachable(&mut self) {
        self.is_unreachable = true;
    }

    /// Add a narrowing for a symbol.
    pub fn narrow(&mut self, symbol: GlobalSymbolId, ty: LocalTypeId) {
        if let Some(index) = self.narrowings.iter().position(|(s, _)| *s == symbol) {
            self.narrowings[index] = (symbol, ty);
        } else {
            self.narrowings.push((symbol, ty));
        }
    }

    /// Get the narrowed type for a symbol, if any.
    pub fn get_narrowed(&self, symbol: GlobalSymbolId) -> Option<LocalTypeId> {
        self.narrowings
            .iter()
            .find(|(s, _)| *s == symbol)
            .map(|(_, ty)| *ty)
    }

    /// Merge two "branched" context together.
    pub fn merge(&mut self, other: &InferContext) {
        // if neither is unreachable
        if self.is_unreachable && other.is_unreachable {
            // both unreachable, stay unreachable
        }
        // if one is unreachable, keep the other's narrowings
        // if both branches are unreachable, result is unreachable
        else if self.is_unreachable {
            // self is unreachable, take other's state
            self.narrowings = other.narrowings.clone();
            self.is_unreachable = false;
        }
        // other is unreachable
        else if other.is_unreachable {
            // keep self's state
        }
        // both reachable -> keep only narrowings that exist in both
        else {
            self.narrowings.retain(|(s, ty)| {
                other
                    .narrowings
                    .iter()
                    .any(|(o_s, o_ty)| *s == *o_s && *ty == *o_ty)
            });
        }
    }
}

impl Default for InferContext {
    fn default() -> Self {
        Self::new()
    }
}
