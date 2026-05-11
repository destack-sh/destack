use serde::{Deserialize, Serialize};

/// Compiler known ambient symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum WellKnownSymbol {
    /// Ambient Array constructor symbol.
    Array,
    /// Ambient FixedArray type alias symbol.
    FixedArray,
    /// Ambient ReadonlyArray type alias symbol.
    ReadonlyArray,
    /// Ambient Map constructor symbol.
    Map,
    /// Ambient Set constructor symbol.
    Set,
    /// Ambient Record type alias symbol.
    Record,
    /// Ambient Slice type alias symbol.
    Slice,
    /// Ambient String constructor symbol.
    String,
    /// Ambient Number namespace symbol.
    Number,
    /// Ambient BigInt namespace symbol.
    BigInt,
    /// Ambient Function constructor symbol.
    Function,
    /// Ambient eval function symbol.
    Eval,
    /// Ambient Reflect namespace class symbol.
    Reflect,
    /// Ambient Promise constructor symbol.
    Promise,
    /// Ambient Iterable interface symbol.
    Iterable,
    /// Ambient Iterator interface symbol.
    Iterator,
    /// Ambient AsyncIterable interface symbol.
    AsyncIterable,
    /// Ambient AsyncIterator interface symbol.
    AsyncIterator,
    /// Ambient Symbol constructor symbol.
    Symbol,
    /// Ambient Vector type symbol.
    Vector,
}

impl WellKnownSymbol {
    /// Return the export name for a top level symbol.
    pub fn export_name(&self) -> &'static str {
        match self {
            WellKnownSymbol::Array => "Array",
            WellKnownSymbol::FixedArray => "FixedArray",
            WellKnownSymbol::ReadonlyArray => "ReadonlyArray",
            WellKnownSymbol::Map => "Map",
            WellKnownSymbol::Set => "Set",
            WellKnownSymbol::Record => "Record",
            WellKnownSymbol::Slice => "Slice",
            WellKnownSymbol::String => "String",
            WellKnownSymbol::Number => "Number",
            WellKnownSymbol::BigInt => "BigInt",
            WellKnownSymbol::Function => "Function",
            WellKnownSymbol::Eval => "eval",
            WellKnownSymbol::Reflect => "Reflect",
            WellKnownSymbol::Promise => "Promise",
            WellKnownSymbol::Iterable => "Iterable",
            WellKnownSymbol::Iterator => "Iterator",
            WellKnownSymbol::AsyncIterable => "AsyncIterable",
            WellKnownSymbol::AsyncIterator => "AsyncIterator",
            WellKnownSymbol::Symbol => "Symbol",
            WellKnownSymbol::Vector => "Vector",
        }
    }

    /// Iterate over all well known symbols.
    pub fn all() -> impl Iterator<Item = Self> {
        const ALL: &[WellKnownSymbol] = &[
            WellKnownSymbol::Array,
            WellKnownSymbol::FixedArray,
            WellKnownSymbol::ReadonlyArray,
            WellKnownSymbol::Map,
            WellKnownSymbol::Set,
            WellKnownSymbol::Record,
            WellKnownSymbol::Slice,
            WellKnownSymbol::String,
            WellKnownSymbol::Number,
            WellKnownSymbol::BigInt,
            WellKnownSymbol::Function,
            WellKnownSymbol::Eval,
            WellKnownSymbol::Reflect,
            WellKnownSymbol::Promise,
            WellKnownSymbol::Iterable,
            WellKnownSymbol::Iterator,
            WellKnownSymbol::AsyncIterable,
            WellKnownSymbol::AsyncIterator,
            WellKnownSymbol::Symbol,
            WellKnownSymbol::Vector,
        ];
        ALL.iter().copied()
    }
}

/// Compiler known Symbol.* keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum WellKnownSymbolKey {
    /// Symbol key for Symbol.iterator.
    SymbolIterator,
    /// Symbol key for Symbol.asyncIterator.
    SymbolAsyncIterator,
    /// Symbol key for Symbol.dispose.
    SymbolDispose,
    /// Symbol key for Symbol.asyncDispose.
    SymbolAsyncDispose,
}

impl WellKnownSymbolKey {
    /// Return the member name for a well known key.
    pub fn member_name(&self) -> &'static str {
        match self {
            WellKnownSymbolKey::SymbolIterator => "iterator",
            WellKnownSymbolKey::SymbolAsyncIterator => "asyncIterator",
            WellKnownSymbolKey::SymbolDispose => "dispose",
            WellKnownSymbolKey::SymbolAsyncDispose => "asyncDispose",
        }
    }

    /// Return the full global name for a well known key.
    pub fn global_symbol_name(&self) -> &'static str {
        match self {
            WellKnownSymbolKey::SymbolIterator => "Symbol.iterator",
            WellKnownSymbolKey::SymbolAsyncIterator => "Symbol.asyncIterator",
            WellKnownSymbolKey::SymbolDispose => "Symbol.dispose",
            WellKnownSymbolKey::SymbolAsyncDispose => "Symbol.asyncDispose",
        }
    }

    /// Return the base symbol for a well known key.
    pub fn base_symbol(&self) -> WellKnownSymbol {
        WellKnownSymbol::Symbol
    }

    /// Iterate over all well known Symbol keys.
    pub fn all() -> impl Iterator<Item = Self> {
        const ALL: &[WellKnownSymbolKey] = &[
            WellKnownSymbolKey::SymbolIterator,
            WellKnownSymbolKey::SymbolAsyncIterator,
            WellKnownSymbolKey::SymbolDispose,
            WellKnownSymbolKey::SymbolAsyncDispose,
        ];
        ALL.iter().copied()
    }
}
