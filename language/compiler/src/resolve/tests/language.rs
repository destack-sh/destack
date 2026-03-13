use super::*;
use destack_builtin::LanguageSymbol;
use destack_dir::{WellKnownSymbol, WellKnownSymbolKey};

/// Test that language item modules can be looked up correctly.
#[test]
fn test_resolve_language_symbol() {
    let test = TestProgram::memory_sequential_with_prelude();
    test.resolve_language_environment();
    test.compile();

    let profile = test.default_profile_id_for_root();
    let symbol_id = test.compiler.language_symbol(profile, LanguageSymbol::Add);
    let dir = test.dir_base(symbol_id.module_id);
    let symbols = dir.symbols.read();
    let symbol = symbols.get_symbol(symbol_id.into_local());
    assert_string!(test.program, symbol.name().unwrap(), "Add");
}

/// Report conflicts when multiple builtin lib versions are requested.
#[test]
fn test_error_on_conflicting_builtin_lib_versions() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs()
        .with_profile_libs(&["node", "node.v24"]);
    test.resolve_language_environment();
    test.resolve_libs();
    test.compile();
    test.check_has_diagnostic("ER402");
}

/// Resolve well known symbols from builtin libs.
#[test]
fn test_resolve_well_known_symbols() {
    let test =
        TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&["es2020", "js"]);
    test.resolve_language_environment();
    test.resolve_libs();
    test.compile();

    let profile = test.default_profile_id_for_root();
    let well_known = test
        .compiler
        .get_well_known_symbols(profile)
        .unwrap_or_else(|| panic!("missing well known symbols for test profile"));

    // verify common baseline symbols
    let required_symbols = [
        WellKnownSymbol::Array,
        WellKnownSymbol::ReadonlyArray,
        WellKnownSymbol::Map,
        WellKnownSymbol::Set,
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
    ];
    for symbol in required_symbols {
        assert!(
            well_known.get_symbol(symbol).is_some(),
            "missing well-known symbol {symbol:?}"
        );
    }

    // verify symbol key metadata
    for symbol in WellKnownSymbolKey::all() {
        let key = well_known
            .get_key(symbol)
            .unwrap_or_else(|| panic!("missing well-known key {symbol:?}"));
        assert_string!(test.program, key.member, symbol.member_name());
        assert_string!(test.program, key.global_name, symbol.global_symbol_name());
    }
}

/// Resolve native-only well-known symbols when the native lib is active.
#[test]
fn test_resolve_native_well_known_fixed_array_symbol() {
    let test =
        TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&["native"]);
    test.resolve_language_environment();
    test.resolve_libs();
    test.compile();

    let profile = test.default_profile_id_for_root();
    let well_known = test
        .compiler
        .get_well_known_symbols(profile)
        .unwrap_or_else(|| panic!("missing well known symbols for test profile"));
    assert!(
        well_known
            .get_type_symbol(WellKnownSymbol::FixedArray)
            .is_some(),
        "missing native FixedArray well-known symbol"
    );
}
