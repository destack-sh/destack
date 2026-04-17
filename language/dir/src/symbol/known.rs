use destack_source::AdaptImage;
use serde::{Deserialize, Serialize};

/// Compiler known builtin symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, AdaptImage)]
#[allow(clippy::upper_case_acronyms)]
pub enum WellKnownSymbol {
    /// Builtin Array constructor symbol.
    Array,
    /// Builtin FixedArray type alias symbol.
    FixedArray,
    /// Builtin ReadonlyArray constructor symbol.
    ReadonlyArray,
    /// Builtin Map constructor symbol.
    Map,
    /// Builtin Set constructor symbol.
    Set,
    /// Builtin Record type symbol.
    Record,
    /// Builtin Slice type symbol.
    Slice,
    /// Builtin Object constructor symbol.
    Object,
    /// Builtin Function constructor symbol.
    Function,
    /// Builtin eval function symbol.
    Eval,
    /// Builtin String constructor symbol.
    String,
    /// Builtin Number constructor symbol.
    Number,
    /// Builtin Boolean constructor symbol.
    Boolean,
    /// Builtin BigInt constructor symbol.
    BigInt,
    /// Builtin Proxy constructor symbol.
    Proxy,
    /// Builtin Reflect namespace symbol.
    Reflect,
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
    /// Builtin Vector type symbol.
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
            WellKnownSymbol::Object => "Object",
            WellKnownSymbol::Function => "Function",
            WellKnownSymbol::Eval => "eval",
            WellKnownSymbol::String => "String",
            WellKnownSymbol::Number => "Number",
            WellKnownSymbol::Boolean => "Boolean",
            WellKnownSymbol::BigInt => "BigInt",
            WellKnownSymbol::Proxy => "Proxy",
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
            WellKnownSymbol::Object,
            WellKnownSymbol::Function,
            WellKnownSymbol::Eval,
            WellKnownSymbol::String,
            WellKnownSymbol::Number,
            WellKnownSymbol::Boolean,
            WellKnownSymbol::BigInt,
            WellKnownSymbol::Proxy,
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

/// Compiler known decorator markers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, AdaptImage)]
pub enum WellKnownDecorator {
    /// The `@binding` decorator marker.
    Binding,
    /// The `@extern` decorator marker.
    Extern,
    /// The `@intrinsic` decorator marker.
    Intrinsic,
    /// The `@deprecated` decorator marker.
    Deprecated,
    /// The `@experimental` decorator marker.
    Experimental,
    /// The `@allow` decorator marker.
    Allow,
    /// The `@warn` decorator marker.
    Warn,
    /// The `@deny` decorator marker.
    Deny,
    /// The `@forbid` decorator marker.
    Forbid,
    /// The `@expect` decorator marker.
    Expect,
    /// The `@languageItem` decorator marker.
    LanguageItem,
    /// The `@noManaged` decorator marker.
    NoManaged,
    /// The `@stackOnly` decorator marker.
    StackOnly,
    /// The `@capture` decorator marker.
    Capture,
    /// The `@inline` decorator marker.
    Inline,
    /// The `@noinline` decorator marker.
    Noinline,
    /// The `@unroll` decorator marker.
    Unroll,
    /// The `@hot` decorator marker.
    Hot,
    /// The `@cold` decorator marker.
    Cold,
    /// The `@likely` decorator marker.
    Likely,
    /// The `@unlikely` decorator marker.
    Unlikely,
    /// The `@mustUse` decorator marker.
    MustUse,
    /// The `@pure` decorator marker.
    Pure,
    /// The `@tailcall` decorator marker.
    Tailcall,
    /// The `@unsafe` decorator marker.
    Unsafe,
    /// The `@transmute` decorator marker.
    Transmute,
    /// The `@taint` decorator marker.
    Taint,
    /// The `@sink` decorator marker.
    Sink,
    /// The `@sanitizer` decorator marker.
    Sanitizer,
    /// The `@tag` decorator marker.
    Tag,
    /// The `@lifetime` decorator marker.
    Lifetime,
    /// The `@space` decorator marker.
    Space,
}

impl WellKnownDecorator {
    /// Return the module path for well known decorators.
    pub fn module(&self) -> &'static str {
        match self {
            WellKnownDecorator::Binding => "intrinsic/binding",
            _ => "intrinsic/decorator",
        }
    }

    /// Return the export name for a decorator marker.
    pub fn export_name(&self) -> &'static str {
        match self {
            WellKnownDecorator::Binding => "binding",
            WellKnownDecorator::Extern => "extern",
            WellKnownDecorator::Intrinsic => "intrinsic",
            WellKnownDecorator::Deprecated => "deprecated",
            WellKnownDecorator::Experimental => "experimental",
            WellKnownDecorator::Allow => "allow",
            WellKnownDecorator::Warn => "warn",
            WellKnownDecorator::Deny => "deny",
            WellKnownDecorator::Forbid => "forbid",
            WellKnownDecorator::Expect => "expect",
            WellKnownDecorator::LanguageItem => "languageItem",
            WellKnownDecorator::NoManaged => "noManaged",
            WellKnownDecorator::StackOnly => "stackOnly",
            WellKnownDecorator::Capture => "capture",
            WellKnownDecorator::Inline => "inline",
            WellKnownDecorator::Noinline => "noinline",
            WellKnownDecorator::Unroll => "unroll",
            WellKnownDecorator::Hot => "hot",
            WellKnownDecorator::Cold => "cold",
            WellKnownDecorator::Likely => "likely",
            WellKnownDecorator::Unlikely => "unlikely",
            WellKnownDecorator::MustUse => "mustUse",
            WellKnownDecorator::Pure => "pure",
            WellKnownDecorator::Tailcall => "tailcall",
            WellKnownDecorator::Unsafe => "unsafe",
            WellKnownDecorator::Transmute => "transmute",
            WellKnownDecorator::Taint => "taint",
            WellKnownDecorator::Sink => "sink",
            WellKnownDecorator::Sanitizer => "sanitizer",
            WellKnownDecorator::Tag => "tag",
            WellKnownDecorator::Lifetime => "lifetime",
            WellKnownDecorator::Space => "space",
        }
    }

    /// Iterate over all well known decorator markers.
    pub fn all() -> impl Iterator<Item = Self> {
        const ALL: &[WellKnownDecorator] = &[
            WellKnownDecorator::Binding,
            WellKnownDecorator::Extern,
            WellKnownDecorator::Intrinsic,
            WellKnownDecorator::Deprecated,
            WellKnownDecorator::Experimental,
            WellKnownDecorator::Allow,
            WellKnownDecorator::Warn,
            WellKnownDecorator::Deny,
            WellKnownDecorator::Forbid,
            WellKnownDecorator::Expect,
            WellKnownDecorator::LanguageItem,
            WellKnownDecorator::NoManaged,
            WellKnownDecorator::StackOnly,
            WellKnownDecorator::Capture,
            WellKnownDecorator::Inline,
            WellKnownDecorator::Noinline,
            WellKnownDecorator::Unroll,
            WellKnownDecorator::Hot,
            WellKnownDecorator::Cold,
            WellKnownDecorator::Likely,
            WellKnownDecorator::Unlikely,
            WellKnownDecorator::MustUse,
            WellKnownDecorator::Pure,
            WellKnownDecorator::Tailcall,
            WellKnownDecorator::Unsafe,
            WellKnownDecorator::Transmute,
            WellKnownDecorator::Taint,
            WellKnownDecorator::Sink,
            WellKnownDecorator::Sanitizer,
            WellKnownDecorator::Tag,
            WellKnownDecorator::Lifetime,
            WellKnownDecorator::Space,
        ];
        ALL.iter().copied()
    }
}

/// Compiler known Symbol.* keys.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, AdaptImage,
)]
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
