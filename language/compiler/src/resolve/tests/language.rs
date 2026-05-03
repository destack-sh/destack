use super::*;
use crate::run_to_completion;
use destack_builtin::{BuiltinLibrary, LANGUAGE_LIBS, LIBRARY_LIBS, LanguageSymbol};
use destack_dir::{
    Declaration, StaticKey, SymbolSpace, SymbolType, WellKnownSymbol, WellKnownSymbolKey,
};
use destack_source::DiagnosticSeverity;

/// Analyze one builtin library and summarize any diagnostics.
fn analyze_builtin_library_summary(library: &BuiltinLibrary) -> Option<String> {
    // selected builtin library
    let test =
        TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&[library.name]);
    let profile = test.default_profile_id_for_root();

    let resolved = run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, context| compiler.require_ambient_environment(context, profile),
    );
    if let Err(error) = resolved {
        return Some(format!("{}: resolve error {error:?}", library.name));
    }

    let environment = test
        .repository
        .ambient_environment(test.program.current_revision(), profile)
        .unwrap_or_else(|| panic!("expected published library environment"));

    // analyze every selected module
    for module_id in environment.modules.iter().copied() {
        test.analyze_module(module_id);
    }

    test.compile();

    let diagnostics = test.diagnostics();
    let highest_severity = diagnostics.highest_severity();
    if highest_severity.is_none_or(|severity| severity < DiagnosticSeverity::Note) {
        return None;
    }

    Some(format!(
        "{}: {} diagnostics",
        library.name,
        diagnostics.len()
    ))
}

/// Analyze every builtin library in one registry and report failures.
fn analyze_builtin_library_registry_clean(libraries: &[BuiltinLibrary]) {
    let mut failures = Vec::new();

    for library in libraries {
        if let Some(summary) = analyze_builtin_library_summary(library) {
            failures.push(summary);
        }
    }

    assert!(
        failures.is_empty(),
        "builtin libraries must stay clean:\n{}",
        failures.join("\n")
    );
}

/// Test that language item modules can be looked up correctly.
#[test]
fn test_resolve_language_symbol() {
    let test = TestProgram::memory_sequential_with_prelude();
    test.resolve_language_environment();
    test.compile();

    let profile = test.default_profile_id_for_root();
    let revision = test.program.current_revision();
    let environment = test
        .repository
        .language_environment(revision, profile)
        .unwrap_or_else(|| panic!("missing language environment for test profile"));
    let symbol_id = environment
        .item(LanguageSymbol::Add)
        .unwrap_or_else(|| panic!("missing Add language symbol"));
    let dir = test
        .repository
        .dir_declared(revision, symbol_id.module_id, profile)
        .unwrap_or_else(|| panic!("missing language item DIR"));
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

// FUGU #Broken: many builtin library entries are not yet standalone clean in isolation
#[test]
#[ignore = "builtin registry inventory for standalone clean-analysis debt"]
fn test_analyze_language_builtin_libraries_clean() {
    analyze_builtin_library_registry_clean(LANGUAGE_LIBS);
}

// FUGU #Broken: many builtin library entries are not yet standalone clean in isolation
#[test]
#[ignore = "builtin registry inventory for standalone clean-analysis debt"]
fn test_analyze_library_builtin_libraries_clean() {
    analyze_builtin_library_registry_clean(LIBRARY_LIBS);
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
        .get_well_known_symbols(test.program.current_revision(), profile)
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
        .get_well_known_symbols(test.program.current_revision(), profile)
        .unwrap_or_else(|| panic!("missing well known symbols for test profile"));
    assert!(
        well_known
            .get_type_symbol(WellKnownSymbol::FixedArray)
            .is_some(),
        "missing native FixedArray well-known symbol"
    );
}

/// Resolve ambient bare JavaScript globals from the selected ES library surface.
#[test]
fn test_resolve_bare_javascript_globals_from_builtin_libraries() {
    let test =
        TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&["es2020", "js"]);
    let module_id = test.add_module(
        "main.ts",
        r#"
const infinityValue = Infinity;
const nanValue = NaN;
const rootValue = globalThis;

export const appValue = [infinityValue, nanValue, rootValue];
"#,
    );

    test.resolve_module(module_id);
    test.compile_check_clean();
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

    let package_id = test.program.module_descriptor(module_id).package_id;
    let target_id = test.target_id(package_id, "native");
    let profile = test
        .program
        .target_profile_id(module_id, &target_id)
        .unwrap_or_else(|| panic!("missing native profile"));

    run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, context| compiler.require_ambient_environment(context, profile),
    )
    .unwrap_or_else(|error| panic!("failed to resolve native library environment: {error:?}"));
    test.compile();

    let well_known = test
        .compiler
        .get_well_known_symbols(test.program.current_revision(), profile)
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

    let package_id = test.program.module_descriptor(module_id).package_id;
    let target_id = test.target_id(package_id, "native");
    let profile = test
        .program
        .target_profile_id(module_id, &target_id)
        .unwrap_or_else(|| panic!("missing native profile"));

    run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, context| compiler.require_ambient_environment(context, profile),
    )
    .unwrap_or_else(|error| panic!("failed to resolve native library environment: {error:?}"));
    test.compile();

    let string_name = test.program.strings.intern("String");
    let string_sources = test
        .compiler
        .get_library_symbol_sources(
            test.program.current_revision(),
            profile,
            StaticKey::Name(string_name),
            SymbolSpace::Type,
        )
        .unwrap_or_default();
    let string_symbol = test
        .compiler
        .get_well_known_concrete_symbol_from(
            test.program.current_revision(),
            profile,
            WellKnownSymbol::String,
            destack_dir::SymbolSpaceOrder::TypeThenValue,
        )
        .unwrap_or_else(|| panic!("missing native String well-known symbol"));

    run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, _context| {
            compiler.require_dir_declared(_context, string_symbol.module_id, profile)
        },
    )
    .unwrap_or_else(|error| panic!("failed to declare native String owner module: {error:?}"));

    let declared = test
        .compiler
        .dir_declared(
            crate::tests::test_provider_context(
                test.compiler.as_ref(),
                test.program.current_revision(),
                destack_artifact::ArtifactKey::WorkspaceLinted,
            )
            .as_ref(),
            string_symbol.module_id,
            profile,
        )
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
