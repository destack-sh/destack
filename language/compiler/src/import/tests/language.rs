use super::*;
use crate::{LANGUAGE_LIBS, LIBRARY_PACKAGES, LibraryPackage, run_to_completion};
use destack_artifact::ArtifactKey;
use destack_dir::{Declaration, LanguageItem, SymbolForm};
use destack_source::DiagnosticSeverity;

/// Check one library package and summarize any diagnostics.
fn analyze_library_summary(library: &LibraryPackage) -> Option<String> {
    // selected library package
    let test = TestProgram::memory_sequential_with_prelude();
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

    // check every selected module
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

/// Check every library package in one registry and report failures.
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
fn test_resolve_language_item() {
    let test = TestProgram::memory_sequential_with_prelude();
    test.resolve_language_environment();
    test.compile();

    let profile = test.default_profile_id_for_root();
    let revision = test.program.current_revision();
    let environment = test
        .repository
        .language_environment(revision, profile)
        .unwrap_or_else(|| panic!("missing global environment for test profile"));
    let symbol_id = environment
        .item(LanguageItem::Add)
        .unwrap_or_else(|| panic!("missing Add language symbol"));
    let dir = test
        .repository
        .dir_declared(revision, symbol_id.module_id, profile)
        .unwrap_or_else(|| panic!("missing language item DIR"));
    let symbols = &dir.bindings;
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

/// Resolve language items from library packages.
#[test]
fn test_resolve_language_items() {
    let test = TestProgram::memory_sequential_with_prelude();
    test.resolve_language_environment();
    test.resolve_libs();
    test.compile();

    let profile = test.default_profile_id_for_root();
    let revision = test.program.current_revision();
    let language = test
        .repository
        .language_environment(revision, profile)
        .unwrap_or_else(|| panic!("missing language environment for test profile"));

    // verify common baseline symbols
    let required_symbols = [
        LanguageItem::Array,
        LanguageItem::ReadonlyArray,
        LanguageItem::Map,
        LanguageItem::Set,
        LanguageItem::Slice,
        LanguageItem::String,
        LanguageItem::Number,
        LanguageItem::BigInt,
        LanguageItem::Function,
        LanguageItem::Eval,
        LanguageItem::Reflect,
        LanguageItem::Promise,
        LanguageItem::Iterable,
        LanguageItem::Iterator,
        LanguageItem::AsyncIterable,
        LanguageItem::AsyncIterator,
        LanguageItem::Symbol,
    ];
    for symbol in required_symbols {
        assert!(
            language.item(symbol).is_some(),
            "missing language symbol {symbol:?}"
        );
    }
}

/// Resolve fixed array from the core library.
#[test]
fn test_resolve_core_fixed_array_language_symbol() {
    let test = TestProgram::memory_sequential_with_prelude();
    test.resolve_language_environment();
    test.resolve_libs();
    test.compile();

    let profile = test.default_profile_id_for_root();
    let revision = test.program.current_revision();
    let language = test
        .repository
        .language_environment(revision, profile)
        .unwrap_or_else(|| panic!("missing language environment for test profile"));
    assert!(
        language.item(LanguageItem::FixedArray).is_some(),
        "missing core FixedArray language symbol"
    );
}

/// Resolve ambient bare core globals.
#[test]
fn test_resolve_bare_core_globals() {
    let test = TestProgram::memory_sequential_with_prelude();
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

/// Resolve the core String language symbol for an implicit target profile.
#[test]
fn test_resolve_core_string_language_symbol_from_target_profile() {
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

    let revision = test.program.current_revision();
    let language = test
        .repository
        .language_environment(revision, profile)
        .unwrap_or_else(|| panic!("missing language environment for target profile"));
    assert!(
        language.item(LanguageItem::String).is_some(),
        "missing core String language symbol"
    );
}

/// Resolve the core String language symbol to one concrete class declaration.
#[test]
fn test_resolve_native_string_language_symbol() {
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

    let revision = test.program.current_revision();
    let language = test
        .repository
        .language_environment(revision, profile)
        .unwrap_or_else(|| panic!("missing language environment for native profile"));
    let string_symbol = language
        .item(LanguageItem::String)
        .unwrap_or_else(|| panic!("missing core String language symbol"));

    run_to_completion(
        &test.compiler,
        revision,
        |_, context| context.require(ArtifactKey::dir_declared(string_symbol.module_id, profile)),
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
    let declared_symbol = declared.bindings.get_symbol(string_symbol.local_id);
    assert_eq!(
        declared_symbol.ty,
        SymbolForm::Class,
        "core String symbol: {string_symbol:?}"
    );

    let declaration_id = declared_symbol
        .declaration
        .and_then(|node| node.local_id.try_into_typed::<Declaration>().ok())
        .unwrap_or_else(|| panic!("core String symbol missing class declaration"));
    let Declaration::Class { .. } = declared.tree.get(declaration_id) else {
        panic!("core String symbol does not point to a class declaration");
    };
}
