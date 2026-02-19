use destack_dir::{GlobalSymbolId, StaticArgument};

/// A canonical static substitution environment for one instantiation event.
#[derive(Debug, Clone)]
pub(crate) struct StaticSubstitutionEnvironment {
    /// Canonical static arguments in declaration parameter order.
    arguments: Vec<StaticArgument>,
    /// Parameter symbols aligned one-to-one with `arguments` when known.
    parameter_symbols: Vec<Option<GlobalSymbolId>>,
    /// Number of inherited owner arguments at the front of `arguments`.
    inherited_arity: usize,
}

impl StaticSubstitutionEnvironment {
    /// Build an environment from known parameter-symbol order.
    pub(crate) fn from_parameter_symbols(
        arguments: Vec<StaticArgument>,
        parameter_symbols: Vec<GlobalSymbolId>,
        inherited_arity: usize,
    ) -> Option<Self> {
        if parameter_symbols.len() != arguments.len() {
            return None;
        }
        if inherited_arity > arguments.len() {
            return None;
        }

        let parameter_symbols = parameter_symbols.into_iter().map(Some).collect();
        Some(Self {
            arguments,
            parameter_symbols,
            inherited_arity,
        })
    }

    /// Build an environment from optional parameter-symbol identities.
    pub(crate) fn from_optional_parameter_symbols(
        arguments: Vec<StaticArgument>,
        parameter_symbols: Vec<Option<GlobalSymbolId>>,
        inherited_arity: usize,
    ) -> Option<Self> {
        if parameter_symbols.len() != arguments.len() {
            return None;
        }
        if inherited_arity > arguments.len() {
            return None;
        }

        Some(Self {
            arguments,
            parameter_symbols,
            inherited_arity,
        })
    }

    /// Return all static arguments.
    pub(crate) fn arguments(&self) -> &[StaticArgument] {
        &self.arguments
    }

    /// Return whether all parameter symbols are known.
    pub(crate) fn is_complete(&self) -> bool {
        self.parameter_symbols.iter().all(|symbol| symbol.is_some())
    }

    /// Return all parameter symbols when the environment is complete.
    pub(crate) fn complete_parameter_symbols(&self) -> Option<Vec<GlobalSymbolId>> {
        if !self.is_complete() {
            return None;
        }

        self.parameter_symbols.iter().copied().collect()
    }

    /// Return the inherited-argument arity.
    pub(crate) fn inherited_arity(&self) -> usize {
        self.inherited_arity
    }

    /// Return whether all arguments are evaluated and committable.
    pub(crate) fn is_committable(&self) -> bool {
        self.arguments.iter().all(StaticArgument::is_evaluated) && self.is_complete()
    }

    /// Consume and return `(arguments, parameter_symbols, inherited_arity)`.
    pub(crate) fn into_parts(self) -> (Vec<StaticArgument>, Vec<Option<GlobalSymbolId>>, usize) {
        (self.arguments, self.parameter_symbols, self.inherited_arity)
    }
}
