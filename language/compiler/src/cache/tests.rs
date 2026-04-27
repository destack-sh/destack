use std::path::Path;
use std::sync::Arc;

use destack_artifact::{
    ArtifactCache, ArtifactCacheLayout, ArtifactFamily, ArtifactImage, ArtifactImageError,
    ArtifactImageHeader, ArtifactImageKey, ArtifactKey, Ast, CacheStore, LanguageEnvironment,
    MemoryCacheStore, PersistedImageValidation,
};
use destack_builtin::LanguageSymbol;
use destack_source::TemporaryPhysicalFileSystem;

use super::store::LANGUAGE_CACHE_ABI;
use crate::run_to_completion;
use crate::tests::scenario::{
    append_file_text, build_disk_cache_compiler, build_memory_cache_compiler, normalize_ast,
    test_profile_key,
};

/// Persist and load one language environment image through the artifact cache.
#[test]
fn test_artifact_cache_roundtrips_language_environment_image() {
    let cache_root = std::path::PathBuf::from("/artifact-store-test");
    let cache_store: Arc<dyn CacheStore> = Arc::new(MemoryCacheStore::new());
    let cache_layout = ArtifactCacheLayout::new(
        &cache_root,
        Path::new("/workspace"),
        LANGUAGE_CACHE_ABI,
        false,
    );
    let artifact_cache = ArtifactCache::new(cache_store.as_ref(), &cache_layout);
    let profile = test_profile_key();
    let header = ArtifactImageHeader::new(ArtifactImageKey::LanguageEnvironment { profile }, 0);
    let image = ArtifactImage::new(header, LanguageEnvironment::default())
        .unwrap_or_else(|error| panic!("failed to build artifact image: {error}"));

    // write the image first
    artifact_cache
        .save(&image)
        .unwrap_or_else(|error| panic!("failed to save artifact image: {error}"));

    // load the same image back
    let loaded = artifact_cache
        .load::<LanguageEnvironment>(&image.header.artifact_image_key)
        .unwrap_or_else(|error| panic!("failed to load artifact image: {error}"))
        .unwrap_or_else(|| panic!("expected stored artifact image"));

    // compare the full image contract
    assert_eq!(loaded.header, image.header);
    assert_eq!(loaded.payload.items, image.payload.items);
    assert_eq!(loaded.payload.symbols, image.payload.symbols);
}

/// Reject one persisted image when its stored content id no longer matches.
#[test]
fn test_artifact_cache_rejects_tampered_content_id() {
    let cache_root = std::path::PathBuf::from("/artifact-store-test");
    let cache_store: Arc<dyn CacheStore> = Arc::new(MemoryCacheStore::new());
    let cache_layout = ArtifactCacheLayout::new(
        &cache_root,
        Path::new("/workspace"),
        LANGUAGE_CACHE_ABI,
        false,
    );
    let artifact_cache = ArtifactCache::new(cache_store.as_ref(), &cache_layout);
    let profile = test_profile_key();
    let header = ArtifactImageHeader::new(ArtifactImageKey::LanguageEnvironment { profile }, 0);
    let image = ArtifactImage::new(header, LanguageEnvironment::default())
        .unwrap_or_else(|error| panic!("failed to build artifact image: {error}"));

    // write the image and then corrupt its persisted bytes in place
    let content_id = artifact_cache
        .save(&image)
        .unwrap_or_else(|error| panic!("failed to save artifact image: {error}"));
    let content_path = cache_layout
        .content_root()
        .join(&content_id.to_hex()[0..2])
        .join(format!("{}.bin", content_id.to_hex()));
    cache_store
        .write(&content_path, b"tampered")
        .unwrap_or_else(|error| panic!("failed to corrupt artifact content: {error}"));

    // reject the tampered bytes before payload deserialization
    let error = artifact_cache
        .load_header(&image.header.artifact_image_key)
        .err()
        .unwrap_or_else(|| panic!("expected tampered image to fail"));

    match error {
        ArtifactImageError::InvalidContentId { .. } => {}
        other => panic!("expected invalid content id, found {other}"),
    }
}

/// Reuse one persisted language environment across a fresh compiler session.
#[test]
fn test_compiler_reuses_language_environment_image_across_sessions() {
    let root = TemporaryPhysicalFileSystem::new_with_prefix("language_environment_image");
    let (session, program, compiler, module_path) = build_disk_cache_compiler(&root);

    // build and persist the environment in the first session
    let module_id = compiler
        .resolve_path_to_module(program.current_revision(), &module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let profile_id = compiler
        .context(program.current_revision())
        .unwrap_or_else(|message| panic!("{message}"))
        .default_profile_id_for_module(module_id);
    run_to_completion(
        &compiler,
        program.current_revision(),
        |compiler, context| compiler.process_language_environment(profile_id, context),
    )
    .unwrap_or_else(|error| panic!("failed to persist language environment: {error:?}"));
    let expected = compiler
        .repository
        .language_environment(program.current_revision(), profile_id)
        .unwrap_or_else(|| panic!("expected published language environment"))
        .as_ref()
        .clone();

    drop(compiler);
    drop(program);
    drop(session);

    let (_session, program, compiler, module_path) = build_disk_cache_compiler(&root);

    // build the same stable profile in a fresh session
    let module_id = compiler
        .resolve_path_to_module(program.current_revision(), &module_path)
        .unwrap_or_else(|error| {
            panic!("failed to resolve main module in fresh session: {error:?}")
        });
    let profile_id = compiler
        .context(program.current_revision())
        .unwrap_or_else(|message| panic!("{message}"))
        .default_profile_id_for_module(module_id);

    // verify the persisted image is available before any rebuild
    let loaded = compiler
        .load_language_environment_image(program.current_revision(), profile_id)
        .unwrap_or_else(|error| panic!("failed to load persisted language environment: {error}"))
        .unwrap_or_else(|| panic!("expected persisted language environment image"));

    // compare the full semantic surface
    assert_eq!(loaded.items, expected.items);
    assert_eq!(loaded.symbols, expected.symbols);

    // validate the public compiler path too
    let resolved = run_to_completion(
        &compiler,
        program.current_revision(),
        |compiler, _context| compiler.resolve_language_environment(_context.revision(), profile_id),
    )
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
        .resolve_path_to_module(program.current_revision(), &module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let profile_id = compiler
        .context(program.current_revision())
        .unwrap_or_else(|message| panic!("{message}"))
        .default_profile_id_for_module(module_id);
    run_to_completion(
        &compiler,
        program.current_revision(),
        |compiler, context| compiler.process_language_environment(profile_id, context),
    )
    .unwrap_or_else(|error| panic!("failed to persist language environment: {error:?}"));
    assert!(
        compiler
            .load_language_environment_image(program.current_revision(), profile_id)
            .unwrap_or_else(|error| panic!(
                "failed to load persisted language environment: {error}"
            ))
            .is_some()
    );

    // perturb one builtin source file that contributes to the language environment
    let builtins = program.builtins();
    let builtin_module_id = builtins.module_for_item(LanguageSymbol::Add);
    let builtin_file_id = program.module_descriptor(builtin_module_id).file_id;
    append_file_text(
        program.as_ref(),
        builtin_file_id,
        "\n// changed for cache invalidation\n",
    );

    // the persisted image should now be rejected
    assert!(
        compiler
            .load_language_environment_image(program.current_revision(), profile_id)
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
        .resolve_path_to_module(program.current_revision(), &module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let profile_id = compiler
        .context(program.current_revision())
        .unwrap_or_else(|message| panic!("{message}"))
        .default_profile_id_for_module(module_id);
    run_to_completion(
        &compiler,
        program.current_revision(),
        |compiler, context| compiler.process_library_environment(profile_id, context),
    )
    .unwrap_or_else(|error| panic!("failed to persist library environment: {error:?}"));
    assert!(
        compiler
            .load_library_environment_image(program.current_revision(), profile_id)
            .unwrap_or_else(|error| panic!("failed to load persisted library environment: {error}"))
            .is_some()
    );

    // perturb one selected library module source file
    let environment = compiler
        .repository
        .library_environment(program.current_revision(), profile_id)
        .unwrap_or_else(|| panic!("expected published library environment"));
    let builtin_module_id = *environment
        .modules
        .first()
        .unwrap_or_else(|| panic!("expected selected library modules"));
    let builtin_file_id = program.module_descriptor(builtin_module_id).file_id;
    append_file_text(
        program.as_ref(),
        builtin_file_id,
        "\n// changed for cache invalidation\n",
    );

    // the persisted image should now be rejected
    assert!(
        compiler
            .load_library_environment_image(program.current_revision(), profile_id)
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
        .resolve_path_to_module(program.current_revision(), &module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let profile_id = compiler
        .context(program.current_revision())
        .unwrap_or_else(|message| panic!("{message}"))
        .default_profile_id_for_module(module_id);
    run_to_completion(
        &compiler,
        program.current_revision(),
        |compiler, context| compiler.process_intrinsic_environment(profile_id, context),
    )
    .unwrap_or_else(|error| panic!("failed to persist intrinsic environment: {error:?}"));
    assert!(
        compiler
            .load_intrinsic_environment_image(program.current_revision(), profile_id)
            .unwrap_or_else(|error| panic!(
                "failed to load persisted intrinsic environment: {error}"
            ))
            .is_some()
    );

    // perturb one builtin source file that contributes intrinsic bindings
    let builtins = program.builtins();
    let builtin_module_id = *builtins
        .intrinsic_module_ids()
        .first()
        .unwrap_or_else(|| panic!("expected intrinsic builtin modules"));
    let builtin_file_id = program.module_descriptor(builtin_module_id).file_id;
    append_file_text(
        program.as_ref(),
        builtin_file_id,
        "\n// changed for cache invalidation\n",
    );

    // the persisted image should now be rejected
    assert!(
        compiler
            .load_intrinsic_environment_image(program.current_revision(), profile_id)
            .unwrap_or_else(|error| panic!("failed to reload intrinsic environment image: {error}"))
            .is_none()
    );
}

/// Keep one resolved directory available after evicting its library environment artifact.
#[test]
fn test_resolved_dir_stays_available_after_library_environment_eviction() {
    let root =
        TemporaryPhysicalFileSystem::new_with_prefix("resolved_dir_environment_requirements");
    let (_session, program, compiler, module_path) = build_memory_cache_compiler(&root);

    // build the resolved directory through the public requirement path
    let module_id = compiler
        .resolve_path_to_module(program.current_revision(), &module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let profile_id = compiler
        .context(program.current_revision())
        .unwrap_or_else(|message| panic!("{message}"))
        .default_profile_id_for_module(module_id);
    run_to_completion(
        &compiler,
        program.current_revision(),
        |compiler, _context| {
            compiler.require_dir_resolved(_context.revision(), module_id, profile_id)
        },
    )
    .unwrap_or_else(|error| panic!("failed to require resolved dir: {error:?}"));

    let resolved_key = ArtifactKey::dir_resolved(module_id, profile_id);
    assert!(compiler.artifact_key_is_available(program.current_revision(), &resolved_key));

    // evict the published library environment version
    let environment_key = ArtifactKey::library_environment(profile_id);
    let environment_version =
        compiler.artifact_version_for_revision(program.current_revision(), &environment_key);
    compiler.artifacts.evict(&environment_version);

    // published artifact availability is direct, not recursive through evicted dependencies
    assert!(compiler.artifact_key_is_available(program.current_revision(), &resolved_key));

    run_to_completion(
        &compiler,
        program.current_revision(),
        |compiler, _context| {
            compiler.require_dir_resolved(_context.revision(), module_id, profile_id)
        },
    )
    .unwrap_or_else(|error| panic!("failed to reuse resolved dir: {error:?}"));
}

/// Persist and load one AST image through the artifact cache.
#[test]
fn test_artifact_cache_roundtrips_ast_image() {
    let cache_root = std::path::PathBuf::from("/artifact-store-test");
    let cache_store: Arc<dyn CacheStore> = Arc::new(MemoryCacheStore::new());
    let cache_layout = ArtifactCacheLayout::new(
        &cache_root,
        Path::new("/workspace"),
        LANGUAGE_CACHE_ABI,
        false,
    );
    let artifact_cache = ArtifactCache::new(cache_store.as_ref(), &cache_layout);
    let module = destack_source::ModuleId::EPHEMERAL;
    let header = ArtifactImageHeader::new(ArtifactImageKey::Ast { module }, 0);
    let payload = Ast::new(module);
    let image = ArtifactImage::new(header, payload)
        .unwrap_or_else(|error| panic!("failed to build ast image: {error}"));

    // write the image first
    artifact_cache
        .save(&image)
        .unwrap_or_else(|error| panic!("failed to save ast image: {error}"));

    // load the same image back
    let loaded = artifact_cache
        .load::<Ast>(&image.header.artifact_image_key)
        .unwrap_or_else(|error| panic!("failed to load ast image: {error}"))
        .unwrap_or_else(|| panic!("expected stored ast image"));

    // compare the full image contract
    assert_eq!(loaded.header, image.header);
    assert_eq!(loaded.payload.id, image.payload.id);
}

/// Reuse one persisted AST image across a fresh compiler session.
#[test]
fn test_compiler_reuses_ast_image_across_sessions() {
    let root = TemporaryPhysicalFileSystem::new_with_prefix("ast_image");
    let (session, program, compiler, module_path) = build_disk_cache_compiler(&root);

    // build and persist the ast in the first session
    let module_id = compiler
        .resolve_path_to_module(program.current_revision(), &module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    run_to_completion(
        &compiler,
        program.current_revision(),
        |compiler, context| compiler.process_ast(module_id, context),
    )
    .unwrap_or_else(|error| panic!("failed to build ast: {error:?}"));
    let expected = compiler
        .repository
        .ast(program.current_revision(), module_id)
        .unwrap_or_else(|| panic!("expected published ast"))
        .as_ref()
        .clone();

    drop(compiler);
    drop(program);
    drop(session);

    let (_session, program, compiler, module_path) = build_disk_cache_compiler(&root);

    // build the same module in a fresh session
    let module_id = compiler
        .resolve_path_to_module(program.current_revision(), &module_path)
        .unwrap_or_else(|error| {
            panic!("failed to resolve main module in fresh session: {error:?}")
        });
    let file_id = program.module_descriptor(module_id).file_id;

    // refresh the source file through the repository revision path
    let file = program.refresh_source_file_from_file_system(file_id);

    // verify the persisted image is available before any rebuild
    let loaded = compiler
        .load_ast_image(
            program.current_revision(),
            module_id,
            compiler.artifact_stamp_for_revision(
                program.current_revision(),
                &ArtifactKey::ast(module_id),
            ),
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
    run_to_completion(
        &compiler,
        program.current_revision(),
        |compiler, context| compiler.process_ast(module_id, context),
    )
    .unwrap_or_else(|error| panic!("failed to load ast: {error:?}"));
    let resolved = compiler
        .repository
        .ast(program.current_revision(), module_id)
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
        .resolve_path_to_module(program.current_revision(), &module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    run_to_completion(
        &compiler,
        program.current_revision(),
        |compiler, context| compiler.process_ast(module_id, context),
    )
    .unwrap_or_else(|error| panic!("failed to build ast: {error:?}"));
    let expected = compiler
        .repository
        .ast(program.current_revision(), module_id)
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
        .resolve_path_to_module(program.current_revision(), &module_path)
        .unwrap_or_else(|error| {
            panic!("failed to resolve main module in fresh session: {error:?}")
        });
    let file_id = program.module_descriptor(module_id).file_id;

    // refresh the changed source file before the direct image read
    let file = program.refresh_source_file_from_file_system(file_id);

    // reject the persisted ast image for the changed file
    let loaded = compiler
        .load_ast_image(
            program.current_revision(),
            module_id,
            compiler.artifact_stamp_for_revision(
                program.current_revision(),
                &ArtifactKey::ast(module_id),
            ),
            file.as_ref(),
            Some(destack_source::LanguageType::TypeScript),
        )
        .unwrap_or_else(|error| panic!("failed to load persisted ast image: {error}"));
    assert!(
        loaded.is_none(),
        "expected changed source to invalidate ast image"
    );

    // rebuild and confirm the ast really changed
    run_to_completion(
        &compiler,
        program.current_revision(),
        |compiler, context| compiler.process_ast(module_id, context),
    )
    .unwrap_or_else(|error| panic!("failed to rebuild ast: {error:?}"));
    let rebuilt = compiler
        .repository
        .ast(program.current_revision(), module_id)
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
        Some(PersistedImageValidation::SelfContained)
    );
    assert_eq!(
        ArtifactFamily::IntrinsicEnvironment.persisted_image_validation(),
        Some(PersistedImageValidation::SelfContained)
    );
    assert_eq!(
        ArtifactFamily::LibraryEnvironment.persisted_image_validation(),
        Some(PersistedImageValidation::SelfContained)
    );
    assert_eq!(
        ArtifactFamily::Ast.persisted_image_validation(),
        Some(PersistedImageValidation::SelfContained)
    );
    assert_eq!(
        ArtifactFamily::DirBase.persisted_image_validation(),
        Some(PersistedImageValidation::DependencyValidated)
    );

    // other self-contained families
    assert_eq!(
        ArtifactFamily::ModuleGraph.persisted_image_validation(),
        Some(PersistedImageValidation::SelfContained)
    );

    // dependency-validated dir families
    assert_eq!(
        ArtifactFamily::DirPrepared.persisted_image_validation(),
        Some(PersistedImageValidation::DependencyValidated)
    );
    assert_eq!(
        ArtifactFamily::DirResolved.persisted_image_validation(),
        Some(PersistedImageValidation::DependencyValidated)
    );
    assert_eq!(
        ArtifactFamily::DirDeclared.persisted_image_validation(),
        Some(PersistedImageValidation::DependencyValidated)
    );
    assert_eq!(
        ArtifactFamily::DirInterface.persisted_image_validation(),
        Some(PersistedImageValidation::DependencyValidated)
    );
    assert_eq!(
        ArtifactFamily::DirAnalyzed.persisted_image_validation(),
        Some(PersistedImageValidation::DependencyValidated)
    );
    assert_eq!(
        ArtifactFamily::DirElaborated.persisted_image_validation(),
        Some(PersistedImageValidation::DependencyValidated)
    );
    assert_eq!(
        ArtifactFamily::DirPatched.persisted_image_validation(),
        Some(PersistedImageValidation::DependencyValidated)
    );

    // dependency-validated families
    assert_eq!(
        ArtifactFamily::MirBase.persisted_image_validation(),
        Some(PersistedImageValidation::DependencyValidated)
    );
    assert_eq!(
        ArtifactFamily::MirOptimized.persisted_image_validation(),
        Some(PersistedImageValidation::DependencyValidated)
    );
    assert_eq!(
        ArtifactFamily::ModuleOutput.persisted_image_validation(),
        Some(PersistedImageValidation::DependencyValidated)
    );
    assert_eq!(
        ArtifactFamily::PackageOutput.persisted_image_validation(),
        Some(PersistedImageValidation::DependencyValidated)
    );
}

/// Skip artifact image loads when persistent cache is disabled.
#[test]
fn test_compiler_skips_artifact_image_loads_when_disk_cache_is_disabled() {
    let root = TemporaryPhysicalFileSystem::new_with_prefix("artifact_image_load_off");
    let (_session, program, compiler, module_path) = build_memory_cache_compiler(&root);
    let module_id = compiler
        .resolve_path_to_module(program.current_revision(), &module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let artifact_key = destack_artifact::ArtifactKey::ast(module_id);

    // the image loader closure should not run at all
    let loaded = compiler.load_artifact::<(), _>(program.current_revision(), &artifact_key, |_| {
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
    let (_session, program, compiler, module_path) = build_memory_cache_compiler(&root);
    let module_id = compiler
        .resolve_path_to_module(program.current_revision(), &module_path)
        .unwrap_or_else(|error| panic!("failed to resolve main module: {error:?}"));
    let artifact_key = destack_artifact::ArtifactKey::ast(module_id);

    // the image store closure should not run at all
    run_to_completion(&compiler, program.current_revision(), |_, _context| {
        _context.store_artifact(&artifact_key, &(), |_, _, _| {
            panic!("artifact image store should be skipped when disk cache is disabled")
        });

        Ok::<_, crate::ResolveError>(())
    })
    .unwrap_or_else(|_: crate::ResolveError| {
        panic!("artifact image store should not yield diagnostics")
    });
}
