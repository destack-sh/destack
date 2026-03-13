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
    ArtifactKey, CacheMode, CachePolicy, CacheScope, CacheValidate, DiskCacheStore, DsConfig,
    FileUpdate, MemoryCacheStore, ModuleAst, ModuleGraphKey, ModuleMir, ModuleSignatureKey,
    Session, TargetId, Workspace, WorkspaceIndexHeader, WorkspaceIndexStore, hash_workspace_config,
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

    /// Replace module source and invalidate program state.
    fn replace_module_source(&self, module_id: ModuleId, content: &str) {
        // capture the module path for filesystem updates
        let module_path = {
            let module = self.program.modules.get(module_id);
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
        let file_id = self.program.modules.get(module_id).read().file_id;
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
    let file_id = test.program.modules.get(module_id).read().file_id;
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
    test.compile_analyze_modules(&[module_a_id, module_b_id]);

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
        .artifacts
        .dir_analyzed(module_b_id, profile)
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
    test.replace_module_source(
        module_a_id,
        r#"
export const value: number = 2;
"#,
    );
    test.compile_analyze_modules(&[module_a_id]);

    let stable_signature = test
        .program
        .index
        .module_signatures
        .get(&signature_key)
        .unwrap_or_else(|| panic!("missing signature for {module_a_id:?}"));
    let stable_hash = stable_signature.value().hash;
    let b_has_dir_after_internal = test
        .program
        .artifacts
        .dir_analyzed(module_b_id, profile)
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

    // check that the signature and dependent state are unchanged
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
    test.replace_module_source(
        module_a_id,
        r#"
export const value: string = "value";
"#,
    );
    test.compile_analyze_modules(&[module_a_id]);

    let changed_signature = test
        .program
        .index
        .module_signatures
        .get(&signature_key)
        .unwrap_or_else(|| panic!("missing signature for {module_a_id:?}"));
    let changed_hash = changed_signature.value().hash;
    let b_has_dir_after_export = test
        .program
        .artifacts
        .dir_analyzed(module_b_id, profile)
        .is_some();
    let b_signature_after_export = test
        .program
        .index
        .module_signatures
        .get(&ModuleSignatureKey::new(module_b_id, profile))
        .unwrap_or_else(|| panic!("missing signature for {module_b_id:?}"));
    let b_signature_hash_after_export = b_signature_after_export.value().hash;

    // check that the signature and dependent state are changed
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
    test.compile_analyze_modules(&[module_b_id, module_a_id, module_c_id]);

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
        .artifacts
        .dir_analyzed(module_c_id, profile)
        .is_some();
    drop(a_signature_before);

    // update module b with export change
    test.replace_module_source(
        module_b_id,
        r#"
export const value: string = "value";
"#,
    );
    test.compile_analyze_modules(&[module_b_id]);

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
    let a_signature_after_b = test
        .program
        .index
        .module_signatures
        .get(&ModuleSignatureKey::new(module_a_id, profile))
        .unwrap_or_else(|| panic!("missing signature for {module_a_id:?}"));
    let a_hash_after_b = a_signature_after_b.value().hash;
    drop(a_signature_after_b);

    // check that the module signature is updated
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
    test.compile_analyze_modules(&[module_a_id]);

    let a_signature_after_a = test
        .program
        .index
        .module_signatures
        .get(&ModuleSignatureKey::new(module_a_id, profile))
        .unwrap_or_else(|| panic!("missing signature for {module_a_id:?}"));
    let a_hash_after_a = a_signature_after_a.value().hash;
    let c_has_dir_after_a = test
        .program
        .artifacts
        .dir_analyzed(module_c_id, profile)
        .is_some();

    // check that the module signature is updated
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
    test.compile_analyze_modules(&[module_a_id, module_b_id]);

    // seed a mir artifact to validate invalidation behavior
    let profile = test.default_profile_id(module_b_id);
    let package_id = test.program.modules.get(module_b_id).read().package_id;
    let target_id = TargetId::new(package_id, "native");
    let module_version = test.module_version(module_b_id);
    let mir = ModuleMir::new(module_b_id, module_version, target_id.clone());
    test.program
        .artifacts
        .set_mir(module_b_id, profile, target_id.clone(), mir.to_data());

    let b_has_mir_before = test
        .program
        .artifacts
        .optimized_mir(module_b_id, profile, &target_id)
        .or_else(|| test.program.artifacts.mir(module_b_id, profile, &target_id))
        .is_some();

    // check that the mir is seeded before edits
    assert!(b_has_mir_before, "expected mir to be seeded before edits");

    // update module a with an export change
    test.replace_module_source(
        module_a_id,
        r#"
export const value: string = "value";
"#,
    );
    test.compile_analyze_modules(&[module_a_id]);

    let b_has_mir_after = test
        .program
        .artifacts
        .optimized_mir(module_b_id, profile, &target_id)
        .or_else(|| test.program.artifacts.mir(module_b_id, profile, &target_id))
        .is_some();

    // check that the mir is cleared after edits
    assert!(
        !b_has_mir_after,
        "expected dependent mir cleared after signature change"
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
    let payload = ModuleAst::new(module_id, ModuleVersion::INITIAL).to_data();

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
    let payload = ModuleAst::new(module_id, ModuleVersion::INITIAL).to_data();

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
    let dsconfig_path = root.path_for("dsconfig.json");
    let dsconfig_content = r#"{ "cache": { "mode": "disk" } }"#;
    root.write_text("dsconfig.json", dsconfig_content).unwrap();
    let module_path = root.path_for("main.ts");
    root.write_text("main.ts", "export const value: number = 1;")
        .unwrap();

    // parse workspace config
    let dsconfig_file = File::from_text_as_jsonc(
        FileId::new(1),
        "dsconfig.json".to_string(),
        Uri::from_path(&dsconfig_path),
        Some(dsconfig_path.clone()),
        FileType::Json,
        dsconfig_content.to_string(),
    )
    .unwrap_or_else(|error| panic!("failed to parse dsconfig: {error}"));
    let dsconfig = DsConfig::parse(&Arc::new(dsconfig_file))
        .unwrap_or_else(|error| panic!("failed to build dsconfig: {error}"));

    // register a module and flush the workspace index
    let workspace = Workspace::single_package(root_path.clone()).with_config(dsconfig.clone());
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
    let workspace = Workspace::single_package(root_path.clone()).with_config(dsconfig);
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
    let payload = ModuleAst::new(module_id, ModuleVersion::INITIAL).to_data();

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
    let payload = ModuleAst::new(module_id, ModuleVersion::INITIAL).to_data();

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
    let payload = ModuleAst::new(module_id, ModuleVersion::INITIAL).to_data();

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
    let payload = ModuleAst::new(module_id, ModuleVersion::INITIAL).to_data();
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
    let payload = ModuleAst::new(module_id, ModuleVersion::INITIAL).to_data();
    registry
        .write_ast_cache(cache_store, &options, &context_before, module_id, payload)
        .unwrap_or_else(|error| panic!("failed to write ast cache entry: {error}"));

    // update tsconfig content
    let tsconfig_id = test
        .program
        .modules
        .get(module_id)
        .read()
        .tsconfig_id
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
    // seed module graph, signatures, and resolved dir artifacts
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

    // update the dependency export surface and rebuild its signature
    test.replace_module_source(module_a_id, "export const value = 'value';");
    test.compile_analyze_modules(&[module_a_id]);

    let context_after = test
        .compiler
        .cache_context_for_module(module_b_id, Some(profile_id), None, CacheKind::DirResolved)
        .unwrap_or_else(|error| panic!("failed to build cache context: {error:?}"));

    // check that the dependency hash is changed
    assert_ne!(
        context_after.dependency_hash, context_before.dependency_hash,
        "expected dependency hash to change after export update"
    );

    // check that the dir cache entry is not read from disk
    let entry = registry
        .read_dir_resolved_cache(cache_store, &options, &context_after, module_b_id)
        .unwrap_or_else(|error| panic!("failed to read dir cache entry: {error}"));
    assert!(
        entry.is_none(),
        "expected cache miss after dependency signature change"
    );
}
