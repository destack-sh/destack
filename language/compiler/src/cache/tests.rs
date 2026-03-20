use std::sync::Arc;

use destack_core::StringPool;
use destack_dir::{Dumper, DumperOptions, NodeVisitor};
use destack_source::{File, FileId, FileType, FileVersion, TemporaryPhysicalFileSystem, Uri};
use destack_workspace::{
    ArtifactImage, ArtifactImageHeader, ArtifactImageKey, ArtifactStore, Ast, AstImage, CacheStore,
    Destack, DirPrepared, DirResolved, EnvSnapshot, LanguageEnvironment, MemoryCacheStore,
    OutputFormat, Platform, ProfileFlags, ProfileKey, Program, Runtime, Session, Workspace,
};

use crate::{Compiler, CompilerOptions};

/// Build one stable profile key for cache image tests.
fn test_profile_key() -> ProfileKey {
    ProfileKey::new(
        OutputFormat::Js,
        Runtime::Node,
        Platform::Web,
        None,
        None,
        None,
        Vec::new(),
        false,
        false,
        false,
        EnvSnapshot::Whitelist {
            keys: Vec::new(),
            hash: 0,
        },
        ProfileFlags::default(),
    )
}

/// Parse one workspace config file.
fn parse_workspace_config(config_path: &std::path::Path, config_content: &str) -> Destack {
    let file = File::from_text_as_jsonc(
        FileId::new(1),
        "destack.json".to_string(),
        Uri::from_path(config_path),
        Some(config_path.to_path_buf()),
        FileType::Json,
        config_content.to_string(),
    )
    .unwrap_or_else(|error| panic!("failed to parse destack.json: {error}"));
    let file = Arc::new(file.with_version(FileVersion::INITIAL));

    Destack::parse(&file).unwrap_or_else(|error| panic!("failed to build destack.json: {error}"))
}

/// Build one compiler over a disk-cache-enabled temporary workspace.
fn build_disk_cache_compiler(
    root: &TemporaryPhysicalFileSystem,
) -> (Arc<Session>, Arc<Program>, Compiler, std::path::PathBuf) {
    let root_path = root.root().to_path_buf();
    let config_path = root.path_for("destack.json");
    let config_content = r#"{ "cache": { "mode": "disk" } }"#;
    let package_manifest_path = root.path_for("package.json");
    let source_path = root.path_for("main.ts");

    // workspace files
    if !package_manifest_path.exists() {
        root.write_bytes("package.json", br#"{ "name": "artifact-image-test" }"#)
            .unwrap_or_else(|error| panic!("failed to write package.json: {error}"));
    }
    if !config_path.exists() {
        root.write_text("destack.json", config_content)
            .unwrap_or_else(|error| panic!("failed to write destack.json: {error}"));
    }
    if !source_path.exists() {
        root.write_text("main.ts", "export const value: number = 1;")
            .unwrap_or_else(|error| panic!("failed to write main.ts: {error}"));
    }

    // workspace config
    let config = parse_workspace_config(&config_path, config_content);
    let workspace = Workspace::single_package(root_path.clone()).with_config(config);
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

    (session, program, compiler, root.path_for("main.ts"))
}

/// Build one compiler over a temporary workspace without persistent artifact caching.
fn build_memory_cache_compiler(
    root: &TemporaryPhysicalFileSystem,
) -> (Arc<Session>, Arc<Program>, Compiler, std::path::PathBuf) {
    let root_path = root.root().to_path_buf();
    let package_manifest_path = root.path_for("package.json");
    let source_path = root.path_for("main.ts");

    // workspace files
    if !package_manifest_path.exists() {
        root.write_bytes("package.json", br#"{ "name": "artifact-image-test" }"#)
            .unwrap_or_else(|error| panic!("failed to write package.json: {error}"));
    }
    if !source_path.exists() {
        root.write_text("main.ts", "export const value: number = 1;")
            .unwrap_or_else(|error| panic!("failed to write main.ts: {error}"));
    }

    // workspace config
    let workspace = Workspace::single_package(root_path.clone());
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

    (session, program, compiler, root.path_for("main.ts"))
}

/// Normalize one AST for stable cross-session comparison.
fn normalize_ast(mut ast: Ast) -> Ast {
    ast.tree.source_map.rebind_file(FileId::new(0));

    for token in &mut ast.tokens {
        token.span = token.span.with_file(FileId::new(0));
    }

    for token in &mut ast.side_tokens {
        token.span = token.span.with_file(FileId::new(0));
    }

    ast
}

/// Dump one prepared DIR node surface deterministically.
fn dump_dir_prepared_nodes(strings: &StringPool, dir: &DirPrepared) -> String {
    let strings = strings.clone().into_immutable();
    let mut dumper = Dumper::new(&strings, &dir.tree, DumperOptions::default());

    for expression_id in dir.roots.iter().copied() {
        let expression = dir.tree.get(expression_id);
        dumper.visit_expression(&dir.tree, expression_id, expression);
    }

    dumper.finish()
}

/// Dump one prepared DIR symbol surface deterministically.
fn dump_dir_prepared_symbols(strings: &StringPool, dir: &DirPrepared) -> String {
    let strings = strings.clone().into_immutable();
    let mut dumper = Dumper::new(&strings, &dir.tree, DumperOptions::default());
    let scope = dir.symbols.get_scope_by_id(dir.namespace_scope);
    dumper.visit_scope(&dir.tree, &dir.symbols, dir.namespace_scope, scope);
    dumper.finish()
}

/// Dump one resolved DIR node surface deterministically.
fn dump_dir_resolved_nodes(strings: &StringPool, dir: &DirResolved) -> String {
    let strings = strings.clone().into_immutable();
    let mut dumper = Dumper::new(&strings, &dir.tree, DumperOptions::default());

    for expression_id in dir.roots.iter().copied() {
        let expression = dir.tree.get(expression_id);
        dumper.visit_expression(&dir.tree, expression_id, expression);
    }

    dumper.finish()
}

/// Dump one resolved DIR symbol surface deterministically.
fn dump_dir_resolved_symbols(strings: &StringPool, dir: &DirResolved) -> String {
    let strings = strings.clone().into_immutable();
    let mut dumper = Dumper::new(&strings, &dir.tree, DumperOptions::default());
    let scope = dir.symbols.get_scope_by_id(dir.namespace_scope);
    dumper.visit_scope(&dir.tree, &dir.symbols, dir.namespace_scope, scope);
    dumper.finish()
}

/// Persist and load one language environment image through the artifact store.
#[test]
fn test_artifact_store_roundtrips_language_environment_image() {
    let cache_root = std::path::PathBuf::from("/artifact-store-test");
    let cache_store: Arc<dyn CacheStore> = Arc::new(MemoryCacheStore::new());
    let artifact_store = ArtifactStore::new(cache_store.as_ref(), &cache_root);
    let profile = test_profile_key();
    let header = ArtifactImageHeader::new(
        ArtifactImageKey::LanguageEnvironment { profile },
        "test".to_string(),
        None,
        0,
        None,
        0,
    );
    let image = ArtifactImage::new(header, LanguageEnvironment::default())
        .unwrap_or_else(|error| panic!("failed to build artifact image: {error}"));

    // write the image first
    artifact_store
        .save(&image)
        .unwrap_or_else(|error| panic!("failed to save artifact image: {error}"));

    // load the same image back
    let loaded = artifact_store
        .load::<LanguageEnvironment>(&image.header.artifact_image_key)
        .unwrap_or_else(|error| panic!("failed to load artifact image: {error}"))
        .unwrap_or_else(|| panic!("expected stored artifact image"));

    // compare the full image contract
    assert_eq!(loaded.header, image.header);
    assert_eq!(loaded.payload.items, image.payload.items);
    assert_eq!(loaded.payload.symbols, image.payload.symbols);
}

/// Reuse one persisted language environment across a fresh compiler session.
#[test]
fn test_compiler_reuses_language_environment_image_across_sessions() {
    let root = TemporaryPhysicalFileSystem::new_with_prefix("language_environment_image");
    let (session, program, compiler, module_path) = build_disk_cache_compiler(&root);

    // build and persist the environment in the first session
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let profile_id = program.default_profile_id_for_module(module_id);
    compiler
        .drive(|compiler| compiler.process_language_environment(profile_id))
        .unwrap_or_else(|error| panic!("failed to persist language environment: {error:?}"));
    let expected = compiler
        .program
        .artifacts
        .language_environment(profile_id)
        .unwrap_or_else(|| panic!("expected published language environment"))
        .as_ref()
        .clone();

    drop(compiler);
    drop(program);
    drop(session);

    let (_session, program, compiler, module_path) = build_disk_cache_compiler(&root);

    // build the same stable profile in a fresh session
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| {
            panic!("failed to resolve main module in fresh session: {error:?}")
        });
    let profile_id = program.default_profile_id_for_module(module_id);

    // verify the persisted image is available before any rebuild
    let loaded = compiler
        .load_language_environment_image(profile_id)
        .unwrap_or_else(|error| panic!("failed to load persisted language environment: {error}"))
        .unwrap_or_else(|| panic!("expected persisted language environment image"));

    // compare the full semantic surface
    assert_eq!(loaded.items, expected.items);
    assert_eq!(loaded.symbols, expected.symbols);

    // validate the public compiler path too
    let resolved = compiler
        .resolve_language_environment(profile_id)
        .unwrap_or_else(|error| panic!("failed to load language environment: {error:?}"));
    assert_eq!(resolved.items, expected.items);
    assert_eq!(resolved.symbols, expected.symbols);
}

/// Persist and load one AST image through the artifact store.
#[test]
fn test_artifact_store_roundtrips_ast_image() {
    let cache_root = std::path::PathBuf::from("/artifact-store-test");
    let cache_store: Arc<dyn CacheStore> = Arc::new(MemoryCacheStore::new());
    let artifact_store = ArtifactStore::new(cache_store.as_ref(), &cache_root);
    let module = destack_source::ModuleId::EPHEMERAL;
    let header = ArtifactImageHeader::new(
        ArtifactImageKey::Ast { module },
        "test".to_string(),
        None,
        0,
        None,
        0,
    );
    let payload = AstImage::from_ast(
        &Ast::new(module, destack_source::ModuleVersion::INITIAL),
        destack_source::FileKey::EPHEMERAL,
        FileVersion::INITIAL,
        0,
    );
    let image = ArtifactImage::new(header, payload)
        .unwrap_or_else(|error| panic!("failed to build ast image: {error}"));

    // write the image first
    artifact_store
        .save(&image)
        .unwrap_or_else(|error| panic!("failed to save ast image: {error}"));

    // load the same image back
    let loaded = artifact_store
        .load::<AstImage>(&image.header.artifact_image_key)
        .unwrap_or_else(|error| panic!("failed to load ast image: {error}"))
        .unwrap_or_else(|| panic!("expected stored ast image"));

    // compare the full image contract
    assert_eq!(loaded.header, image.header);
    assert_eq!(loaded.payload.file_key, image.payload.file_key);
    assert_eq!(loaded.payload.file_version, image.payload.file_version);
    assert_eq!(loaded.payload.source_hash, image.payload.source_hash);
    assert_eq!(loaded.payload.module_version, image.payload.module_version);
}

/// Reuse one persisted AST image across a fresh compiler session.
#[test]
fn test_compiler_reuses_ast_image_across_sessions() {
    let root = TemporaryPhysicalFileSystem::new_with_prefix("ast_image");
    let (session, program, compiler, module_path) = build_disk_cache_compiler(&root);

    // build and persist the ast in the first session
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    compiler
        .drive(|compiler| compiler.process_ast(module_id))
        .unwrap_or_else(|error| panic!("failed to build ast: {error:?}"));
    let expected = compiler
        .program
        .artifacts
        .ast(module_id)
        .unwrap_or_else(|| panic!("expected published ast"))
        .as_ref()
        .clone();

    drop(compiler);
    drop(program);
    drop(session);

    let (_session, program, compiler, module_path) = build_disk_cache_compiler(&root);

    // build the same module in a fresh session
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| {
            panic!("failed to resolve main module in fresh session: {error:?}")
        });
    let file_id = program.modules.get(module_id).file_id;
    let file = program.files.get(file_id);

    // load the source file before the direct image read
    if !file.is_loaded() {
        let content = compiler
            .program
            .fs
            .read_to_string(&module_path)
            .unwrap_or_else(|error| panic!("failed to load main.ts: {error}"));
        let loaded_file = File::from_text(
            file_id,
            file.name.clone(),
            file.uri.clone(),
            file.path.clone(),
            file.ty,
            content,
        );
        program.files.replace(loaded_file);
    }
    let file = program.files.get(file_id);

    // verify the persisted image is available before any rebuild
    let loaded = compiler
        .load_ast_image(
            module_id,
            compiler.module_version(module_id),
            file.as_ref(),
            Some(destack_source::LanguageType::TypeScript),
        )
        .unwrap_or_else(|error| panic!("failed to load persisted ast image: {error}"))
        .unwrap_or_else(|| panic!("expected persisted ast image"));

    // compare the full normalized artifact
    let expected_bytes = postcard::to_allocvec(&normalize_ast(expected))
        .unwrap_or_else(|error| panic!("failed to encode expected ast: {error}"));
    let loaded_bytes = postcard::to_allocvec(&normalize_ast(loaded.clone()))
        .unwrap_or_else(|error| panic!("failed to encode loaded ast: {error}"));
    assert_eq!(loaded_bytes, expected_bytes);

    // validate the public compiler path too
    compiler
        .drive(|compiler| compiler.process_ast(module_id))
        .unwrap_or_else(|error| panic!("failed to load ast: {error:?}"));
    let resolved = compiler
        .program
        .artifacts
        .ast(module_id)
        .unwrap_or_else(|| panic!("expected published ast after load"))
        .as_ref()
        .clone();
    let resolved_bytes = postcard::to_allocvec(&normalize_ast(resolved))
        .unwrap_or_else(|error| panic!("failed to encode resolved ast: {error}"));
    assert_eq!(resolved_bytes, expected_bytes);
}

/// Reject one persisted AST image when the source content changes across sessions.
#[test]
fn test_compiler_invalidates_ast_image_when_source_changes() {
    let root = TemporaryPhysicalFileSystem::new_with_prefix("ast_image_invalidation");
    let (session, program, compiler, module_path) = build_disk_cache_compiler(&root);

    // build and persist the initial ast
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    compiler
        .drive(|compiler| compiler.process_ast(module_id))
        .unwrap_or_else(|error| panic!("failed to build ast: {error:?}"));
    let expected = compiler
        .program
        .artifacts
        .ast(module_id)
        .unwrap_or_else(|| panic!("expected published ast"))
        .as_ref()
        .clone();
    let expected_bytes = postcard::to_allocvec(&normalize_ast(expected))
        .unwrap_or_else(|error| panic!("failed to encode expected ast: {error}"));

    drop(compiler);
    drop(program);
    drop(session);

    let (_session, program, compiler, module_path) = build_disk_cache_compiler(&root);

    // change the module source on disk after the fresh session is created
    root.write_text("main.ts", "export const value: string = 'updated';")
        .unwrap_or_else(|error| panic!("failed to update main.ts: {error}"));
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| {
            panic!("failed to resolve main module in fresh session: {error:?}")
        });
    let file_id = program.modules.get(module_id).file_id;
    let file = program.files.get(file_id);

    // load the changed source file before the direct image read
    if !file.is_loaded() {
        let content = compiler
            .program
            .fs
            .read_to_string(&module_path)
            .unwrap_or_else(|error| panic!("failed to load changed main.ts: {error}"));
        let loaded_file = File::from_text(
            file_id,
            file.name.clone(),
            file.uri.clone(),
            file.path.clone(),
            file.ty,
            content,
        );
        program.files.replace(loaded_file);
    }
    let file = program.files.get(file_id);

    // reject the persisted ast image for the changed file
    let loaded = compiler
        .load_ast_image(
            module_id,
            compiler.module_version(module_id),
            file.as_ref(),
            Some(destack_source::LanguageType::TypeScript),
        )
        .unwrap_or_else(|error| panic!("failed to load persisted ast image: {error}"));
    assert!(
        loaded.is_none(),
        "expected changed source to invalidate ast image"
    );

    // rebuild and confirm the ast really changed
    compiler
        .drive(|compiler| compiler.process_ast(module_id))
        .unwrap_or_else(|error| panic!("failed to rebuild ast: {error:?}"));
    let rebuilt = compiler
        .program
        .artifacts
        .ast(module_id)
        .unwrap_or_else(|| panic!("expected rebuilt ast"))
        .as_ref()
        .clone();
    let rebuilt_bytes = postcard::to_allocvec(&normalize_ast(rebuilt))
        .unwrap_or_else(|error| panic!("failed to encode rebuilt ast: {error}"));
    assert_ne!(rebuilt_bytes, expected_bytes);
}

/// Reuse one persisted prepared DIR image across a fresh compiler session.
#[test]
fn test_compiler_reuses_dir_prepared_image_across_sessions() {
    let root = TemporaryPhysicalFileSystem::new_with_prefix("dir_prepared_image");
    let (session, program, compiler, module_path) = build_disk_cache_compiler(&root);

    // build and persist the prepared dir in the first session
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let profile_id = program.default_profile_id_for_module(module_id);
    compiler
        .drive(|compiler| compiler.process_dir_prepared(module_id, profile_id))
        .unwrap_or_else(|error| panic!("failed to build prepared dir: {error:?}"));
    let expected = compiler
        .program
        .artifacts
        .dir_prepared(module_id, profile_id)
        .unwrap_or_else(|| panic!("expected published prepared dir"))
        .as_ref()
        .clone();
    let direct_loaded = compiler
        .load_dir_prepared_image(module_id, compiler.module_version(module_id), profile_id)
        .unwrap_or_else(|error| {
            panic!("failed to read prepared dir image in same session: {error}")
        })
        .unwrap_or_else(|| panic!("expected prepared dir image in same session"));
    let expected_nodes = dump_dir_prepared_nodes(&program.strings, &expected);
    let expected_symbols = dump_dir_prepared_symbols(&program.strings, &expected);
    let loaded_nodes = dump_dir_prepared_nodes(&program.strings, &direct_loaded);
    let loaded_symbols = dump_dir_prepared_symbols(&program.strings, &direct_loaded);
    assert_eq!(loaded_nodes, expected_nodes);
    assert_eq!(loaded_symbols, expected_symbols);
    compiler
        .flush_workspace_index()
        .unwrap_or_else(|error| panic!("failed to flush workspace index: {error}"));

    drop(compiler);
    drop(program);
    drop(session);

    let (_session, program, compiler, module_path) = build_disk_cache_compiler(&root);
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| {
            panic!("failed to resolve main module in fresh session: {error:?}")
        });
    let profile_id = program.default_profile_id_for_module(module_id);

    // load current source state before attempting direct dir image reuse
    compiler
        .drive(|compiler| compiler.process_dir_base(module_id))
        .unwrap_or_else(|error| {
            panic!("failed to build dir base for prepared dir reuse: {error:?}")
        });

    // verify the persisted image is available before any rebuild
    let loaded = compiler
        .load_dir_prepared_image(module_id, compiler.module_version(module_id), profile_id)
        .unwrap_or_else(|error| panic!("failed to load persisted prepared dir image: {error}"))
        .unwrap_or_else(|| panic!("expected persisted prepared dir image"));

    // compare the full dumped artifact surface
    let loaded_nodes = dump_dir_prepared_nodes(&program.strings, &loaded);
    let loaded_symbols = dump_dir_prepared_symbols(&program.strings, &loaded);
    assert_eq!(loaded_nodes, expected_nodes);
    assert_eq!(loaded_symbols, expected_symbols);

    // validate the public compiler path too
    compiler
        .drive(|compiler| compiler.process_dir_prepared(module_id, profile_id))
        .unwrap_or_else(|error| panic!("failed to load prepared dir: {error:?}"));
    let resolved = compiler
        .program
        .artifacts
        .dir_prepared(module_id, profile_id)
        .unwrap_or_else(|| panic!("expected published prepared dir after load"))
        .as_ref()
        .clone();
    let resolved_nodes = dump_dir_prepared_nodes(&program.strings, &resolved);
    let resolved_symbols = dump_dir_prepared_symbols(&program.strings, &resolved);
    assert_eq!(resolved_nodes, expected_nodes);
    assert_eq!(resolved_symbols, expected_symbols);
}

/// Reject one persisted prepared DIR image when the source changes across sessions.
#[test]
fn test_compiler_invalidates_dir_prepared_image_when_source_changes() {
    let root = TemporaryPhysicalFileSystem::new_with_prefix("dir_prepared_image_invalidation");
    let (session, program, compiler, module_path) = build_disk_cache_compiler(&root);

    // build and persist the initial prepared dir
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let profile_id = program.default_profile_id_for_module(module_id);
    compiler
        .drive(|compiler| compiler.process_dir_prepared(module_id, profile_id))
        .unwrap_or_else(|error| panic!("failed to build prepared dir: {error:?}"));
    let expected = compiler
        .program
        .artifacts
        .dir_prepared(module_id, profile_id)
        .unwrap_or_else(|| panic!("expected published prepared dir"))
        .as_ref()
        .clone();
    let expected_nodes = dump_dir_prepared_nodes(&program.strings, &expected);
    let expected_symbols = dump_dir_prepared_symbols(&program.strings, &expected);
    compiler
        .flush_workspace_index()
        .unwrap_or_else(|error| panic!("failed to flush workspace index: {error}"));

    drop(compiler);
    drop(program);
    drop(session);

    // change the module source before the fresh session is created
    root.write_text("main.ts", "export const changed = 'updated';")
        .unwrap_or_else(|error| panic!("failed to update main.ts: {error}"));
    let (_session, program, compiler, module_path) = build_disk_cache_compiler(&root);
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| {
            panic!("failed to resolve main module in fresh session: {error:?}")
        });
    let profile_id = program.default_profile_id_for_module(module_id);

    // load current source state before attempting direct dir image reuse
    compiler
        .drive(|compiler| compiler.process_dir_base(module_id))
        .unwrap_or_else(|error| {
            panic!("failed to build dir base for invalidation check: {error:?}")
        });

    // reject the stale prepared image directly
    let loaded = compiler
        .load_dir_prepared_image(module_id, compiler.module_version(module_id), profile_id)
        .unwrap_or_else(|error| panic!("failed to load prepared dir image: {error}"));
    assert!(
        loaded.is_none(),
        "expected changed source to invalidate prepared dir image"
    );

    // rebuild through the public compiler path and confirm the prepared dir changed
    compiler
        .drive(|compiler| compiler.process_dir_prepared(module_id, profile_id))
        .unwrap_or_else(|error| panic!("failed to rebuild prepared dir: {error:?}"));
    let rebuilt = compiler
        .program
        .artifacts
        .dir_prepared(module_id, profile_id)
        .unwrap_or_else(|| panic!("expected rebuilt prepared dir"))
        .as_ref()
        .clone();
    let rebuilt_nodes = dump_dir_prepared_nodes(&program.strings, &rebuilt);
    let rebuilt_symbols = dump_dir_prepared_symbols(&program.strings, &rebuilt);
    assert_ne!(rebuilt_nodes, expected_nodes);
    assert_ne!(rebuilt_symbols, expected_symbols);
}

/// Reject one persisted prepared DIR image when the workspace string universe changes.
#[test]
fn test_compiler_invalidates_dir_prepared_image_when_workspace_strings_change() {
    let root = TemporaryPhysicalFileSystem::new_with_prefix("dir_prepared_image_strings");
    let (_session, program, compiler, module_path) = build_disk_cache_compiler(&root);

    // build and persist the prepared dir first
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let profile_id = program.default_profile_id_for_module(module_id);
    compiler
        .drive(|compiler| compiler.process_dir_prepared(module_id, profile_id))
        .unwrap_or_else(|error| panic!("failed to build prepared dir: {error:?}"));

    // mutate the shared string universe after the image was produced
    let _new_string = program.strings.intern("new-cache-boundary-string");

    // reject the stale prepared image directly
    let loaded = compiler
        .load_dir_prepared_image(module_id, compiler.module_version(module_id), profile_id)
        .unwrap_or_else(|error| panic!("failed to load prepared dir image: {error}"));
    assert!(
        loaded.is_none(),
        "expected changed workspace strings to invalidate prepared dir image"
    );
}

/// Reuse one persisted resolved DIR image across a fresh compiler session.
#[test]
fn test_compiler_reuses_dir_resolved_image_across_sessions() {
    let root = TemporaryPhysicalFileSystem::new_with_prefix("dir_resolved_image");
    let (session, program, compiler, module_path) = build_disk_cache_compiler(&root);

    // build and persist the resolved dir in the first session
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let profile_id = program.default_profile_id_for_module(module_id);
    compiler
        .drive(|compiler| compiler.process_dir_resolved(module_id, profile_id))
        .unwrap_or_else(|error| panic!("failed to build resolved dir: {error:?}"));
    let expected = compiler
        .program
        .artifacts
        .dir_resolved(module_id, profile_id)
        .unwrap_or_else(|| panic!("expected published resolved dir"))
        .as_ref()
        .clone();
    let direct_loaded = compiler
        .load_dir_resolved_image(module_id, compiler.module_version(module_id), profile_id)
        .unwrap_or_else(|error| {
            panic!("failed to read resolved dir image in same session: {error}")
        })
        .unwrap_or_else(|| panic!("expected resolved dir image in same session"));
    let expected_nodes = dump_dir_resolved_nodes(&program.strings, &expected);
    let expected_symbols = dump_dir_resolved_symbols(&program.strings, &expected);
    let loaded_nodes = dump_dir_resolved_nodes(&program.strings, &direct_loaded);
    let loaded_symbols = dump_dir_resolved_symbols(&program.strings, &direct_loaded);
    assert_eq!(loaded_nodes, expected_nodes);
    assert_eq!(loaded_symbols, expected_symbols);
    compiler
        .flush_workspace_index()
        .unwrap_or_else(|error| panic!("failed to flush workspace index: {error}"));

    drop(compiler);
    drop(program);
    drop(session);

    let (_session, program, compiler, module_path) = build_disk_cache_compiler(&root);
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| {
            panic!("failed to resolve main module in fresh session: {error:?}")
        });
    let profile_id = program.default_profile_id_for_module(module_id);

    // load current source state before attempting direct dir image reuse
    compiler
        .drive(|compiler| compiler.process_dir_base(module_id))
        .unwrap_or_else(|error| {
            panic!("failed to build dir base for resolved dir reuse: {error:?}")
        });

    // verify the persisted image is available before any rebuild
    let loaded = compiler
        .load_dir_resolved_image(module_id, compiler.module_version(module_id), profile_id)
        .unwrap_or_else(|error| panic!("failed to load persisted resolved dir image: {error}"))
        .unwrap_or_else(|| panic!("expected persisted resolved dir image"));

    // compare the full dumped artifact surface
    let loaded_nodes = dump_dir_resolved_nodes(&program.strings, &loaded);
    let loaded_symbols = dump_dir_resolved_symbols(&program.strings, &loaded);
    assert_eq!(loaded_nodes, expected_nodes);
    assert_eq!(loaded_symbols, expected_symbols);

    // validate the public compiler path too
    compiler
        .drive(|compiler| compiler.process_dir_resolved(module_id, profile_id))
        .unwrap_or_else(|error| panic!("failed to load resolved dir: {error:?}"));
    let resolved = compiler
        .program
        .artifacts
        .dir_resolved(module_id, profile_id)
        .unwrap_or_else(|| panic!("expected published resolved dir after load"))
        .as_ref()
        .clone();
    let resolved_nodes = dump_dir_resolved_nodes(&program.strings, &resolved);
    let resolved_symbols = dump_dir_resolved_symbols(&program.strings, &resolved);
    assert_eq!(resolved_nodes, expected_nodes);
    assert_eq!(resolved_symbols, expected_symbols);
}

/// Reject one persisted resolved DIR image when the source changes across sessions.
#[test]
fn test_compiler_invalidates_dir_resolved_image_when_source_changes() {
    let root = TemporaryPhysicalFileSystem::new_with_prefix("dir_resolved_image_invalidation");
    let (session, program, compiler, module_path) = build_disk_cache_compiler(&root);

    // build and persist the initial resolved dir
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let profile_id = program.default_profile_id_for_module(module_id);
    compiler
        .drive(|compiler| compiler.process_dir_resolved(module_id, profile_id))
        .unwrap_or_else(|error| panic!("failed to build resolved dir: {error:?}"));
    let expected = compiler
        .program
        .artifacts
        .dir_resolved(module_id, profile_id)
        .unwrap_or_else(|| panic!("expected published resolved dir"))
        .as_ref()
        .clone();
    let expected_nodes = dump_dir_resolved_nodes(&program.strings, &expected);
    let expected_symbols = dump_dir_resolved_symbols(&program.strings, &expected);
    compiler
        .flush_workspace_index()
        .unwrap_or_else(|error| panic!("failed to flush workspace index: {error}"));

    drop(compiler);
    drop(program);
    drop(session);

    // change the module source before the fresh session is created
    root.write_text("main.ts", "export const changed = 'updated';")
        .unwrap_or_else(|error| panic!("failed to update main.ts: {error}"));
    let (_session, program, compiler, module_path) = build_disk_cache_compiler(&root);
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| {
            panic!("failed to resolve main module in fresh session: {error:?}")
        });
    let profile_id = program.default_profile_id_for_module(module_id);

    // load current source state before attempting direct dir image reuse
    compiler
        .drive(|compiler| compiler.process_dir_base(module_id))
        .unwrap_or_else(|error| {
            panic!("failed to build dir base for invalidation check: {error:?}")
        });

    // reject the stale resolved image directly
    let loaded = compiler
        .load_dir_resolved_image(module_id, compiler.module_version(module_id), profile_id)
        .unwrap_or_else(|error| panic!("failed to load resolved dir image: {error}"));
    assert!(
        loaded.is_none(),
        "expected changed source to invalidate resolved dir image"
    );

    // rebuild through the public compiler path and confirm the resolved dir changed
    compiler
        .drive(|compiler| compiler.process_dir_resolved(module_id, profile_id))
        .unwrap_or_else(|error| panic!("failed to rebuild resolved dir: {error:?}"));
    let rebuilt = compiler
        .program
        .artifacts
        .dir_resolved(module_id, profile_id)
        .unwrap_or_else(|| panic!("expected rebuilt resolved dir"))
        .as_ref()
        .clone();
    let rebuilt_nodes = dump_dir_resolved_nodes(&program.strings, &rebuilt);
    let rebuilt_symbols = dump_dir_resolved_symbols(&program.strings, &rebuilt);
    assert_ne!(rebuilt_nodes, expected_nodes);
    assert_ne!(rebuilt_symbols, expected_symbols);
}

/// Reject one persisted resolved DIR image when the workspace string universe changes.
#[test]
fn test_compiler_invalidates_dir_resolved_image_when_workspace_strings_change() {
    let root = TemporaryPhysicalFileSystem::new_with_prefix("dir_resolved_image_strings");
    let (_session, program, compiler, module_path) = build_disk_cache_compiler(&root);

    // build and persist the resolved dir first
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let profile_id = program.default_profile_id_for_module(module_id);
    compiler
        .drive(|compiler| compiler.process_dir_resolved(module_id, profile_id))
        .unwrap_or_else(|error| panic!("failed to build resolved dir: {error:?}"));

    // mutate the shared string universe after the image was produced
    let _new_string = program.strings.intern("new-cache-boundary-string");

    // reject the stale resolved image directly
    let loaded = compiler
        .load_dir_resolved_image(module_id, compiler.module_version(module_id), profile_id)
        .unwrap_or_else(|error| panic!("failed to load resolved dir image: {error}"));
    assert!(
        loaded.is_none(),
        "expected changed workspace strings to invalidate resolved dir image"
    );
}

/// Skip artifact image loads when persistent cache is disabled.
#[test]
fn test_compiler_skips_artifact_image_loads_when_disk_cache_is_disabled() {
    let root = TemporaryPhysicalFileSystem::new_with_prefix("artifact_image_load_off");
    let (_session, _program, compiler, module_path) = build_memory_cache_compiler(&root);
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let artifact_key = destack_workspace::ArtifactKey::ast(module_id);

    // the image loader closure should not run at all
    let loaded = compiler.load_artifact::<(), _>(&artifact_key, |_| {
        panic!("artifact image load should be skipped when disk cache is disabled")
    });

    assert!(
        loaded.is_none(),
        "expected no image load when disk cache is off"
    );
}

/// Skip artifact image writes when persistent cache is disabled.
#[test]
fn test_compiler_skips_artifact_image_writes_when_disk_cache_is_disabled() {
    let root = TemporaryPhysicalFileSystem::new_with_prefix("artifact_image_store_off");
    let (_session, _program, compiler, module_path) = build_memory_cache_compiler(&root);
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let artifact_key = destack_workspace::ArtifactKey::ast(module_id);

    // the image store closure should not run at all
    compiler.store_artifact(&artifact_key, &(), |_, _| {
        panic!("artifact image store should be skipped when disk cache is disabled")
    });
}
