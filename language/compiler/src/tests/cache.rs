use std::time::{SystemTime, UNIX_EPOCH};

use destack_source::{FileVersion, ModuleId, ModuleVersion, PackageId, ProfileId, ProfileVersion};
use destack_workspace::{CacheMode, CachePolicy, CacheScope, CacheValidate, ModuleAst};

use crate::{CacheContext, CacheOptions, CacheRegistry};

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
