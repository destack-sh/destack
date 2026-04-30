use std::collections::HashSet;

use destack_artifact::{Platform, ProfileKey, Runtime};

use crate::{
    CompilerOptions, HostEnvironment, ProfileEnvironment, ProfileOptions, Target, TsConfigOptions,
    builtin_libs_for_type_entries, profile_flags_for_compiler_options, typescript_default_libs,
};

/// Collect type libraries for one target profile.
fn collect_types_for_target(
    target: &Target,
    compiler_options: &CompilerOptions,
    profile_config: Option<&ProfileOptions>,
) -> Vec<String> {
    let mut type_entries = Vec::new();

    // compiler type entries
    if !compiler_options.types.is_empty() {
        type_entries.extend(compiler_options.types.clone());
    }

    // target type entries
    if let Some(target_types) = &target.types {
        type_entries.extend(target_types.clone());
    }

    // profile type entries
    if let Some(profile_types) = profile_config.and_then(|profile| profile.types.as_ref()) {
        type_entries.extend(profile_types.clone());
    }

    builtin_libs_for_type_entries(&type_entries)
}

/// Return the effective builtin library set for one target profile.
fn effective_libs_for_target_profile(
    target: &Target,
    compiler_options: &CompilerOptions,
    profile_config: Option<&ProfileOptions>,
    tsconfig_options: Option<&TsConfigOptions>,
    runtime: Runtime,
    runtime_version: Option<String>,
    platform: Platform,
) -> Vec<String> {
    let derived_target = Target {
        runtime,
        runtime_version,
        platform,
        ..Target::default()
    };

    // explicit or derived base libraries
    let base_libs =
        if let Some(profile_lib) = profile_config.and_then(|profile| profile.lib.as_ref()) {
            profile_lib.clone()
        } else if let Some(target_lib) = target.lib.as_ref() {
            target_lib.clone()
        } else if !compiler_options.lib.is_empty() {
            compiler_options.lib.clone()
        } else if let Some(tsconfig_options) = tsconfig_options {
            typescript_default_libs(tsconfig_options)
        } else {
            derived_target.derived_lib()
        };

    let mut libs = base_libs;
    let mut seen = HashSet::new();

    // type library expansion
    for lib_name in &libs {
        seen.insert(lib_name.clone());
    }

    for type_lib in collect_types_for_target(target, compiler_options, profile_config) {
        if seen.insert(type_lib.clone()) {
            libs.push(type_lib);
        }
    }

    // native library boundary
    if runtime.is_native() {
        libs.retain(|lib| {
            let Some(builtin) = destack_builtin::builtin_library(lib) else {
                return true;
            };

            matches!(builtin.kind, destack_builtin::BuiltinLibraryKind::Language)
                || matches!(builtin.name, "native" | "platform" | "destack")
        });

        if !libs.iter().any(|lib| lib == "native") {
            libs.push("native".to_string());
        }
    }

    libs
}

/// Build one profile key for one target.
pub(crate) fn profile_key_for_target(
    target: &Target,
    compiler_options: &CompilerOptions,
    profile_config: Option<&ProfileOptions>,
    tsconfig_options: Option<&TsConfigOptions>,
    environment: &HostEnvironment,
) -> ProfileKey {
    let compiler_options = target.compiler_options(compiler_options);
    let emit = target.emit;

    // runtime surface
    let runtime = profile_config
        .and_then(|profile| profile.runtime.as_ref().map(|runtime| runtime.host))
        .unwrap_or(target.runtime);
    let runtime_version = profile_config
        .and_then(|profile| {
            profile
                .runtime
                .as_ref()
                .and_then(|runtime| runtime.version.clone())
        })
        .or_else(|| target.runtime_version.clone());
    let platform = profile_config
        .and_then(|profile| profile.platform)
        .unwrap_or(target.platform);

    // profile mode
    let debug = profile_config
        .and_then(|profile| profile.debug)
        .unwrap_or(target.debug);

    let libs = effective_libs_for_target_profile(
        target,
        &compiler_options,
        profile_config,
        tsconfig_options,
        runtime,
        runtime_version,
        platform,
    );

    // comptime environment
    let env = profile_config
        .and_then(|profile| profile.comptime_env.as_ref())
        .or(compiler_options.comptime_env.as_ref())
        .map(|keys| environment.key_whitelist(keys))
        .unwrap_or_else(|| environment.key_all());

    let flags = profile_flags_for_compiler_options(&compiler_options);
    let (_, _, _, test) = ProfileEnvironment::mode_from_key(&env, environment, debug);
    let globals = target
        .globals
        .iter()
        .map(|path| path.display().to_string())
        .collect();

    ProfileKey::new(
        emit,
        runtime,
        platform,
        target.target_arch.clone(),
        target.target_vendor.clone(),
        target.target_abi.clone(),
        libs,
        globals,
        debug,
        test,
        compiler_options.skip_lib_check,
        env,
        flags,
    )
}
