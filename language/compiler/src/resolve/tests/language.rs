use super::*;
use destack_builtin::LanguageSymbol;
use destack_dir::{
    Declaration, StaticKey, SymbolSpace, SymbolType, WellKnownSymbol, WellKnownSymbolKey,
};
use destack_workspace::TargetId;

/// Test that language item modules can be looked up correctly.
#[test]
fn test_resolve_language_symbol() {
    let test = TestProgram::memory_sequential_with_prelude();
    test.resolve_language_environment();
    test.compile();

    let profile = test.default_profile_id_for_root();
    let symbol_id = test.compiler.language_symbol(profile, LanguageSymbol::Add);
    let dir = test.dir_base(symbol_id.module_id);
    let symbols = &dir.symbols;
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
        assert_eq!(key.member, symbol.member_name());
        assert_eq!(key.global_name, symbol.global_symbol_name());
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

/// Resolve the native String well-known symbol for an implicit native target profile.
#[test]
fn test_resolve_native_well_known_string_symbol_from_target_profile() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function main(): int32 {
    return 0;
}
"#,
    );
    test.add_target(module_id, "native");

    let package_id = test.program.modules.get(module_id).package_id;
    let target_id = TargetId::new(package_id, "native");
    let profile = test
        .program
        .profile_id_for_target(module_id, &target_id)
        .unwrap_or_else(|| panic!("missing native profile"));

    test.compiler
        .drive(|compiler| compiler.require_library_environment(profile))
        .unwrap_or_else(|error| panic!("failed to resolve native library environment: {error:?}"));
    test.compile();

    let well_known = test
        .compiler
        .get_well_known_symbols(profile)
        .unwrap_or_else(|| panic!("missing well known symbols for native target profile"));
    assert!(
        well_known
            .get_type_symbol(WellKnownSymbol::String)
            .is_some(),
        "missing native String well-known symbol"
    );
}

/// Resolve the native String well-known symbol to one concrete struct declaration.
#[test]
fn test_resolve_native_well_known_string_concrete_symbol() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function main(): int32 {
    return 0;
}
"#,
    );
    test.add_target(module_id, "native");

    let package_id = test.program.modules.get(module_id).package_id;
    let target_id = TargetId::new(package_id, "native");
    let profile = test
        .program
        .profile_id_for_target(module_id, &target_id)
        .unwrap_or_else(|| panic!("missing native profile"));

    test.compiler
        .drive(|compiler| compiler.require_library_environment(profile))
        .unwrap_or_else(|error| panic!("failed to resolve native library environment: {error:?}"));
    test.compile();

    let string_name = test.program.strings.intern("String");
    let string_sources = test
        .compiler
        .get_library_symbol_sources(profile, StaticKey::Name(string_name), SymbolSpace::Type)
        .unwrap_or_default();
    let string_symbol = test
        .compiler
        .get_well_known_concrete_symbol_from(
            profile,
            WellKnownSymbol::String,
            destack_dir::SymbolSpaceOrder::TypeThenValue,
        )
        .unwrap_or_else(|| panic!("missing native String well-known symbol"));

    let declared = test
        .compiler
        .require_artifact_dir_declared(string_symbol.module_id, profile)
        .unwrap_or_else(|error| panic!("missing declared dir for String symbol: {error:?}"));
    let declared_symbol = declared.symbols.get_symbol(string_symbol.local_id);
    assert_eq!(
        declared_symbol.ty,
        SymbolType::Struct,
        "native String symbol: {string_symbol:?}, type sources: {string_sources:?}"
    );

    let declaration_id = declared_symbol
        .primary_declaration
        .and_then(|node| node.local_id.try_into_typed::<Declaration>().ok())
        .unwrap_or_else(|| panic!("native String symbol missing struct declaration"));
    let Declaration::Struct { .. } = declared.tree.get(declaration_id) else {
        panic!("native String symbol does not point to a struct declaration");
    };
}
