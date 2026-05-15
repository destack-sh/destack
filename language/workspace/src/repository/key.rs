use destack_artifact::ProfileKey;

use crate::{
    CompilerOptions, DestackConfig, HostEnvironment, ProfileOptions, Target,
    profile_flags_for_compiler_options,
};

/// Build one profile key for one target.
pub(crate) fn profile_key_for_target(
    target: &Target,
    compiler_options: &CompilerOptions,
    profile_config: Option<&ProfileOptions>,
    config: Option<&DestackConfig>,
    environment: &HostEnvironment,
    product: Option<&str>,
    product_role: Option<&str>,
) -> ProfileKey {
    let mut compiler_options = compiler_options.clone();

    // profile compiler overrides
    if let Some(profile_config) = profile_config {
        if profile_config.tree.is_some() {
            compiler_options.tree = profile_config.tree.clone();
        }
        compiler_options
            .globals
            .extend(profile_config.globals.clone());
        compiler_options
            .derive
            .extend(profile_config.derive.clone());
        compiler_options.modes.extend(profile_config.modes.clone());
        compiler_options.roles.extend(profile_config.roles.clone());
        compiler_options
            .features
            .extend(profile_config.features.clone());
        compiler_options.tags.extend(profile_config.tags.clone());
    }
    if let Some(product_role) = product_role {
        compiler_options.roles.push(product_role.to_string());
    }
    let compiler_options = target.compiler_options(&compiler_options);
    let emit = target.emit;

    // runtime surface
    let runtime = profile_config
        .and_then(|profile| profile.runtime.as_ref().map(|runtime| runtime.runtime))
        .unwrap_or(target.runtime);
    let platform = profile_config
        .and_then(|profile| profile.platform)
        .unwrap_or(target.platform);
    let host = profile_config
        .and_then(|profile| profile.host)
        .unwrap_or(target.host);

    // comptime environment
    let env = profile_config
        .and_then(|profile| profile.comptime_env.as_ref())
        .or(compiler_options.comptime_env.as_ref())
        .map(|keys| environment.key_whitelist(keys))
        .unwrap_or_else(|| environment.key_all());

    let mut modes = Vec::new();
    for mode in &compiler_options.modes {
        // declared parent modes
        if let Some(options) = config.and_then(|config| config.modes.get(mode)) {
            for parent in &options.extends {
                if !modes.contains(parent) {
                    modes.push(parent.clone());
                }
            }
        }

        // selected mode
        if !modes.contains(mode) {
            modes.push(mode.clone());
        }
    }
    let mut roles = Vec::new();
    for role in &compiler_options.roles {
        // declared parent roles
        if let Some(options) = config.and_then(|config| config.roles.get(role)) {
            for parent in &options.extends {
                if !roles.contains(parent) {
                    roles.push(parent.clone());
                }
            }
        }

        // selected role
        if !roles.contains(role) {
            roles.push(role.clone());
        }
    }
    let mut features = Vec::new();
    for feature in &compiler_options.features {
        // declared parent features
        if let Some(options) = config.and_then(|config| config.features.get(feature)) {
            for parent in &options.extends {
                if !features.contains(parent) {
                    features.push(parent.clone());
                }
            }
        }

        // selected feature
        if !features.contains(feature) {
            features.push(feature.clone());
        }
    }
    let mut tags = Vec::new();
    for tag in &compiler_options.tags {
        // declared parent tags
        if let Some(options) = config.and_then(|config| config.tags.get(tag)) {
            for parent in &options.extends {
                if !tags.contains(parent) {
                    tags.push(parent.clone());
                }
            }
        }

        // selected tag
        if !tags.contains(tag) {
            tags.push(tag.clone());
        }
    }

    // compiler flags
    let flags = profile_flags_for_compiler_options(&compiler_options);
    let globals = compiler_options
        .globals
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    let tree = compiler_options.tree.clone();
    let derive = compiler_options.derive.clone();

    ProfileKey::new(
        emit,
        runtime,
        platform,
        host,
        target.target_arch.clone(),
        target.target_vendor.clone(),
        target.target_abi.clone(),
        globals,
        tree,
        derive,
        modes,
        roles,
        features,
        tags,
        product.map(str::to_string),
        env,
        flags,
    )
}
