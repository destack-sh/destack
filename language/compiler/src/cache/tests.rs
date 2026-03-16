use std::collections::HashSet;
use std::sync::Arc;

use super::hasher::CacheHasher;
use crate::{
    BuildKey, CacheContext, CacheKey, CacheOptions, CacheRegistry, Compiler, CompilerOptions,
    TaskOutcome, TaskStatus, TestFileSystem, TestProgram,
};
use destack_resolver::TypeScriptOptionsDiscovery;
use destack_source::{
    CacheKind, DiagnosticSeverity, File, FileId, FileType, FileVersion, ModuleId, ModuleVersion,
    PackageId, TemporaryPhysicalFileSystem, Uri,
};
use destack_workspace::{
    ArtifactKey, CacheMode, CachePolicy, CacheScope, CacheValidate, Destack, DiskCacheStore,
    FileUpdate, MemoryCacheStore, ModuleAst, ModuleGraphKey, Session, Workspace,
    WorkspaceIndexHeader, WorkspaceIndexStore, hash_workspace_config,
};

impl TestProgram {
    /// Compile and analyze the provided modules.
    fn compile_analyze_modules(&self, modules: &[ModuleId]) {
        // reset diagnostics for a clean assertion pass
        let _ = self.program.diagnostics.drain();

        // build a fresh compiler so tasks rerun
        let options = self.compiler.options.clone();
        let compiler = Compiler::new(self.session.clone(), self.program.clone(), options);

        // enqueue analyze tasks for the requested modules
        for module_id in modules {
            let profile = self.default_profile_id(*module_id);
            compiler.enqueue_build_key(BuildKey::Artifact(ArtifactKey::DirAnalyzed {
                module: *module_id,
                profile,
            }));
        }

        // run compilation and check diagnostics
        compiler.compile();
        self.check_no_diagnostic(DiagnosticSeverity::Note);
    }

    /// Compile and analyze the provided modules with one retained compiler.
    fn compile_analyze_modules_with(&self, compiler: &Compiler, modules: &[ModuleId]) {
        // reset diagnostics for a clean assertion pass
        let _ = self.program.diagnostics.drain();

        // enqueue analyze tasks for the requested modules
        for module_id in modules {
            let profile = self.default_profile_id(*module_id);
            compiler.enqueue_build_key(BuildKey::Artifact(ArtifactKey::DirAnalyzed {
                module: *module_id,
                profile,
            }));
        }

        // run compilation and check diagnostics
        compiler.compile();
        self.check_no_diagnostic(DiagnosticSeverity::Note);
    }

    /// Replace module source and invalidate program state.
    fn replace_module_source(&self, module_id: ModuleId, content: &str) {
        // capture the module path for filesystem updates
        let module_path = {
            let module = self.program.modules.get(module_id);
            let module = module.as_ref();
            if !module.is_code() {
                panic!("expected a code module for source replacement");
            }
            module
                .path
                .clone()
                .unwrap_or_else(|| panic!("module path missing for {module_id:?}"))
        };

        // update the memory filesystem content
        match &self.fs {
            TestFileSystem::Memory { fs } => {
                fs.add_file(&module_path, content.as_bytes())
                    .unwrap_or_else(|error| panic!("failed to write module file: {error}"));
            }
            TestFileSystem::Physical { .. } => {
                panic!("incremental tests require memory filesystem");
            }
        }

        // invalidate the module via program api
        let file_id = self.program.modules.get(module_id).file_id;
        self.program
            .invalidate_file(
                file_id,
                FileUpdate::Text {
                    content: content.to_string(),
                },
            )
            .unwrap_or_else(|error| panic!("failed to invalidate file: {error}"));
    }
}

/// Rebuild tasks when module versions change.
#[test]
fn test_task_rebuilds_after_module_version_change() {
    // set up a program and register a module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("main.ts", "export const value = 1;");
    let build_key = BuildKey::Artifact(ArtifactKey::Ast { module: module_id });
    let artifact_key = ArtifactKey::Ast { module: module_id };

    // seed the initial artifact
    let outcome = test.compiler.run_build_key(build_key.clone());
    assert!(matches!(outcome, TaskOutcome::Complete { .. }));

    let initial_dependency = test
        .program
        .artifacts
        .dependency(&artifact_key)
        .unwrap_or_else(|| panic!("missing dependency for {artifact_key:?}"));

    // invalidate the module to bump its version
    let file_id = test.program.modules.get(module_id).file_id;
    test.program
        .invalidate_file(file_id, FileUpdate::Touch)
        .unwrap_or_else(|error| panic!("failed to invalidate file: {error}"));

    // rebuild the same build key against the new input version
    let outcome = test.compiler.run_build_key(build_key.clone());

    // check that the task completed again under a new dependency
    assert!(matches!(outcome, TaskOutcome::Complete { .. }));
    let status = test
        .compiler
        .get_status(&build_key)
        .unwrap_or_else(|| panic!("missing task status"));
    assert!(matches!(status, TaskStatus::Complete));

    let updated_dependency = test
        .program
        .artifacts
        .dependency(&artifact_key)
        .unwrap_or_else(|| panic!("missing dependency for {artifact_key:?}"));

    assert_ne!(
        updated_dependency, initial_dependency,
        "expected artifact dependency to change after module invalidation"
    );
}

/// Exact build requirements mark dependent DIR artifacts stale.
#[test]
fn test_exact_requirements_mark_dependent_dir_stale() {
    let test = TestProgram::memory_sequential();
    let compiler = Compiler::new(
        test.session.clone(),
        test.program.clone(),
        test.compiler.options.clone(),
    );
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

    // seed analyzed artifacts and exact requirements
    test.compile_analyze_modules_with(&compiler, &[module_a_id, module_b_id]);

    let profile = test.default_profile_id(module_a_id);
    let build_key = BuildKey::Artifact(ArtifactKey::DirAnalyzed {
        module: module_b_id,
        profile,
    });
    let b_has_dir_before = test
        .program
        .artifacts
        .dir_analyzed(module_b_id, profile)
        .is_some();
    assert!(
        compiler.build_key_is_available(&build_key),
        "expected dependent dir to be available before edits"
    );

    // update module a without changing its export surface
    test.replace_module_source(
        module_a_id,
        r#"
export const value: number = 2;
"#,
    );
    test.compile_analyze_modules_with(&compiler, &[module_a_id]);

    let b_has_dir_after_internal = test
        .program
        .artifacts
        .dir_analyzed(module_b_id, profile)
        .is_some();

    assert_eq!(
        b_has_dir_after_internal, b_has_dir_before,
        "expected dependent dir artifact to remain published"
    );
    assert!(
        !compiler.build_key_is_available(&build_key),
        "expected dependent dir to become stale after dependency change"
    );

    // rerun the dependent build and restore availability
    let outcome = compiler.run_build_key(build_key.clone());
    assert!(
        matches!(outcome, TaskOutcome::Complete { .. }),
        "expected dependent dir rebuild to complete"
    );
    assert!(
        compiler.build_key_is_available(&build_key),
        "expected dependent dir to be available after rebuild"
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
    test.compile_analyze_modules(&[module_a_id, module_b_id, module_c_id]);

    let profile = test.default_profile_id(module_a_id);
    let graph_key = ModuleGraphKey::new(profile);
    let graph = test
        .program
        .index
        .module_graphs
        .get(&graph_key)
        .unwrap_or_else(|| panic!("missing module graph for {profile:?}"));
    let deps_before: HashSet<_> = graph.dependencies_for(module_a_id).into_iter().collect();
    let dependents_b_before: HashSet<_> = graph.dependents_for(module_b_id).into_iter().collect();
    let dependents_c_before: HashSet<_> = graph.dependents_for(module_c_id).into_iter().collect();
    drop(graph);

    // check that the module graph is updated
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
    test.replace_module_source(
        module_a_id,
        r#"
import { value } from "./c.ts";

value;
"#,
    );
    test.compile_analyze_modules(&[module_a_id, module_c_id]);

    let graph = test
        .program
        .index
        .module_graphs
        .get(&graph_key)
        .unwrap_or_else(|| panic!("missing module graph for {profile:?}"));
    let deps_after: HashSet<_> = graph.dependencies_for(module_a_id).into_iter().collect();
    let dependents_b_after: HashSet<_> = graph.dependents_for(module_b_id).into_iter().collect();
    let dependents_c_after: HashSet<_> = graph.dependents_for(module_c_id).into_iter().collect();
    drop(graph);

    // check that the module graph is updated
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

/// Exact build requirements propagate staleness through reexports.
#[test]
fn test_exact_requirements_propagate_through_reexports() {
    let test = TestProgram::memory_sequential();
    let compiler = Compiler::new(
        test.session.clone(),
        test.program.clone(),
        test.compiler.options.clone(),
    );
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

    // seed analyzed artifacts and exact requirements
    test.compile_analyze_modules_with(&compiler, &[module_b_id, module_a_id, module_c_id]);

    let profile = test.default_profile_id(module_a_id);
    let a_build_key = BuildKey::Artifact(ArtifactKey::DirAnalyzed {
        module: module_a_id,
        profile,
    });
    let c_build_key = BuildKey::Artifact(ArtifactKey::DirAnalyzed {
        module: module_c_id,
        profile,
    });
    assert!(compiler.build_key_is_available(&a_build_key));
    assert!(compiler.build_key_is_available(&c_build_key));

    // update module b without changing its export surface
    test.replace_module_source(
        module_b_id,
        r#"
export const value: number = 2;
"#,
    );
    test.compile_analyze_modules_with(&compiler, &[module_b_id]);

    let a_has_dir_after_b = test
        .program
        .artifacts
        .dir_analyzed(module_a_id, profile)
        .is_some();
    let c_has_dir_after_b = test
        .program
        .artifacts
        .dir_analyzed(module_c_id, profile)
        .is_some();

    // check that both downstream artifacts remain published but stale
    assert!(
        a_has_dir_after_b,
        "expected reexporting module dir artifact to remain published"
    );
    assert!(
        c_has_dir_after_b,
        "expected transitive dependent dir artifact to remain published"
    );
    assert!(
        !compiler.build_key_is_available(&a_build_key),
        "expected reexporting module dir to become stale"
    );
    assert!(
        !compiler.build_key_is_available(&c_build_key),
        "expected transitive dependent dir to become stale"
    );
}

/// Exact build requirements mark dependent patched DIR artifacts stale.
#[test]
fn test_exact_requirements_mark_dependent_patched_dir_stale() {
    let test = TestProgram::memory_sequential();
    let compiler = Compiler::new(
        test.session.clone(),
        test.program.clone(),
        test.compiler.options.clone(),
    );
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

    // seed analyzed artifacts first
    test.compile_analyze_modules_with(&compiler, &[module_a_id, module_b_id]);

    // seed a patched dir artifact to validate freshness behavior
    let profile = test.default_profile_id(module_b_id);
    let build_key = BuildKey::Artifact(ArtifactKey::DirPatched {
        module: module_b_id,
        profile,
    });
    let outcome = compiler.run_build_key(build_key.clone());
    assert!(matches!(outcome, TaskOutcome::Complete { .. }));

    let b_has_patched_dir_before = test
        .program
        .artifacts
        .dir_patched(module_b_id, profile)
        .is_some();

    // check that the patched dir is seeded before edits
    assert!(
        b_has_patched_dir_before,
        "expected patched dir to be seeded before edits"
    );
    assert!(compiler.build_key_is_available(&build_key));

    // update module a without changing its export surface
    test.replace_module_source(
        module_a_id,
        r#"
export const value: number = 2;
"#,
    );
    test.compile_analyze_modules_with(&compiler, &[module_a_id]);

    let b_has_patched_dir_after = test
        .program
        .artifacts
        .dir_patched(module_b_id, profile)
        .is_some();

    // check that the patched dir artifact remains published but stale
    assert!(
        b_has_patched_dir_after,
        "expected dependent patched dir artifact to remain published"
    );
    assert!(
        !compiler.build_key_is_available(&build_key),
        "expected dependent patched dir to become stale after dependency change"
    );
}

/// Cache entries roundtrip through disk storage.
#[test]
#[ignore]
fn test_cache_roundtrip_disk() {
    let cache_root = TemporaryPhysicalFileSystem::new_with_prefix("cache_roundtrip_disk");

    let options = CacheOptions {
        mode: CacheMode::Disk,
        dir: cache_root.root().to_path_buf(),
        policy: CachePolicy::Lru,
        validate: CacheValidate::Strict,
        scope: CacheScope::Workspace,
        max_size_mb: None,
    };
    let context = CacheContext {
        compiler_version: "test".to_string(),
        file_version: FileVersion::INITIAL,
        profile_id: None,
        profile_version: None,
        source_hash: 1,
        config_hash: 2,
        target_hash: 3,
        dependency_hash: 0,
    };
    let module_id = ModuleId::new(PackageId::new(1), 1);
    let payload = ModuleAst::new(module_id, ModuleVersion::INITIAL);

    let cache_store = DiskCacheStore::new();
    let registry = CacheRegistry::new();
    registry
        .write_ast_cache(&cache_store, &options, &context, module_id, payload.clone())
        .unwrap_or_else(|error| panic!("failed to write cache entry: {error}"));

    // check that the ast cache entry is read from disk
    let entry = registry
        .read_ast_cache(&cache_store, &options, &context, module_id)
        .unwrap_or_else(|error| panic!("failed to read cache entry: {error}"));
    assert!(entry.is_some(), "expected ast cache entry");

    let fresh_registry = CacheRegistry::new();
    let entry = fresh_registry
        .read_ast_cache(&cache_store, &options, &context, module_id)
        .unwrap_or_else(|error| panic!("failed to read cache entry: {error}"));

    // check that the ast cache entry is read from disk
    assert!(entry.is_some(), "expected ast cache entry from disk");
}

/// Disk cache hits should update access markers for LRU eviction.
#[test]
#[ignore]
fn test_cache_disk_access_markers() {
    let cache_root = TemporaryPhysicalFileSystem::new_with_prefix("cache_access");

    let options = CacheOptions {
        mode: CacheMode::Disk,
        dir: cache_root.root().to_path_buf(),
        policy: CachePolicy::Lru,
        validate: CacheValidate::Strict,
        scope: CacheScope::Workspace,
        max_size_mb: None,
    };
    let context = CacheContext {
        compiler_version: "test".to_string(),
        file_version: FileVersion::INITIAL,
        profile_id: None,
        profile_version: None,
        source_hash: 10,
        config_hash: 20,
        target_hash: 30,
        dependency_hash: 0,
    };
    let module_id = ModuleId::new(PackageId::new(2), 3);
    let payload = ModuleAst::new(module_id, ModuleVersion::INITIAL);

    let cache_store = DiskCacheStore::new();
    let registry = CacheRegistry::new();
    registry
        .write_ast_cache(&cache_store, &options, &context, module_id, payload)
        .unwrap_or_else(|error| panic!("failed to write cache entry: {error}"));

    let read_registry = CacheRegistry::new();
    let entry = read_registry
        .read_ast_cache(&cache_store, &options, &context, module_id)
        .unwrap_or_else(|error| panic!("failed to read cache entry: {error}"));
    assert!(entry.is_some(), "expected ast cache entry");

    let key = CacheKey::new(CacheKind::Ast, module_id, &context);
    let entry_path = read_registry.cache_entry_path(&options, &key);
    let access_path = read_registry.cache_access_path(&entry_path);

    // check that the access marker is written
    assert!(access_path.exists(), "expected access marker to be written");
}

/// Workspace index snapshots roundtrip through disk.
#[test]
fn test_workspace_index_roundtrip_disk() {
    // set up a physical workspace
    let root = TemporaryPhysicalFileSystem::new_with_prefix("workspace_index_disk");
    let root_path = root.root().to_path_buf();
    root.write_bytes("package.json", br#"{ "name": "workspace-index-test" }"#)
        .unwrap();
    let config_path = root.path_for("destack.json");
    let config_content = r#"{ "cache": { "mode": "disk" } }"#;
    root.write_text("destack.json", config_content).unwrap();
    let module_path = root.path_for("main.ts");
    root.write_text("main.ts", "export const value: number = 1;")
        .unwrap();

    // parse workspace config
    let config_file = File::from_text_as_jsonc(
        FileId::new(1),
        "destack.json".to_string(),
        Uri::from_path(&config_path),
        Some(config_path.clone()),
        FileType::Json,
        config_content.to_string(),
    )
    .unwrap_or_else(|error| panic!("failed to parse destack.json: {error}"));
    let config = Destack::parse(&Arc::new(config_file))
        .unwrap_or_else(|error| panic!("failed to build destack.json: {error}"));

    // register a module and flush the workspace index
    let workspace = Workspace::single_package(root_path.clone()).with_config(config.clone());
    let session = Arc::new(Session::workspace(root_path.clone(), Arc::new(workspace)));
    let program = session.add_root(root_path.clone());
    let compiler = Compiler::new(
        session.clone(),
        program.clone(),
        CompilerOptions {
            workers: 1,
            ..CompilerOptions::default()
        },
    );
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| panic!("failed to register module: {error:?}"));
    compiler
        .flush_workspace_index()
        .unwrap_or_else(|error| panic!("failed to flush workspace index: {error}"));

    // load workspace index from disk
    let cache_root = session.workspace_cache_dir();
    let index_store = WorkspaceIndexStore::new(session.cache_store.as_ref(), &cache_root);
    let workspace = session.workspace_snapshot();
    let config_hash = hash_workspace_config(&workspace, session.fs.as_ref())
        .unwrap_or_else(|error| panic!("failed to hash config: {error}"));
    let mut compiler_hasher = CacheHasher::new();
    compiler_hasher.hash_compiler_options(&compiler.options);
    let compiler_options_hash = compiler_hasher.finish();

    let mut resolve_hasher = CacheHasher::new();
    resolve_hasher.hash_resolve_options(&compiler.options.import_resolve);
    let resolve_options_hash = resolve_hasher.finish();

    let header = WorkspaceIndexHeader::new(
        env!("CARGO_PKG_VERSION").to_string(),
        root_path.clone(),
        config_hash,
        compiler_options_hash,
        resolve_options_hash,
        CacheValidate::Strict,
    );
    let snapshot = index_store
        .load(&header)
        .unwrap_or_else(|error| panic!("failed to read workspace index: {error}"))
        .unwrap_or_else(|| panic!("expected workspace index snapshot"));

    // check that the workspace index snapshot is loaded
    assert!(
        snapshot.files.contains_key(&module_path),
        "expected snapshot to include module file"
    );
    assert!(
        snapshot.modules.contains_key(&module_id),
        "expected snapshot to include module id"
    );

    // reload workspace index into a new program
    let workspace = Workspace::single_package(root_path.clone()).with_config(config);
    let session = Arc::new(Session::workspace(root_path.clone(), Arc::new(workspace)));
    let program = session.add_root(root_path.clone());
    let _compiler = Compiler::new(session, program.clone(), CompilerOptions::default());

    // check that the workspace index is loaded
    assert!(
        program.workspace_file_entry(&module_path).is_some(),
        "expected workspace index to load file entry"
    );
    assert!(
        program.workspace_module_entry(module_id).is_some(),
        "expected workspace index to load module entry"
    );
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
        profile_id: None,
        profile_version: None,
        source_hash: 1,
        config_hash: 2,
        target_hash: 3,
        dependency_hash: 0,
    };
    let module_id = ModuleId::new(PackageId::new(2), 2);
    let payload = ModuleAst::new(module_id, ModuleVersion::INITIAL);

    let cache_store = MemoryCacheStore::new();
    let registry = CacheRegistry::new();
    registry
        .write_ast_cache(&cache_store, &options, &context, module_id, payload)
        .unwrap_or_else(|error| panic!("failed to write cache entry: {error}"));

    // check that the ast cache entry is read from disk
    let entry = registry
        .read_ast_cache(&cache_store, &options, &context, module_id)
        .unwrap_or_else(|error| panic!("failed to read cache entry: {error}"));
    assert!(entry.is_some(), "expected ast cache entry");

    // check that the ast cache entry is not read from disk
    let fresh_registry = CacheRegistry::new();
    let entry = fresh_registry
        .read_ast_cache(&cache_store, &options, &context, module_id)
        .unwrap_or_else(|error| panic!("failed to read cache entry: {error}"));
    assert!(
        entry.is_none(),
        "expected no ast cache entry in fresh registry"
    );
}

/// Cache entries are invalidated when the context changes.
#[test]
fn test_cache_miss_on_context_change() {
    let cache_root = TemporaryPhysicalFileSystem::new_with_prefix("cache_miss_context");

    let options = CacheOptions {
        mode: CacheMode::Disk,
        dir: cache_root.root().to_path_buf(),
        policy: CachePolicy::Lru,
        validate: CacheValidate::Strict,
        scope: CacheScope::Workspace,
        max_size_mb: None,
    };
    let context = CacheContext {
        compiler_version: "test".to_string(),
        file_version: FileVersion::INITIAL,
        profile_id: None,
        profile_version: None,
        source_hash: 1,
        config_hash: 2,
        target_hash: 3,
        dependency_hash: 0,
    };
    let module_id = ModuleId::new(PackageId::new(3), 3);
    let payload = ModuleAst::new(module_id, ModuleVersion::INITIAL);

    let cache_store = DiskCacheStore::new();
    let registry = CacheRegistry::new();
    registry
        .write_ast_cache(&cache_store, &options, &context, module_id, payload)
        .unwrap_or_else(|error| panic!("failed to write cache entry: {error}"));

    let mismatched_context = CacheContext {
        compiler_version: "test".to_string(),
        file_version: FileVersion::INITIAL,
        profile_id: None,
        profile_version: None,
        source_hash: 10,
        config_hash: 2,
        target_hash: 3,
        dependency_hash: 0,
    };

    // check that the ast cache entry is read from disk
    let entry = registry
        .read_ast_cache(&cache_store, &options, &mismatched_context, module_id)
        .unwrap_or_else(|error| panic!("failed to read cache entry: {error}"));
    assert!(
        entry.is_none(),
        "expected cache miss for mismatched context"
    );
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
        profile_id: None,
        profile_version: None,
        source_hash: 1,
        config_hash: 2,
        target_hash: 3,
        dependency_hash: 10,
    };
    let module_id = ModuleId::new(PackageId::new(4), 4);
    let payload = ModuleAst::new(module_id, ModuleVersion::INITIAL);

    let cache_store = MemoryCacheStore::new();
    let registry = CacheRegistry::new();
    registry
        .write_ast_cache(&cache_store, &options, &context, module_id, payload)
        .unwrap_or_else(|error| panic!("failed to write cache entry: {error}"));

    let mismatched_context = CacheContext {
        compiler_version: "test".to_string(),
        file_version: FileVersion::INITIAL,
        profile_id: None,
        profile_version: None,
        source_hash: 1,
        config_hash: 2,
        target_hash: 3,
        dependency_hash: 999,
    };

    // check that the ast cache entry is not read from disk
    let entry = registry
        .read_ast_cache(&cache_store, &options, &mismatched_context, module_id)
        .unwrap_or_else(|error| panic!("failed to read cache entry: {error}"));
    assert!(
        entry.is_none(),
        "expected dependency hash mismatch to invalidate cache entry"
    );
}

/// Cache entries are invalidated when file versions change.
#[test]
fn test_cache_miss_on_file_version_bump() {
    // set up a cached module context
    let test = TestProgram::memory_sequential().with_cache_mode(CacheMode::Memory, None);
    let module_id = test.add_module("main.ts", "export const value = 1;");
    test.compiler
        .import_module_parse(module_id, test.module_version(module_id))
        .unwrap_or_else(|error| panic!("failed to parse module: {error:?}"));
    let context_before = test
        .compiler
        .cache_context_for_module(module_id, None, None, CacheKind::Ast)
        .unwrap_or_else(|error| panic!("failed to build cache context: {error:?}"));

    let options = CacheOptions {
        mode: CacheMode::Memory,
        dir: std::env::temp_dir(),
        policy: CachePolicy::Lru,
        validate: CacheValidate::Strict,
        scope: CacheScope::Workspace,
        max_size_mb: None,
    };
    let cache_store = test.session.cache_store.as_ref();
    let registry = CacheRegistry::new();
    let payload = ModuleAst::new(module_id, ModuleVersion::INITIAL);
    registry
        .write_ast_cache(cache_store, &options, &context_before, module_id, payload)
        .unwrap_or_else(|error| panic!("failed to write ast cache entry: {error}"));

    // update file content and rebuild cache context
    test.replace_module_source(module_id, "export const value = 2;");
    let context_after = test
        .compiler
        .cache_context_for_module(module_id, None, None, CacheKind::Ast)
        .unwrap_or_else(|error| panic!("failed to build cache context: {error:?}"));

    // check that the file version is bumped
    assert!(
        context_after.file_version > context_before.file_version,
        "expected file version to bump after source update"
    );
    assert_ne!(
        context_after.source_hash, context_before.source_hash,
        "expected source hash to change after source update"
    );

    // check that the ast cache entry is not read from disk
    let entry = registry
        .read_ast_cache(cache_store, &options, &context_after, module_id)
        .unwrap_or_else(|error| panic!("failed to read ast cache entry: {error}"));
    assert!(
        entry.is_none(),
        "expected cache miss after file version change"
    );
}

/// Cache entries are invalidated when tsconfig changes.
#[test]
fn test_cache_miss_on_tsconfig_change() {
    // set up a cached module context with a tsconfig
    let test = TestProgram::memory_sequential()
        .with_options_mut(|options| {
            options.import_resolve.tsconfig = Some(TypeScriptOptionsDiscovery::Automatic);
        })
        .with_cache_mode(CacheMode::Memory, None);
    let root = test.program.cwd.clone();
    let tsconfig_path = root.join("tsconfig.json");
    let tsconfig_path_str = tsconfig_path.to_string_lossy().to_string();
    test.add_file(
        &tsconfig_path_str,
        r#"{ "compilerOptions": { "strict": true } }"#,
    );
    let module_path = root.join("main.ts");
    let module_path_str = module_path.to_string_lossy().to_string();
    let module_id = test.add_module(&module_path_str, "export const value = 1;");
    test.compiler
        .import_module_parse(module_id, test.module_version(module_id))
        .unwrap_or_else(|error| panic!("failed to parse module: {error:?}"));
    let context_before = test
        .compiler
        .cache_context_for_module(module_id, None, None, CacheKind::Ast)
        .unwrap_or_else(|error| panic!("failed to build cache context: {error:?}"));

    let options = CacheOptions {
        mode: CacheMode::Memory,
        dir: std::env::temp_dir(),
        policy: CachePolicy::Lru,
        validate: CacheValidate::Strict,
        scope: CacheScope::Workspace,
        max_size_mb: None,
    };
    let cache_store = test.session.cache_store.as_ref();
    let registry = CacheRegistry::new();
    let payload = ModuleAst::new(module_id, ModuleVersion::INITIAL);
    registry
        .write_ast_cache(cache_store, &options, &context_before, module_id, payload)
        .unwrap_or_else(|error| panic!("failed to write ast cache entry: {error}"));

    // update tsconfig content
    let tsconfig_id = test
        .program
        .modules
        .get(module_id)
        .tsconfig_id()
        .unwrap_or_else(|| panic!("expected tsconfig for {module_id:?}"));
    let tsconfig = test.program.tsconfigs.get(tsconfig_id);
    let tsconfig = tsconfig.read();
    let tsconfig_file_id = tsconfig.file_id;
    drop(tsconfig);
    test.program
        .invalidate_file(
            tsconfig_file_id,
            FileUpdate::Text {
                content: r#"{ "compilerOptions": { "strict": false } }"#.to_string(),
            },
        )
        .unwrap_or_else(|error| panic!("failed to invalidate tsconfig: {error}"));

    let context_after = test
        .compiler
        .cache_context_for_module(module_id, None, None, CacheKind::Ast)
        .unwrap_or_else(|error| panic!("failed to build cache context: {error:?}"));

    // check that the config hash is changed
    assert_ne!(
        context_after.config_hash, context_before.config_hash,
        "expected config hash to change after tsconfig update"
    );

    // check that the ast cache entry is not read from disk
    let entry = registry
        .read_ast_cache(cache_store, &options, &context_after, module_id)
        .unwrap_or_else(|error| panic!("failed to read ast cache entry: {error}"));
    assert!(entry.is_none(), "expected cache miss after tsconfig change");
}

/// Cache entries are invalidated when dir dependencies change.
#[test]
fn test_cache_miss_on_dir_dependency_change() {
    // set up modules and cache a profile dir
    let test = TestProgram::memory_sequential().with_cache_mode(CacheMode::Memory, None);
    let module_a_id = test.add_module("a.ts", "export const value = 1;");
    let module_b_id = test.add_module(
        "b.ts",
        r#"
import { value } from "./a.ts";

value;
"#,
    );
    let profile_id = test.default_profile_id(module_b_id);
    // seed module graph and resolved dir artifacts
    test.compile_analyze_modules(&[module_a_id, module_b_id]);
    let context_before = test
        .compiler
        .cache_context_for_module(module_b_id, Some(profile_id), None, CacheKind::DirResolved)
        .unwrap_or_else(|error| panic!("failed to build cache context: {error:?}"));

    let options = CacheOptions {
        mode: CacheMode::Memory,
        dir: std::env::temp_dir(),
        policy: CachePolicy::Lru,
        validate: CacheValidate::Strict,
        scope: CacheScope::Workspace,
        max_size_mb: None,
    };
    let cache_store = test.session.cache_store.as_ref();
    let registry = CacheRegistry::new();
    let dir_payload = test
        .program
        .artifacts
        .dir_resolved(module_b_id, profile_id)
        .unwrap_or_else(|| panic!("missing resolved dir for {module_b_id:?}"))
        .as_ref()
        .clone();
    registry
        .write_dir_resolved_cache(
            cache_store,
            &options,
            &context_before,
            module_b_id,
            dir_payload,
        )
        .unwrap_or_else(|error| panic!("failed to write dir cache entry: {error}"));

    // update the dependency without changing its export surface
    test.replace_module_source(module_a_id, "export const value = 2;");
    test.compile_analyze_modules(&[module_a_id]);

    let context_after = test
        .compiler
        .cache_context_for_module(module_b_id, Some(profile_id), None, CacheKind::DirResolved)
        .unwrap_or_else(|error| panic!("failed to build cache context: {error:?}"));

    // check that the dependency hash is changed
    assert_ne!(
        context_after.dependency_hash, context_before.dependency_hash,
        "expected dependency hash to change after dependency artifact update"
    );

    // check that the dir cache entry is not read from disk
    let entry = registry
        .read_dir_resolved_cache(cache_store, &options, &context_after, module_b_id)
        .unwrap_or_else(|error| panic!("failed to read dir cache entry: {error}"));
    assert!(
        entry.is_none(),
        "expected cache miss after dependency artifact change"
    );
}
