use destack_artifact::ProfileKey;

use crate::{
    CompilerOptions, HostEnvironment, ProfileEnvironment, ProfileOptions, Target,
    profile_flags_for_compiler_options,
};

/// Build one profile key for one target.
pub(crate) fn profile_key_for_target(
    target: &Target,
    compiler_options: &CompilerOptions,
    profile_config: Option<&ProfileOptions>,
    environment: &HostEnvironment,
) -> ProfileKey {
    let mut compiler_options = compiler_options.clone();
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
    }
    let compiler_options = target.compiler_options(&compiler_options);
    let emit = target.emit;

    // runtime surface
    let runtime = profile_config
        .and_then(|profile| profile.runtime.as_ref().map(|runtime| runtime.host))
        .unwrap_or(target.runtime);
    let platform = profile_config
        .and_then(|profile| profile.platform)
        .unwrap_or(target.platform);

    // profile mode
    let debug = profile_config
        .and_then(|profile| profile.debug)
        .unwrap_or(target.debug);

    // comptime environment
    let env = profile_config
        .and_then(|profile| profile.comptime_env.as_ref())
        .or(compiler_options.comptime_env.as_ref())
        .map(|keys| environment.key_whitelist(keys))
        .unwrap_or_else(|| environment.key_all());

    let flags = profile_flags_for_compiler_options(&compiler_options);
    let (_, _, _, test) = ProfileEnvironment::mode_from_key(&env, environment, debug);
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
        target.target_arch.clone(),
        target.target_vendor.clone(),
        target.target_abi.clone(),
        globals,
        tree,
        derive,
        debug,
        test,
        env,
        flags,
    )
}
