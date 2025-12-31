/// Compiler known builtin symbols and symbol keys.
/// NOTE #Architecture: split well known declarations (Array/Promise/..) from Symbol.iteartor/split/...?
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

impl WellKnownSymbol {
    /// Return the export name for a top level symbol.
    pub fn export_name(&self) -> Option<&'static str> {
        match self {
            WellKnownSymbol::Array => Some("Array"),
            WellKnownSymbol::Promise => Some("Promise"),
            WellKnownSymbol::Iterable => Some("Iterable"),
            WellKnownSymbol::Iterator => Some("Iterator"),
            WellKnownSymbol::AsyncIterable => Some("AsyncIterable"),
            WellKnownSymbol::AsyncIterator => Some("AsyncIterator"),
            WellKnownSymbol::Symbol => Some("Symbol"),
            _ => None,
        }
    }

    /// Return the base symbol for a well known key.
    pub fn base_symbol(&self) -> Option<Self> {
        self.is_symbol_key().then_some(WellKnownSymbol::Symbol)
    }

    /// Return the member name for a well known key.
    pub fn member_name(&self) -> Option<&'static str> {
        match self {
            WellKnownSymbol::SymbolIterator => Some("iterator"),
            WellKnownSymbol::SymbolAsyncIterator => Some("asyncIterator"),
            WellKnownSymbol::SymbolHasInstance => Some("hasInstance"),
            WellKnownSymbol::SymbolIsConcatSpreadable => Some("isConcatSpreadable"),
            WellKnownSymbol::SymbolMatch => Some("match"),
            WellKnownSymbol::SymbolMatchAll => Some("matchAll"),
            WellKnownSymbol::SymbolReplace => Some("replace"),
            WellKnownSymbol::SymbolSearch => Some("search"),
            WellKnownSymbol::SymbolSpecies => Some("species"),
            WellKnownSymbol::SymbolSplit => Some("split"),
            WellKnownSymbol::SymbolToPrimitive => Some("toPrimitive"),
            WellKnownSymbol::SymbolToStringTag => Some("toStringTag"),
            WellKnownSymbol::SymbolUnscopables => Some("unscopables"),
            WellKnownSymbol::SymbolDispose => Some("dispose"),
            WellKnownSymbol::SymbolAsyncDispose => Some("asyncDispose"),
            _ => None,
        }
    }

    /// Return the full global name for a well known key.
    pub fn global_symbol_name(&self) -> Option<&'static str> {
        match self {
            WellKnownSymbol::SymbolIterator => Some("Symbol.iterator"),
            WellKnownSymbol::SymbolAsyncIterator => Some("Symbol.asyncIterator"),
            WellKnownSymbol::SymbolHasInstance => Some("Symbol.hasInstance"),
            WellKnownSymbol::SymbolIsConcatSpreadable => Some("Symbol.isConcatSpreadable"),
            WellKnownSymbol::SymbolMatch => Some("Symbol.match"),
            WellKnownSymbol::SymbolMatchAll => Some("Symbol.matchAll"),
            WellKnownSymbol::SymbolReplace => Some("Symbol.replace"),
            WellKnownSymbol::SymbolSearch => Some("Symbol.search"),
            WellKnownSymbol::SymbolSpecies => Some("Symbol.species"),
            WellKnownSymbol::SymbolSplit => Some("Symbol.split"),
            WellKnownSymbol::SymbolToPrimitive => Some("Symbol.toPrimitive"),
            WellKnownSymbol::SymbolToStringTag => Some("Symbol.toStringTag"),
            WellKnownSymbol::SymbolUnscopables => Some("Symbol.unscopables"),
            WellKnownSymbol::SymbolDispose => Some("Symbol.dispose"),
            WellKnownSymbol::SymbolAsyncDispose => Some("Symbol.asyncDispose"),
            _ => None,
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
            WellKnownSymbol::SymbolIterator,
            WellKnownSymbol::SymbolAsyncIterator,
            WellKnownSymbol::SymbolHasInstance,
            WellKnownSymbol::SymbolIsConcatSpreadable,
            WellKnownSymbol::SymbolMatch,
            WellKnownSymbol::SymbolMatchAll,
            WellKnownSymbol::SymbolReplace,
            WellKnownSymbol::SymbolSearch,
            WellKnownSymbol::SymbolSpecies,
            WellKnownSymbol::SymbolSplit,
            WellKnownSymbol::SymbolToPrimitive,
            WellKnownSymbol::SymbolToStringTag,
            WellKnownSymbol::SymbolUnscopables,
            WellKnownSymbol::SymbolDispose,
            WellKnownSymbol::SymbolAsyncDispose,
        ];
        ALL.iter().copied()
    }

    /// Return true when this is a Symbol key.
    fn is_symbol_key(&self) -> bool {
        matches!(
            self,
            WellKnownSymbol::SymbolIterator
                | WellKnownSymbol::SymbolAsyncIterator
                | WellKnownSymbol::SymbolHasInstance
                | WellKnownSymbol::SymbolIsConcatSpreadable
                | WellKnownSymbol::SymbolMatch
                | WellKnownSymbol::SymbolMatchAll
                | WellKnownSymbol::SymbolReplace
                | WellKnownSymbol::SymbolSearch
                | WellKnownSymbol::SymbolSpecies
                | WellKnownSymbol::SymbolSplit
                | WellKnownSymbol::SymbolToPrimitive
                | WellKnownSymbol::SymbolToStringTag
                | WellKnownSymbol::SymbolUnscopables
                | WellKnownSymbol::SymbolDispose
                | WellKnownSymbol::SymbolAsyncDispose
        )
    }
}
