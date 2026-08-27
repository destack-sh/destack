use destack_core::{StringId, StringPool};
use destack_source::{ProvenanceId, ProvenanceJournal};

use crate::{FunctionParameter, LifetimeParameter, LifetimeSlot, StaticId, Symbol, TypeId, Value};

/// Header used to declare or build one MIR function.
#[derive(Debug, Clone)]
pub struct FunctionHeader {
    /// The function name.
    pub name: StringId,
    /// Concrete generic arguments specializing this function.
    pub arguments: Vec<StaticId>,
    /// The persistent function identity.
    pub symbol: Symbol,
    /// Lifetime parameters in function-local slot order.
    pub lifetimes: Vec<LifetimeParameter>,
    /// Parameters in SSA value order.
    pub parameters: Vec<FunctionParameter>,
    /// The return type.
    pub result: TypeId,
}

/// Builder for one MIR function header.
#[derive(Debug)]
pub struct FunctionHeaderBuilder<'a> {
    /// The string pool used for names.
    strings: &'a mut StringPool,
    /// The journal producing parameter provenance.
    provenance: ProvenanceJournal<'a>,
    /// The function name.
    name: StringId,
    /// Concrete generic arguments specializing this function.
    arguments: Vec<StaticId>,
    /// The persistent function identity.
    symbol: Symbol,
    /// Lifetime parameters in function-local slot order.
    lifetimes: Vec<LifetimeParameter>,
    /// Parameters in SSA value order.
    parameters: Vec<FunctionParameter>,
}

impl<'a> FunctionHeaderBuilder<'a> {
    /// Create a function header builder.
    pub(in crate::build) fn new(
        strings: &'a mut StringPool,
        provenance: ProvenanceJournal<'a>,
        name: &str,
    ) -> Self {
        let name = strings.intern(name);

        Self {
            strings,
            provenance,
            name,
            arguments: Vec::new(),
            symbol: Symbol::named(name),
            lifetimes: Vec::new(),
            parameters: Vec::new(),
        }
    }

    /// Set the persistent function identity.
    pub fn symbol(mut self, symbol: Symbol) -> Self {
        self.symbol = symbol;

        self
    }

    /// Set the concrete generic arguments specializing this function.
    pub fn arguments(mut self, arguments: impl IntoIterator<Item = StaticId>) -> Self {
        self.arguments.extend(arguments);

        self
    }

    /// Add one lifetime parameter.
    pub fn lifetime(mut self, name: &str) -> Self {
        let name = self.strings.intern(name);
        self.lifetimes.push(LifetimeParameter::new(Some(name)));

        self
    }

    /// Add a lifetime parameter with declared outlives slots.
    pub fn lifetime_outlives(mut self, name: &str, outlives: Vec<LifetimeSlot>) -> Self {
        let name = self.strings.intern(name);
        self.lifetimes
            .push(LifetimeParameter::with_outlives(Some(name), outlives));

        self
    }

    /// Add lifetime parameters.
    pub fn lifetimes<const N: usize>(mut self, names: [&str; N]) -> Self {
        for name in names {
            let name = self.strings.intern(name);
            self.lifetimes.push(LifetimeParameter::new(Some(name)));
        }

        self
    }

    /// Add one parameter derived from the given provenance.
    pub fn parameter(mut self, ty: TypeId, source: ProvenanceId) -> Self {
        self.push_parameter(ty, source);

        self
    }

    /// Add parameters derived from the given provenance.
    pub fn parameters(
        mut self,
        parameters: impl IntoIterator<Item = (TypeId, ProvenanceId)>,
    ) -> Self {
        for (ty, source) in parameters {
            self.push_parameter(ty, source);
        }

        self
    }

    /// Finish the header with its return type.
    pub fn result(self, result: TypeId) -> FunctionHeader {
        FunctionHeader {
            name: self.name,
            arguments: self.arguments,
            symbol: self.symbol,
            lifetimes: self.lifetimes,
            parameters: self.parameters,
            result,
        }
    }

    /// Append one parameter occurrence.
    fn push_parameter(&mut self, ty: TypeId, source: ProvenanceId) {
        let value = Value::new(self.parameters.len() as u32);
        let provenance = self.provenance.derive(source);
        self.parameters
            .push(FunctionParameter::new(value, ty, provenance));
    }
}
