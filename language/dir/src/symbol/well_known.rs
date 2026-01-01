/// Compiler known builtin symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(clippy::upper_case_acronyms)]
pub enum WellKnownSymbol {
    /// Builtin Array constructor symbol.
    Array,
    /// Builtin Promise constructor symbol.
    Promise,
    /// Builtin Iterable type symbol.
    Iterable,
    /// Builtin Iterator type symbol.
    Iterator,
    /// Builtin AsyncIterable type symbol.
    AsyncIterable,
    /// Builtin AsyncIterator type symbol.
    AsyncIterator,
    /// Builtin Symbol constructor symbol.
    Symbol,
}

impl WellKnownSymbol {
    /// Return the export name for a top level symbol.
    pub fn export_name(&self) -> &'static str {
        match self {
            WellKnownSymbol::Array => "Array",
            WellKnownSymbol::Promise => "Promise",
            WellKnownSymbol::Iterable => "Iterable",
            WellKnownSymbol::Iterator => "Iterator",
            WellKnownSymbol::AsyncIterable => "AsyncIterable",
            WellKnownSymbol::AsyncIterator => "AsyncIterator",
            WellKnownSymbol::Symbol => "Symbol",
        }
    }

    /// Iterate over all well known symbols.
    pub fn all() -> impl Iterator<Item = Self> {
        const ALL: &[WellKnownSymbol] = &[
            WellKnownSymbol::Array,
            WellKnownSymbol::Promise,
            WellKnownSymbol::Iterable,
            WellKnownSymbol::Iterator,
            WellKnownSymbol::AsyncIterable,
            WellKnownSymbol::AsyncIterator,
            WellKnownSymbol::Symbol,
        ];
        ALL.iter().copied()
    }
}

/// Compiler known Symbol.* keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(clippy::upper_case_acronyms)]
pub enum WellKnownSymbolKey {
    /// Symbol key for Symbol.iterator.
    SymbolIterator,
    /// Symbol key for Symbol.asyncIterator.
    SymbolAsyncIterator,
    /// Symbol key for Symbol.hasInstance.
    SymbolHasInstance,
    /// Symbol key for Symbol.isConcatSpreadable.
    SymbolIsConcatSpreadable,
    /// Symbol key for Symbol.match.
    SymbolMatch,
    /// Symbol key for Symbol.matchAll.
    SymbolMatchAll,
    /// Symbol key for Symbol.replace.
    SymbolReplace,
    /// Symbol key for Symbol.search.
    SymbolSearch,
    /// Symbol key for Symbol.species.
    SymbolSpecies,
    /// Symbol key for Symbol.split.
    SymbolSplit,
    /// Symbol key for Symbol.toPrimitive.
    SymbolToPrimitive,
    /// Symbol key for Symbol.toStringTag.
    SymbolToStringTag,
    /// Symbol key for Symbol.unscopables.
    SymbolUnscopables,
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
            WellKnownSymbolKey::SymbolHasInstance => "hasInstance",
            WellKnownSymbolKey::SymbolIsConcatSpreadable => "isConcatSpreadable",
            WellKnownSymbolKey::SymbolMatch => "match",
            WellKnownSymbolKey::SymbolMatchAll => "matchAll",
            WellKnownSymbolKey::SymbolReplace => "replace",
            WellKnownSymbolKey::SymbolSearch => "search",
            WellKnownSymbolKey::SymbolSpecies => "species",
            WellKnownSymbolKey::SymbolSplit => "split",
            WellKnownSymbolKey::SymbolToPrimitive => "toPrimitive",
            WellKnownSymbolKey::SymbolToStringTag => "toStringTag",
            WellKnownSymbolKey::SymbolUnscopables => "unscopables",
            WellKnownSymbolKey::SymbolDispose => "dispose",
            WellKnownSymbolKey::SymbolAsyncDispose => "asyncDispose",
        }
    }

    /// Return the full global name for a well known key.
    pub fn global_symbol_name(&self) -> &'static str {
        match self {
            WellKnownSymbolKey::SymbolIterator => "Symbol.iterator",
            WellKnownSymbolKey::SymbolAsyncIterator => "Symbol.asyncIterator",
            WellKnownSymbolKey::SymbolHasInstance => "Symbol.hasInstance",
            WellKnownSymbolKey::SymbolIsConcatSpreadable => "Symbol.isConcatSpreadable",
            WellKnownSymbolKey::SymbolMatch => "Symbol.match",
            WellKnownSymbolKey::SymbolMatchAll => "Symbol.matchAll",
            WellKnownSymbolKey::SymbolReplace => "Symbol.replace",
            WellKnownSymbolKey::SymbolSearch => "Symbol.search",
            WellKnownSymbolKey::SymbolSpecies => "Symbol.species",
            WellKnownSymbolKey::SymbolSplit => "Symbol.split",
            WellKnownSymbolKey::SymbolToPrimitive => "Symbol.toPrimitive",
            WellKnownSymbolKey::SymbolToStringTag => "Symbol.toStringTag",
            WellKnownSymbolKey::SymbolUnscopables => "Symbol.unscopables",
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
            WellKnownSymbolKey::SymbolHasInstance,
            WellKnownSymbolKey::SymbolIsConcatSpreadable,
            WellKnownSymbolKey::SymbolMatch,
            WellKnownSymbolKey::SymbolMatchAll,
            WellKnownSymbolKey::SymbolReplace,
            WellKnownSymbolKey::SymbolSearch,
            WellKnownSymbolKey::SymbolSpecies,
            WellKnownSymbolKey::SymbolSplit,
            WellKnownSymbolKey::SymbolToPrimitive,
            WellKnownSymbolKey::SymbolToStringTag,
            WellKnownSymbolKey::SymbolUnscopables,
            WellKnownSymbolKey::SymbolDispose,
            WellKnownSymbolKey::SymbolAsyncDispose,
        ];
        ALL.iter().copied()
    }
}
