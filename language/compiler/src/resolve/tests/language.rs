use super::*;
use crate::{run_to_completion, LANGUAGE_LIBS, LIBRARY_PACKAGES, LibraryPackage};
use destack_dir::{
    Declaration, LanguageSymbol, StaticKey, SymbolSpace, SymbolType, WellKnownSymbol,
    WellKnownSymbolKey,
};
use destack_source::DiagnosticSeverity;

/// Analyze one library package and summarize any diagnostics.
fn analyze_library_summary(library: &LibraryPackage) -> Option<String> {
    // selected library package
    let test =
        TestProgram::memory_sequential_with_prelude();
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

/// Analyze every library package in one registry and report failures.
fn analyze_library_registry_clean(libraries: &[LibraryPackage]) {
    let mut failures = Vec::new();

    for library in libraries {
        if let Some(summary) = analyze_library_summary(library) {
            failures.push(summary);
        }
    }

    assert!(
        failures.is_empty(),
        "library packages must stay clean:\n{}",
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

// FUGU #Broken: many library package entries are not yet standalone clean in isolation
#[test]
#[ignore = "library catalog inventory for standalone clean-analysis debt"]
fn test_analyze_language_libraries_clean() {
    analyze_library_registry_clean(LANGUAGE_LIBS);
}

// FUGU #Broken: many library package entries are not yet standalone clean in isolation
#[test]
#[ignore = "library catalog inventory for standalone clean-analysis debt"]
fn test_analyze_library_packages_clean() {
    analyze_library_registry_clean(LIBRARY_PACKAGES);
}

/// Resolve well known symbols from library packages.
#[test]
fn test_resolve_well_known_symbols() {
    let test =
        TestProgram::memory_sequential_with_prelude();
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

/// Resolve fixed array from the core library.
#[test]
fn test_resolve_core_well_known_fixed_array_symbol() {
    let test =
        TestProgram::memory_sequential_with_prelude();
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
        "missing core FixedArray well-known symbol"
    );
}

/// Resolve ambient bare core globals.
#[test]
fn test_resolve_bare_core_globals() {
    let test =
        TestProgram::memory_sequential_with_prelude();
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

/// Resolve the core String well-known symbol for an implicit target profile.
#[test]
fn test_resolve_core_well_known_string_symbol_from_target_profile() {
    let test = TestProgram::memory_sequential_with_prelude();
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
        .unwrap_or_else(|| panic!("missing target profile"));

    run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, context| compiler.require_ambient_environment(context, profile),
    )
    .unwrap_or_else(|error| panic!("failed to resolve core library environment: {error:?}"));
    test.compile();

    let well_known = test
        .compiler
        .get_well_known_symbols(test.program.current_revision(), profile)
        .unwrap_or_else(|| panic!("missing well known symbols for target profile"));
    assert!(
        well_known
            .get_type_symbol(WellKnownSymbol::String)
            .is_some(),
        "missing core String well-known symbol"
    );
}

/// Resolve the core String well-known symbol to one concrete struct declaration.
#[test]
fn test_resolve_native_well_known_string_concrete_symbol() {
    let test = TestProgram::memory_sequential_with_prelude();
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
        .unwrap_or_else(|| panic!("missing core String well-known symbol"));

    run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, _context| {
            compiler.require_dir_declared(_context, string_symbol.module_id, profile)
        },
    )
    .unwrap_or_else(|error| panic!("failed to declare core String owner module: {error:?}"));

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
        "core String symbol: {string_symbol:?}, type sources: {string_sources:?}"
    );

    let declaration_id = declared_symbol
        .primary_declaration
        .and_then(|node| node.local_id.try_into_typed::<Declaration>().ok())
        .unwrap_or_else(|| panic!("core String symbol missing struct declaration"));
    let Declaration::Struct { .. } = declared.tree.get(declaration_id) else {
        panic!("core String symbol does not point to a struct declaration");
    };
}
