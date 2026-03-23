use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactFamily, ArtifactImage, ArtifactImageError, ArtifactImageHeader,
    ArtifactImageKey, ArtifactImageRequirement, ArtifactImageStore, ArtifactKey, Ast, AstImage,
    CacheStore, DirPreparedImage, LanguageEnvironment, MemoryCacheStore, PersistedImageValidation,
};
use destack_builtin::LanguageSymbol;
use destack_source::{File, FileVersion, TemporaryPhysicalFileSystem};

use crate::tests::scenario::{
    append_file_text, build_disk_cache_compiler, build_memory_cache_compiler,
    dump_dir_prepared_nodes, dump_dir_prepared_symbols, dump_dir_resolved_nodes,
    dump_dir_resolved_symbols, normalize_ast, test_profile_key,
};

/// Persist and load one language environment image through the artifact store.
#[test]
fn test_artifact_store_roundtrips_language_environment_image() {
    let cache_root = std::path::PathBuf::from("/artifact-store-test");
    let cache_store: Arc<dyn CacheStore> = Arc::new(MemoryCacheStore::new());
    let artifact_store = ArtifactImageStore::new(cache_store.as_ref(), &cache_root);
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

/// Reject one persisted image header with a tampered validation hash.
#[test]
fn test_artifact_store_rejects_tampered_validation_hash() {
    let cache_root = std::path::PathBuf::from("/artifact-store-test");
    let cache_store: Arc<dyn CacheStore> = Arc::new(MemoryCacheStore::new());
    let artifact_store = ArtifactImageStore::new(cache_store.as_ref(), &cache_root);
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

    // write a tampered image with the same stable key
    let mut tampered = image.clone();
    tampered.header.validation_hash = tampered.header.validation_hash.wrapping_add(1);
    artifact_store
        .save(&tampered)
        .unwrap_or_else(|error| panic!("failed to save tampered artifact image: {error}"));

    // reject the tampered header before payload deserialization
    let error = artifact_store
        .load_header(&tampered.header.artifact_image_key)
        .err()
        .unwrap_or_else(|| panic!("expected tampered header to fail"));

    match error {
        ArtifactImageError::InvalidValidationHash { .. } => {}
        other => panic!("expected invalid validation hash, found {other}"),
    }
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
        .run_to_completion(|compiler| compiler.process_language_environment(profile_id))
        .unwrap_or_else(|error| panic!("failed to persist language environment: {error:?}"));
    let expected = compiler
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

/// Invalidate a persisted language environment image when builtin source changes.
#[test]
fn test_language_environment_image_tracks_builtin_source_content() {
    let root = TemporaryPhysicalFileSystem::new_with_prefix("language_environment_source_hash");
    let (_session, program, compiler, module_path) = build_disk_cache_compiler(&root);

    // persist the language environment once
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let profile_id = program.default_profile_id_for_module(module_id);
    compiler
        .run_to_completion(|compiler| compiler.process_language_environment(profile_id))
        .unwrap_or_else(|error| panic!("failed to persist language environment: {error:?}"));
    assert!(
        compiler
            .load_language_environment_image(profile_id)
            .unwrap_or_else(|error| panic!(
                "failed to load persisted language environment: {error}"
            ))
            .is_some()
    );

    // perturb one builtin source file that contributes to the language environment
    let builtins = program
        .builtins
        .as_ref()
        .unwrap_or_else(|| panic!("expected builtin registry"));
    let builtin_module_id = builtins.module_for_item(LanguageSymbol::Add);
    let builtin_file_id = program.modules.get(builtin_module_id).file_id;
    append_file_text(
        program.as_ref(),
        builtin_file_id,
        "\n// changed for cache invalidation\n",
    );

    // the persisted image should now be rejected
    assert!(
        compiler
            .load_language_environment_image(profile_id)
            .unwrap_or_else(|error| panic!("failed to reload language environment image: {error}"))
            .is_none()
    );
}

/// Invalidate a persisted library environment image when library source changes.
#[test]
fn test_library_environment_image_tracks_library_source_content() {
    let root = TemporaryPhysicalFileSystem::new_with_prefix("library_environment_source_hash");
    let (_session, program, compiler, module_path) = build_disk_cache_compiler(&root);

    // persist the library environment once
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let profile_id = program.default_profile_id_for_module(module_id);
    compiler
        .run_to_completion(|compiler| compiler.process_library_environment(profile_id))
        .unwrap_or_else(|error| panic!("failed to persist library environment: {error:?}"));
    assert!(
        compiler
            .load_library_environment_image(profile_id)
            .unwrap_or_else(|error| panic!("failed to load persisted library environment: {error}"))
            .is_some()
    );

    // perturb one selected library module source file
    let environment = compiler
        .artifacts
        .library_environment(profile_id)
        .unwrap_or_else(|| panic!("expected published library environment"));
    let builtin_module_id = *environment
        .modules
        .first()
        .unwrap_or_else(|| panic!("expected selected library modules"));
    let builtin_file_id = program.modules.get(builtin_module_id).file_id;
    append_file_text(
        program.as_ref(),
        builtin_file_id,
        "\n// changed for cache invalidation\n",
    );

    // the persisted image should now be rejected
    assert!(
        compiler
            .load_library_environment_image(profile_id)
            .unwrap_or_else(|error| panic!("failed to reload library environment image: {error}"))
            .is_none()
    );
}

/// Invalidate a persisted intrinsic environment image when builtin source changes.
#[test]
fn test_intrinsic_environment_image_tracks_builtin_source_content() {
    let root = TemporaryPhysicalFileSystem::new_with_prefix("intrinsic_environment_source_hash");
    let (_session, program, compiler, module_path) = build_disk_cache_compiler(&root);

    // persist the intrinsic environment once
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let profile_id = program.default_profile_id_for_module(module_id);
    compiler
        .run_to_completion(|compiler| compiler.process_intrinsic_environment(profile_id))
        .unwrap_or_else(|error| panic!("failed to persist intrinsic environment: {error:?}"));
    assert!(
        compiler
            .load_intrinsic_environment_image(profile_id)
            .unwrap_or_else(|error| panic!(
                "failed to load persisted intrinsic environment: {error}"
            ))
            .is_some()
    );

    // perturb one builtin source file that contributes intrinsic bindings
    let builtins = program
        .builtins
        .as_ref()
        .unwrap_or_else(|| panic!("expected builtin registry"));
    let builtin_module_id = *builtins
        .intrinsic_module_ids()
        .first()
        .unwrap_or_else(|| panic!("expected intrinsic builtin modules"));
    let builtin_file_id = program.modules.get(builtin_module_id).file_id;
    append_file_text(
        program.as_ref(),
        builtin_file_id,
        "\n// changed for cache invalidation\n",
    );

    // the persisted image should now be rejected
    assert!(
        compiler
            .load_intrinsic_environment_image(profile_id)
            .unwrap_or_else(|error| panic!("failed to reload intrinsic environment image: {error}"))
            .is_none()
    );
}

/// Invalidate a resolved directory when its library environment dependency changes.
#[test]
fn test_resolved_dir_tracks_library_environment_requirements() {
    let root =
        TemporaryPhysicalFileSystem::new_with_prefix("resolved_dir_environment_requirements");
    let (_session, program, compiler, module_path) = build_memory_cache_compiler(&root);

    // build the resolved directory through the public requirement path
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let profile_id = program.default_profile_id_for_module(module_id);
    compiler
        .run_to_completion(|compiler| compiler.require_dir_resolved(module_id, profile_id))
        .unwrap_or_else(|error| panic!("failed to require resolved dir: {error:?}"));

    let resolved_key = ArtifactKey::dir_resolved(module_id, profile_id);
    assert!(compiler.artifact_key_is_available(&resolved_key));

    // perturb the published library environment dependency
    let environment_key = ArtifactKey::library_environment(profile_id);
    let dependency = compiler
        .artifacts
        .dependency(&environment_key)
        .unwrap_or_else(|| panic!("expected published library environment dependency"));
    let bumped_dependency = ArtifactDependency::new(dependency.0.wrapping_add(1));
    compiler
        .artifacts
        .set_dependency(environment_key, bumped_dependency);

    // the resolved dir should now be considered stale
    assert!(!compiler.artifact_key_is_available(&resolved_key));
}

/// Invalidate a library environment when one recorded exact requirement changes.
#[test]
fn test_library_environment_tracks_exact_requirements() {
    let root = TemporaryPhysicalFileSystem::new_with_prefix("library_environment_requirements");
    let (_session, program, compiler, module_path) = build_memory_cache_compiler(&root);

    // build the library environment through the public requirement path
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let profile_id = program.default_profile_id_for_module(module_id);
    compiler
        .run_to_completion(|compiler| compiler.require_library_environment(profile_id))
        .unwrap_or_else(|error| panic!("failed to require library environment: {error:?}"));

    let environment_key = ArtifactKey::library_environment(profile_id);
    assert!(compiler.artifact_key_is_available(&environment_key));

    // inspect the recorded exact requirements for the completed task
    let handle = compiler
        .task_handles()
        .into_iter()
        .find(|handle| handle.artifact_key == environment_key)
        .unwrap_or_else(|| panic!("expected library environment task handle"));
    let requirement = handle
        .final_requirements
        .first()
        .cloned()
        .unwrap_or_else(|| panic!("expected library environment exact requirements"));

    // perturb one recorded requirement dependency
    let bumped_dependency = ArtifactDependency::new(requirement.dependency.0.wrapping_add(1));
    compiler
        .artifacts
        .set_dependency(requirement.key, bumped_dependency);

    // the environment should now be considered stale
    assert!(!compiler.artifact_key_is_available(&environment_key));
}

/// Persist and load one AST image through the artifact store.
#[test]
fn test_artifact_store_roundtrips_ast_image() {
    let cache_root = std::path::PathBuf::from("/artifact-store-test");
    let cache_store: Arc<dyn CacheStore> = Arc::new(MemoryCacheStore::new());
    let artifact_store = ArtifactImageStore::new(cache_store.as_ref(), &cache_root);
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
        .run_to_completion(|compiler| compiler.process_ast(module_id))
        .unwrap_or_else(|error| panic!("failed to build ast: {error:?}"));
    let expected = compiler
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
        .run_to_completion(|compiler| compiler.process_ast(module_id))
        .unwrap_or_else(|error| panic!("failed to load ast: {error:?}"));
    let resolved = compiler
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
        .run_to_completion(|compiler| compiler.process_ast(module_id))
        .unwrap_or_else(|error| panic!("failed to build ast: {error:?}"));
    let expected = compiler
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
        .run_to_completion(|compiler| compiler.process_ast(module_id))
        .unwrap_or_else(|error| panic!("failed to rebuild ast: {error:?}"));
    let rebuilt = compiler
        .artifacts
        .ast(module_id)
        .unwrap_or_else(|| panic!("expected rebuilt ast"))
        .as_ref()
        .clone();
    let rebuilt_bytes = postcard::to_allocvec(&normalize_ast(rebuilt))
        .unwrap_or_else(|error| panic!("failed to encode rebuilt ast: {error}"));
    assert_ne!(rebuilt_bytes, expected_bytes);
}

/// Keep persisted image validation contracts centralized on artifact families.
#[test]
fn test_artifact_families_classify_persisted_image_validation() {
    // self-contained families
    assert_eq!(
        ArtifactFamily::LanguageEnvironment.persisted_image_validation(),
        PersistedImageValidation::SelfContained
    );
    assert_eq!(
        ArtifactFamily::IntrinsicEnvironment.persisted_image_validation(),
        PersistedImageValidation::SelfContained
    );
    assert_eq!(
        ArtifactFamily::LibraryEnvironment.persisted_image_validation(),
        PersistedImageValidation::SelfContained
    );
    assert_eq!(
        ArtifactFamily::Ast.persisted_image_validation(),
        PersistedImageValidation::SelfContained
    );
    assert_eq!(
        ArtifactFamily::DirBase.persisted_image_validation(),
        PersistedImageValidation::SelfContained
    );

    // dependency-validated families
    assert_eq!(
        ArtifactFamily::ModuleGraph.persisted_image_validation(),
        PersistedImageValidation::SelfContained
    );
    assert_eq!(
        ArtifactFamily::DirPrepared.persisted_image_validation(),
        PersistedImageValidation::DependencyValidated
    );
    assert_eq!(
        ArtifactFamily::DirResolved.persisted_image_validation(),
        PersistedImageValidation::DependencyValidated
    );
    assert_eq!(
        ArtifactFamily::DirDeclared.persisted_image_validation(),
        PersistedImageValidation::DependencyValidated
    );
    assert_eq!(
        ArtifactFamily::DirInterface.persisted_image_validation(),
        PersistedImageValidation::DependencyValidated
    );
    assert_eq!(
        ArtifactFamily::DirAnalyzed.persisted_image_validation(),
        PersistedImageValidation::DependencyValidated
    );
    assert_eq!(
        ArtifactFamily::DirElaborated.persisted_image_validation(),
        PersistedImageValidation::DependencyValidated
    );
    assert_eq!(
        ArtifactFamily::DirPatched.persisted_image_validation(),
        PersistedImageValidation::DependencyValidated
    );
    assert_eq!(
        ArtifactFamily::MirBase.persisted_image_validation(),
        PersistedImageValidation::DependencyValidated
    );
    assert_eq!(
        ArtifactFamily::MirOptimized.persisted_image_validation(),
        PersistedImageValidation::DependencyValidated
    );
    assert_eq!(
        ArtifactFamily::ModuleArtifact.persisted_image_validation(),
        PersistedImageValidation::DependencyValidated
    );
    assert_eq!(
        ArtifactFamily::PackageOutput.persisted_image_validation(),
        PersistedImageValidation::DependencyValidated
    );
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
        .run_to_completion(|compiler| compiler.process_dir_prepared(module_id, profile_id))
        .unwrap_or_else(|error| panic!("failed to build prepared dir: {error:?}"));
    let expected = compiler
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

    let (_session, program, compiler, module_path) = build_disk_cache_compiler(&root);
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| {
            panic!("failed to resolve main module in fresh session: {error:?}")
        });
    let profile_id = program.default_profile_id_for_module(module_id);

    // load current source state before attempting direct dir image reuse
    compiler
        .run_to_completion(|compiler| compiler.process_dir_base(module_id))
        .unwrap_or_else(|error| {
            panic!("failed to build dir base for prepared dir reuse: {error:?}")
        });

    // verify the persisted image is available before any rebuild
    let loaded = compiler
        .load_dir_prepared_image(module_id, compiler.module_version(module_id), profile_id)
        .unwrap_or_else(|error| panic!("failed to load prepared dir image: {error}"));
    let loaded = loaded.unwrap_or_else(|| panic!("expected persisted prepared dir image"));
    let loaded_nodes = dump_dir_prepared_nodes(&program.strings, &loaded);
    let loaded_symbols = dump_dir_prepared_symbols(&program.strings, &loaded);
    assert_eq!(loaded_nodes, expected_nodes);
    assert_eq!(loaded_symbols, expected_symbols);
}

/// Reject one persisted DIR image without creating live profiles from its proof metadata.
#[test]
fn test_compiler_does_not_create_profiles_from_persisted_requirements() {
    let root = TemporaryPhysicalFileSystem::new_with_prefix("dir_prepared_profile_proof_lookup");
    let (session, program, compiler, module_path) = build_disk_cache_compiler(&root);

    // build and persist the prepared dir first
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let profile_id = program.default_profile_id_for_module(module_id);
    compiler
        .run_to_completion(|compiler| compiler.process_dir_prepared(module_id, profile_id))
        .unwrap_or_else(|error| panic!("failed to build prepared dir: {error:?}"));
    compiler
        .flush_workspace_index()
        .unwrap_or_else(|error| panic!("failed to flush workspace index: {error}"));

    drop(compiler);
    drop(program);
    drop(session);

    let (session, program, compiler, module_path) = build_disk_cache_compiler(&root);
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| {
            panic!("failed to resolve main module in fresh session: {error:?}")
        });
    let profile_id = program.default_profile_id_for_module(module_id);
    let artifact_store =
        ArtifactImageStore::new(session.cache_store.as_ref(), &session.workspace_cache_dir());
    let image_key = ArtifactImageKey::DirPrepared {
        module: module_id,
        profile: program.profile(profile_id).key.clone(),
    };
    let image = artifact_store
        .load::<DirPreparedImage>(&image_key)
        .unwrap_or_else(|error| panic!("failed to load prepared dir image: {error}"))
        .unwrap_or_else(|| panic!("expected prepared dir image"));

    // replace the requirement list with one bogus profile scoped proof
    let mut bogus_profile = test_profile_key();
    bogus_profile.debug = !bogus_profile.debug;
    let tampered = ArtifactImage::new(
        image
            .header
            .with_requirements(vec![ArtifactImageRequirement {
                key: ArtifactImageKey::LanguageEnvironment {
                    profile: bogus_profile,
                },
                validation_hash: 0,
            }]),
        image.payload.clone(),
    )
    .unwrap_or_else(|error| panic!("failed to build tampered dir image: {error}"));
    artifact_store
        .save(&tampered)
        .unwrap_or_else(|error| panic!("failed to save tampered dir image: {error}"));

    // prepare the current source state and capture the live profile count
    compiler
        .run_to_completion(|compiler| compiler.process_dir_base(module_id))
        .unwrap_or_else(|error| {
            panic!("failed to build dir base for prepared dir load: {error:?}")
        });
    let profile_count_before = program.profiles.len();

    // reject the tampered image without allocating a new live profile
    let loaded = compiler
        .load_dir_prepared_image(module_id, compiler.module_version(module_id), profile_id)
        .unwrap_or_else(|error| panic!("failed to load prepared dir image: {error}"));
    assert!(
        loaded.is_none(),
        "expected tampered prepared dir image to fail"
    );
    assert_eq!(program.profiles.len(), profile_count_before);
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
        .run_to_completion(|compiler| compiler.process_dir_resolved(module_id, profile_id))
        .unwrap_or_else(|error| panic!("failed to build resolved dir: {error:?}"));
    let expected = compiler
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

    let (_session, program, compiler, module_path) = build_disk_cache_compiler(&root);
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| {
            panic!("failed to resolve main module in fresh session: {error:?}")
        });
    let profile_id = program.default_profile_id_for_module(module_id);

    // load current source state before attempting direct dir image reuse
    compiler
        .run_to_completion(|compiler| compiler.process_dir_base(module_id))
        .unwrap_or_else(|error| {
            panic!("failed to build dir base for resolved dir reuse: {error:?}")
        });

    // verify the persisted image is available before any rebuild
    let loaded = compiler
        .load_dir_resolved_image(module_id, compiler.module_version(module_id), profile_id)
        .unwrap_or_else(|error| panic!("failed to load resolved dir image: {error}"));
    let loaded = loaded.unwrap_or_else(|| panic!("expected persisted resolved dir image"));
    let loaded_nodes = dump_dir_resolved_nodes(&program.strings, &loaded);
    let loaded_symbols = dump_dir_resolved_symbols(&program.strings, &loaded);
    assert_eq!(loaded_nodes, expected_nodes);
    assert_eq!(loaded_symbols, expected_symbols);
}

/// Skip artifact image loads when persistent cache is disabled.
#[test]
fn test_compiler_skips_artifact_image_loads_when_disk_cache_is_disabled() {
    let root = TemporaryPhysicalFileSystem::new_with_prefix("artifact_image_load_off");
    let (_session, _program, compiler, module_path) = build_memory_cache_compiler(&root);
    let module_id = compiler
        .resolve_path_to_module(&module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let artifact_key = destack_artifact::ArtifactKey::ast(module_id);

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
    let artifact_key = destack_artifact::ArtifactKey::ast(module_id);

    // the image store closure should not run at all
    compiler.store_artifact(&artifact_key, &(), |_, _| {
        panic!("artifact image store should be skipped when disk cache is disabled")
    });
}
