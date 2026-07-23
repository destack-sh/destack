use destack_core::{StringId, StringPool};

use crate::{FunctionParameter, LifetimeParameter, LifetimeSlot, LocalNodeId, Symbol, Type, Value};

/// Header used to declare or build one MIR function.
#[derive(Debug, Clone)]
pub struct FunctionHeader {
    /// The function name.
    pub name: StringId,
    /// The persistent function identity.
    pub symbol: Symbol,
    /// Lifetime parameters in function-local slot order.
    pub lifetimes: Vec<LifetimeParameter>,
    /// Parameter types in SSA parameter order.
    pub parameters: Vec<LocalNodeId<Type>>,
    /// The return type.
    pub result: LocalNodeId<Type>,
}

/// Builder for one MIR function header.
#[derive(Debug)]
pub struct FunctionHeaderBuilder<'a> {
    /// The string pool used for names.
    strings: &'a mut StringPool,
    /// The function name.
    name: StringId,
    /// The persistent function identity.
    symbol: Symbol,
    /// Lifetime parameters in function-local slot order.
    lifetimes: Vec<LifetimeParameter>,
    /// Parameter types in SSA parameter order.
    parameters: Vec<LocalNodeId<Type>>,
}

impl<'a> FunctionHeaderBuilder<'a> {
    /// Create a function header builder.
    pub(in crate::build) fn new(strings: &'a mut StringPool, name: &str) -> Self {
        let name = strings.intern(name);

        Self {
            strings,
            name,
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

    /// Add one parameter type.
    pub fn parameter(mut self, ty: LocalNodeId<Type>) -> Self {
        self.parameters.push(ty);

        self
    }

    /// Add parameter types.
    pub fn parameters(mut self, types: impl IntoIterator<Item = LocalNodeId<Type>>) -> Self {
        self.parameters.extend(types);

        self
    }

    /// Finish the header with its return type.
    pub fn result(self, result: LocalNodeId<Type>) -> FunctionHeader {
        FunctionHeader {
            name: self.name,
            symbol: self.symbol,
            lifetimes: self.lifetimes,
            parameters: self.parameters,
            result,
        }
    }
}

impl FunctionHeader {
    /// Build SSA parameters from parameter types.
    pub(in crate::build) fn parameters_from_types(
        parameters: Vec<LocalNodeId<Type>>,
    ) -> Vec<FunctionParameter> {
        parameters
            .into_iter()
            .enumerate()
            .map(|(index, ty)| FunctionParameter::new(Value::new(index as u32), ty))
            .collect()
    }
}
