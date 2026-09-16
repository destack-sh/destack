use destack_core::{StringId, StringPool};
use destack_source::ModuleId;

use crate::{
    Function, FunctionKind, FunctionParameter, GenericArgument, GenericParameter, Lifetime,
    LifetimeParameter, Symbol, TypeId, Value,
};

/// Header used to declare or build one MIR function.
#[derive(Debug, Clone)]
pub struct FunctionHeader {
    /// The function name.
    pub name: StringId,
    /// The generic parameters the function takes.
    pub generics: Vec<GenericParameter>,
    /// The generic arguments a specialization applies to its template.
    pub arguments: Vec<GenericArgument>,
    /// The persistent function identity.
    pub symbol: Symbol,
    /// Lifetime parameters in function-local slot order.
    pub lifetimes: Vec<LifetimeParameter>,
    /// Parameter types in SSA parameter order.
    pub parameters: Vec<TypeId>,
    /// The return type.
    pub result: TypeId,
    /// The role of the function.
    pub kind: FunctionKind,
}

/// Builder for one MIR function header.
#[derive(Debug)]
pub struct FunctionHeaderBuilder<'a> {
    /// The string pool used for names.
    strings: &'a StringPool,
    /// The function name.
    name: StringId,
    /// The generic parameters the function takes.
    generics: Vec<GenericParameter>,
    /// The generic arguments a specialization applies to its template.
    arguments: Vec<GenericArgument>,
    /// The persistent function identity.
    symbol: Symbol,
    /// Lifetime parameters in function-local slot order.
    lifetimes: Vec<LifetimeParameter>,
    /// Parameter types in SSA parameter order.
    parameters: Vec<TypeId>,
    /// The role of the function.
    kind: FunctionKind,
}

impl<'a> FunctionHeaderBuilder<'a> {
    /// Create one function header builder.
    pub fn new(strings: &'a StringPool, module: ModuleId, name: &str) -> Self {
        let name = strings.intern(name);

        Self {
            strings,
            name,
            generics: Vec::new(),
            arguments: Vec::new(),
            symbol: Symbol::named(module, name),
            lifetimes: Vec::new(),
            parameters: Vec::new(),
            kind: FunctionKind::Function,
        }
    }

    /// Set the role of the function.
    pub fn kind(mut self, kind: FunctionKind) -> Self {
        self.kind = kind;

        self
    }

    /// Set the persistent function identity.
    pub fn symbol(mut self, symbol: Symbol) -> Self {
        self.symbol = symbol;

        self
    }

    /// Set the concrete generic arguments specializing this function.
    pub fn arguments(mut self, arguments: impl IntoIterator<Item = GenericArgument>) -> Self {
        self.arguments.extend(arguments);

        self
    }

    /// Declare the generic parameters the function takes.
    pub fn generics(mut self, generics: Vec<GenericParameter>) -> Self {
        self.generics = generics;

        self
    }

    /// Add one lifetime parameter.
    pub fn lifetime(mut self, name: &str) -> Self {
        let name = self.strings.intern(name);
        self.lifetimes.push(LifetimeParameter::new(Some(name)));

        self
    }

    /// Add one lifetime parameter with its declared outlives slots.
    pub fn lifetime_outlives(mut self, name: &str, outlives: Lifetime) -> Self {
        let name = self.strings.intern(name);
        self.lifetimes
            .push(LifetimeParameter::with_outlives(Some(name), outlives));

        self
    }

    /// Add several lifetime parameters.
    pub fn lifetimes<const N: usize>(mut self, names: [&str; N]) -> Self {
        for name in names {
            let name = self.strings.intern(name);
            self.lifetimes.push(LifetimeParameter::new(Some(name)));
        }

        self
    }

    /// Add one parameter type.
    pub fn parameter(mut self, ty: TypeId) -> Self {
        self.parameters.push(ty);

        self
    }

    /// Add several parameter types.
    pub fn parameters(mut self, types: impl IntoIterator<Item = TypeId>) -> Self {
        self.parameters.extend(types);

        self
    }

    /// Finish the header with its return type.
    pub fn result(self, result: TypeId) -> FunctionHeader {
        FunctionHeader {
            name: self.name,
            generics: self.generics,
            arguments: self.arguments,
            symbol: self.symbol,
            lifetimes: self.lifetimes,
            parameters: self.parameters,
            result,
            kind: self.kind,
        }
    }
}

impl FunctionHeader {
    /// Consume this header into a locally declared function.
    pub fn declared(self) -> Function {
        let parameters = Self::parameters_from_types(self.parameters);
        let module = self.symbol.declaring_module();

        Function::declare(module, self.name, self.lifetimes, parameters, self.result)
            .with_kind(self.kind)
            .with_generics(self.generics)
            .with_arguments(self.arguments)
            .with_symbol(self.symbol)
    }

    /// Consume this header into an imported function.
    pub fn imported(self) -> Function {
        let parameters = Self::parameters_from_types(self.parameters);
        let module = self.symbol.declaring_module();

        Function::import(module, self.name, self.lifetimes, parameters, self.result)
            .with_kind(self.kind)
            .with_generics(self.generics)
            .with_arguments(self.arguments)
            .with_symbol(self.symbol)
    }

    /// Build SSA parameters from parameter types.
    pub(in crate::build) fn parameters_from_types(
        parameters: Vec<TypeId>,
    ) -> Vec<FunctionParameter> {
        parameters
            .into_iter()
            .enumerate()
            .map(|(index, ty)| FunctionParameter::new(Value::new(index as u32), ty))
            .collect()
    }
}
