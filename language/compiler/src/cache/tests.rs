use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use destack_source::{
    DiagnosticSeverity, File, FileVersion, ModuleId, ModuleVersion, PackageId, ProfileId,
    ProfileVersion,
};
use destack_workspace::{
    CacheMode, CachePolicy, CacheScope, CacheValidate, ModuleAst, ModuleGraphKey, ModuleMir,
    ModuleSignatureKey, TargetId,
};

use crate::{
    AnalyzeTask, CacheContext, CacheOptions, CacheRegistry, Compiler, TestFileSystem, TestProgram,
};

fn compile_analyze_modules(test: &TestProgram, modules: &[destack_source::ModuleId]) {
    // reset diagnostics for a clean assertion pass
    let _ = test.program.diagnostics.drain();

    // build a fresh compiler so tasks rerun
    let options = test.compiler.options.clone();
    let compiler = Compiler::new(test.session.clone(), test.program.clone(), options);

    // enqueue analyze tasks for the requested modules
    for module_id in modules {
        let profile = test.default_profile_id(*module_id);
        compiler.enqueue(AnalyzeTask::AnalyzeModuleValidate {
            module: *module_id,
            profile,
        });
    }

    // run compilation and check diagnostics
    compiler.compile();
    test.check_no_diagnostic(DiagnosticSeverity::Note);
}

fn replace_module_source(test: &TestProgram, module_id: destack_source::ModuleId, content: &str) {
    // capture the module path for filesystem updates
    let module_path = {
        let module = test.program.modules.get(module_id);
        let module = module.read();
        if !module.is_code() {
            panic!("expected a code module for source replacement");
        }
        module
            .path
            .clone()
            .unwrap_or_else(|| panic!("module path missing for {module_id:?}"))
    };

    // update the memory filesystem content
    match &test.fs {
        TestFileSystem::Memory { fs } => {
            fs.add_file(&module_path, content.as_bytes())
                .unwrap_or_else(|error| panic!("failed to write module file: {error}"));
        }
        TestFileSystem::Physical { .. } => {
            panic!("incremental tests require memory filesystem");
        }
    }

    // replace the file registry entry with an updated version
    let file = test.file(module_id);
    let next_version = file.version.next();
    let mut updated_file = File::from_text(
        file.id,
        file.name.clone(),
        file.uri.clone(),
        file.path.clone(),
        file.ty,
        content.to_string(),
    );
    updated_file.version = next_version;
    test.program.files.replace(updated_file);

    // clear module caches so import and analyze rerun
    let module = test.program.modules.get(module_id);
    let mut module = module.write();
    module.version = module.version.next();
    module.source_version = next_version;

    let code = module.code_mut();
    code.ast = None;
    code.dir_base = None;
    code.dirs.clear();
    code.comptimes.clear();
    code.mirs.clear();
}

/// Signature changes invalidate dependent modules.
#[test]
fn test_module_signature_invalidation() {
    let test = TestProgram::memory_sequential();
    let module_a_id = test.add_module(
        "a.ts",
        r#"
export const value: number = 1;
"#,
    );
    let module_b_id = test.add_module(
        "b.ts",
        r#"
import { value } from "./a.ts";

value;
"#,
    );

    // seed signatures and module graph
    compile_analyze_modules(&test, &[module_a_id, module_b_id]);

    let profile = test.default_profile_id(module_a_id);
    let signature_key = ModuleSignatureKey::new(module_a_id, profile);
    let initial_signature = test
        .program
        .index
        .module_signatures
        .get(&signature_key)
        .unwrap_or_else(|| panic!("missing signature for {module_a_id:?}"));
    let initial_hash = initial_signature.value().hash;
    let b_has_dir_before = test
        .program
        .modules
        .get(module_b_id)
        .read()
        .dir_maybe(profile)
        .is_some();
    let b_signature_before = test
        .program
        .index
        .module_signatures
        .get(&ModuleSignatureKey::new(module_b_id, profile))
        .unwrap_or_else(|| panic!("missing signature for {module_b_id:?}"));
    let b_signature_hash_before = b_signature_before.value().hash;
    drop(initial_signature);
    drop(b_signature_before);

    // update module a without changing its export surface
    replace_module_source(
        &test,
        module_a_id,
        r#"
export const value: number = 2;
"#,
    );
    compile_analyze_modules(&test, &[module_a_id]);

    let stable_signature = test
        .program
        .index
        .module_signatures
        .get(&signature_key)
        .unwrap_or_else(|| panic!("missing signature for {module_a_id:?}"));
    let stable_hash = stable_signature.value().hash;
    let b_has_dir_after_internal = test
        .program
        .modules
        .get(module_b_id)
        .read()
        .dir_maybe(profile)
        .is_some();
    let b_signature_after_internal = test
        .program
        .index
        .module_signatures
        .get(&ModuleSignatureKey::new(module_b_id, profile))
        .unwrap_or_else(|| panic!("missing signature for {module_b_id:?}"));
    let b_signature_hash_after_internal = b_signature_after_internal.value().hash;
    drop(stable_signature);
    drop(b_signature_after_internal);

    // assertion block
    assert_eq!(
        stable_hash, initial_hash,
        "expected signature hash to remain stable for internal change"
    );
    assert_eq!(
        b_has_dir_after_internal, b_has_dir_before,
        "expected dependent dir preserved for internal change"
    );
    assert_eq!(
        b_signature_hash_after_internal, b_signature_hash_before,
        "expected dependent signature hash preserved for internal change"
    );

    // update module a with an export change
    replace_module_source(
        &test,
        module_a_id,
        r#"
export const value: string = "value";
"#,
    );
    compile_analyze_modules(&test, &[module_a_id]);

    let changed_signature = test
        .program
        .index
        .module_signatures
        .get(&signature_key)
        .unwrap_or_else(|| panic!("missing signature for {module_a_id:?}"));
    let changed_hash = changed_signature.value().hash;
    let b_has_dir_after_export = test
        .program
        .modules
        .get(module_b_id)
        .read()
        .dir_maybe(profile)
        .is_some();
    let b_signature_after_export = test
        .program
        .index
        .module_signatures
        .get(&ModuleSignatureKey::new(module_b_id, profile))
        .unwrap_or_else(|| panic!("missing signature for {module_b_id:?}"));
    let b_signature_hash_after_export = b_signature_after_export.value().hash;

    // assertion block
    assert_ne!(
        changed_hash, initial_hash,
        "expected signature hash to change when exports change"
    );
    assert!(
        !b_has_dir_after_export,
        "expected dependent dir invalidated on export change"
    );
    assert_eq!(
        b_signature_hash_after_export, b_signature_hash_before,
        "expected dependent signature retained for export change"
    );
}

/// Module graph updates when imports change.
#[test]
fn test_module_graph_updates_on_import_change() {
    let test = TestProgram::memory_sequential();
    let module_b_id = test.add_module(
        "b.ts",
        r#"
export const value: number = 1;
"#,
    );
    let module_c_id = test.add_module(
        "c.ts",
        r#"
export const value: number = 2;
"#,
    );
    let module_a_id = test.add_module(
        "a.ts",
        r#"
import { value } from "./b.ts";

value;
"#,
    );

    // seed module graph
    compile_analyze_modules(&test, &[module_a_id, module_b_id, module_c_id]);

    let profile = test.default_profile_id(module_a_id);
    let graph_key = ModuleGraphKey::new(profile);
    let graph = test
        .program
        .index
        .module_graphs
        .get(&graph_key)
        .unwrap_or_else(|| panic!("missing module graph for {profile:?}"));
    let deps_before: HashSet<_> = graph.dependencies_for(module_a_id).into_iter().collect();
    let dependents_b_before: HashSet<_> =
        graph.dependents_for(module_b_id).into_iter().collect();
    let dependents_c_before: HashSet<_> =
        graph.dependents_for(module_c_id).into_iter().collect();
    drop(graph);

    // assertion block
    assert!(
        deps_before.contains(&module_b_id),
        "expected module b to be a dependency before edits"
    );
    assert!(
        !deps_before.contains(&module_c_id),
        "expected module c to be absent before edits"
    );
    assert!(
        dependents_b_before.contains(&module_a_id),
        "expected module a to depend on module b before edits"
    );
    assert!(
        dependents_c_before.is_empty(),
        "expected module c to have no dependents before edits"
    );

    // update module a to import module c instead
    replace_module_source(
        &test,
        module_a_id,
        r#"
import { value } from "./c.ts";

value;
"#,
    );
    compile_analyze_modules(&test, &[module_a_id, module_c_id]);

    let graph = test
        .program
        .index
        .module_graphs
        .get(&graph_key)
        .unwrap_or_else(|| panic!("missing module graph for {profile:?}"));
    let deps_after: HashSet<_> = graph.dependencies_for(module_a_id).into_iter().collect();
    let dependents_b_after: HashSet<_> =
        graph.dependents_for(module_b_id).into_iter().collect();
    let dependents_c_after: HashSet<_> =
        graph.dependents_for(module_c_id).into_iter().collect();
    drop(graph);

    // assertion block
    assert!(
        deps_after.contains(&module_c_id),
        "expected module c to be a dependency after edits"
    );
    assert!(
        !deps_after.contains(&module_b_id),
        "expected module b to be removed after edits"
    );
    assert!(
        !dependents_b_after.contains(&module_a_id),
        "expected module a removed from module b dependents"
    );
    assert!(
        dependents_c_after.contains(&module_a_id),
        "expected module a to depend on module c after edits"
    );
}

/// Signature changes propagate through reexports.
#[test]
fn test_module_signature_transitive_invalidation() {
    let test = TestProgram::memory_sequential();
    let module_b_id = test.add_module(
        "b.ts",
        r#"
export const value: number = 1;
"#,
    );
    let module_a_id = test.add_module(
        "a.ts",
        r#"
export { value } from "./b.ts";
"#,
    );
    let module_c_id = test.add_module(
        "c.ts",
        r#"
import { value } from "./a.ts";

value;
"#,
    );

    // seed signatures and module graph
    compile_analyze_modules(&test, &[module_b_id, module_a_id, module_c_id]);

    let profile = test.default_profile_id(module_a_id);
    let a_signature_before = test
        .program
        .index
        .module_signatures
        .get(&ModuleSignatureKey::new(module_a_id, profile))
        .unwrap_or_else(|| panic!("missing signature for {module_a_id:?}"));
    let a_hash_before = a_signature_before.value().hash;
    let c_has_dir_before = test
        .program
        .modules
        .get(module_c_id)
        .read()
        .dir_maybe(profile)
        .is_some();
    drop(a_signature_before);

    // update module b with export change
    replace_module_source(
        &test,
        module_b_id,
        r#"
export const value: string = "value";
"#,
    );
    compile_analyze_modules(&test, &[module_b_id]);

    let a_has_dir_after_b = test
        .program
        .modules
        .get(module_a_id)
        .read()
        .dir_maybe(profile)
        .is_some();
    let c_has_dir_after_b = test
        .program
        .modules
        .get(module_c_id)
        .read()
        .dir_maybe(profile)
        .is_some();
    let a_signature_after_b = test
        .program
        .index
        .module_signatures
        .get(&ModuleSignatureKey::new(module_a_id, profile))
        .unwrap_or_else(|| panic!("missing signature for {module_a_id:?}"));
    let a_hash_after_b = a_signature_after_b.value().hash;
    drop(a_signature_after_b);

    // assertion block
    assert!(
        !a_has_dir_after_b,
        "expected reexporting module dir invalidated by dependency change"
    );
    assert!(
        c_has_dir_after_b && c_has_dir_before,
        "expected dependent dir retained before signature recompute"
    );
    assert_eq!(
        a_hash_after_b, a_hash_before,
        "expected signature retained until recompute"
    );

    // recompute module a signature to propagate invalidation
    compile_analyze_modules(&test, &[module_a_id]);

    let a_signature_after_a = test
        .program
        .index
        .module_signatures
        .get(&ModuleSignatureKey::new(module_a_id, profile))
        .unwrap_or_else(|| panic!("missing signature for {module_a_id:?}"));
    let a_hash_after_a = a_signature_after_a.value().hash;
    let c_has_dir_after_a = test
        .program
        .modules
        .get(module_c_id)
        .read()
        .dir_maybe(profile)
        .is_some();

    // assertion block
    assert_ne!(
        a_hash_after_a, a_hash_before,
        "expected signature change after reexport update"
    );
    assert!(
        !c_has_dir_after_a,
        "expected dependent dir invalidated after signature update"
    );
}

/// Signature changes clear dependent MIR caches.
#[test]
fn test_module_signature_clears_dependent_mir() {
    let test = TestProgram::memory_sequential();
    let module_a_id = test.add_module(
        "a.ts",
        r#"
export const value: number = 1;
"#,
    );
    let module_b_id = test.add_module(
        "b.ts",
        r#"
import { value } from "./a.ts";

value;
"#,
    );

    // seed module graph and signatures
    compile_analyze_modules(&test, &[module_a_id, module_b_id]);

    // seed a mir entry to validate invalidation behavior
    let package_id = test.program.modules.get(module_b_id).read().package_id;
    let target_id = TargetId::new(package_id, "native");
    {
        let module = test.program.modules.get(module_b_id);
        let mut module = module.write();
        let module_version = module.version;
        module.code_mut().mirs.push(ModuleMir::new(
            module_b_id,
            module_version,
            target_id.clone(),
        ));
    }
    let b_has_mir_before = test
        .program
        .modules
        .get(module_b_id)
        .read()
        .code()
        .mirs
        .iter()
        .any(|mir| mir.target == target_id);

    // assertion block
    assert!(b_has_mir_before, "expected mir to be seeded before edits");

    // update module a with an export change
    replace_module_source(
        &test,
        module_a_id,
        r#"
export const value: string = "value";
"#,
    );
    compile_analyze_modules(&test, &[module_a_id]);

    let b_has_mir_after = test
        .program
        .modules
        .get(module_b_id)
        .read()
        .code()
        .mirs
        .iter()
        .any(|mir| mir.target == target_id);

    // assertion block
    assert!(
        !b_has_mir_after,
        "expected dependent mir cleared after signature change"
    );
}

/// Cache entries roundtrip through disk storage.
#[test]
fn test_cache_roundtrip_disk() {
    let cache_root = std::env::temp_dir().join(format!(
        "destack-cache-test-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&cache_root)
        .unwrap_or_else(|error| panic!("failed to create cache dir: {error}"));

    let options = CacheOptions {
        mode: CacheMode::Disk,
        dir: cache_root.clone(),
        policy: CachePolicy::Lru,
        validate: CacheValidate::Strict,
        scope: CacheScope::Workspace,
        max_size_mb: None,
    };
    let context = CacheContext {
        compiler_version: "test".to_string(),
        file_version: FileVersion::INITIAL,
        profile_id: ProfileId::new(0),
        profile_version: ProfileVersion::INITIAL,
        source_hash: 1,
        config_hash: 2,
        target_hash: 3,
        dependency_hash: 0,
    };
    let module_id = ModuleId::new(PackageId::new(1), 1);
    let payload = ModuleAst::new(module_id, ModuleVersion::INITIAL).to_data();

    let registry = CacheRegistry::new();
    registry
        .write_ast_cache(&options, &context, module_id, payload.clone())
        .unwrap_or_else(|error| panic!("failed to write cache entry: {error}"));

    // assertion block
    let entry = registry
        .read_ast_cache(&options, &context, module_id)
        .unwrap_or_else(|error| panic!("failed to read cache entry: {error}"));
    assert!(entry.is_some(), "expected ast cache entry");

    let fresh_registry = CacheRegistry::new();
    let entry = fresh_registry
        .read_ast_cache(&options, &context, module_id)
        .unwrap_or_else(|error| panic!("failed to read cache entry: {error}"));

    // assertion block
    assert!(entry.is_some(), "expected ast cache entry from disk");

    let _ = std::fs::remove_dir_all(cache_root);
}

/// Memory cache entries roundtrip within the same registry.
#[test]
fn test_cache_roundtrip_memory() {
    let options = CacheOptions {
        mode: CacheMode::Memory,
        dir: std::env::temp_dir(),
        policy: CachePolicy::Lru,
        validate: CacheValidate::Strict,
        scope: CacheScope::Workspace,
        max_size_mb: None,
    };
    let context = CacheContext {
        compiler_version: "test".to_string(),
        file_version: FileVersion::INITIAL,
        profile_id: ProfileId::new(0),
        profile_version: ProfileVersion::INITIAL,
        source_hash: 1,
        config_hash: 2,
        target_hash: 3,
        dependency_hash: 0,
    };
    let module_id = ModuleId::new(PackageId::new(2), 2);
    let payload = ModuleAst::new(module_id, ModuleVersion::INITIAL).to_data();

    let registry = CacheRegistry::new();
    registry
        .write_ast_cache(&options, &context, module_id, payload)
        .unwrap_or_else(|error| panic!("failed to write cache entry: {error}"));

    // assertion block
    let entry = registry
        .read_ast_cache(&options, &context, module_id)
        .unwrap_or_else(|error| panic!("failed to read cache entry: {error}"));
    assert!(entry.is_some(), "expected ast cache entry");

    // assertion block
    let fresh_registry = CacheRegistry::new();
    let entry = fresh_registry
        .read_ast_cache(&options, &context, module_id)
        .unwrap_or_else(|error| panic!("failed to read cache entry: {error}"));
    assert!(
        entry.is_none(),
        "expected no ast cache entry in fresh registry"
    );
}

/// Cache entries are invalidated when the context changes.
#[test]
fn test_cache_miss_on_context_change() {
    let cache_root = std::env::temp_dir().join(format!(
        "destack-cache-test-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&cache_root)
        .unwrap_or_else(|error| panic!("failed to create cache dir: {error}"));

    let options = CacheOptions {
        mode: CacheMode::Disk,
        dir: cache_root.clone(),
        policy: CachePolicy::Lru,
        validate: CacheValidate::Strict,
        scope: CacheScope::Workspace,
        max_size_mb: None,
    };
    let context = CacheContext {
        compiler_version: "test".to_string(),
        file_version: FileVersion::INITIAL,
        profile_id: ProfileId::new(0),
        profile_version: ProfileVersion::INITIAL,
        source_hash: 1,
        config_hash: 2,
        target_hash: 3,
        dependency_hash: 0,
    };
    let module_id = ModuleId::new(PackageId::new(3), 3);
    let payload = ModuleAst::new(module_id, ModuleVersion::INITIAL).to_data();

    let registry = CacheRegistry::new();
    registry
        .write_ast_cache(&options, &context, module_id, payload)
        .unwrap_or_else(|error| panic!("failed to write cache entry: {error}"));

    let mismatched_context = CacheContext {
        compiler_version: "test".to_string(),
        file_version: FileVersion::INITIAL,
        profile_id: ProfileId::new(0),
        profile_version: ProfileVersion::INITIAL,
        source_hash: 10,
        config_hash: 2,
        target_hash: 3,
        dependency_hash: 0,
    };

    // assertion block
    let entry = registry
        .read_ast_cache(&options, &mismatched_context, module_id)
        .unwrap_or_else(|error| panic!("failed to read cache entry: {error}"));
    assert!(
        entry.is_none(),
        "expected cache miss for mismatched context"
    );

    let _ = std::fs::remove_dir_all(cache_root);
}

/// Cache entries are invalidated when dependency hashes change.
#[test]
fn test_cache_miss_on_dependency_change() {
    let options = CacheOptions {
        mode: CacheMode::Memory,
        dir: std::env::temp_dir(),
        policy: CachePolicy::Lru,
        validate: CacheValidate::Strict,
        scope: CacheScope::Workspace,
        max_size_mb: None,
    };
    let context = CacheContext {
        compiler_version: "test".to_string(),
        file_version: FileVersion::INITIAL,
        profile_id: ProfileId::new(0),
        profile_version: ProfileVersion::INITIAL,
        source_hash: 1,
        config_hash: 2,
        target_hash: 3,
        dependency_hash: 10,
    };
    let module_id = ModuleId::new(PackageId::new(4), 4);
    let payload = ModuleAst::new(module_id, ModuleVersion::INITIAL).to_data();

    let registry = CacheRegistry::new();
    registry
        .write_ast_cache(&options, &context, module_id, payload)
        .unwrap_or_else(|error| panic!("failed to write cache entry: {error}"));

    let mismatched_context = CacheContext {
        compiler_version: "test".to_string(),
        file_version: FileVersion::INITIAL,
        profile_id: ProfileId::new(0),
        profile_version: ProfileVersion::INITIAL,
        source_hash: 1,
        config_hash: 2,
        target_hash: 3,
        dependency_hash: 999,
    };

    // assertion block
    let entry = registry
        .read_ast_cache(&options, &mismatched_context, module_id)
        .unwrap_or_else(|error| panic!("failed to read cache entry: {error}"));
    assert!(
        entry.is_none(),
        "expected dependency hash mismatch to invalidate cache entry"
    );
}
