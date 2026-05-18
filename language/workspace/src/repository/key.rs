use destack_artifact::ProfileKey;

use crate::{
    CompilerOptions, ConditionSelection, ConditionSet, Destack, Environment, ProfileOptions,
    Target, profile_flags_for_compiler_options,
};

/// Build one profile key for one target.
pub(crate) fn profile_key_for_target(
    target_name: &str,
    target: &Target,
    compiler_options: &CompilerOptions,
    profile_config: Option<&ProfileOptions>,
    config: Option<&Destack>,
    environment: &Environment,
    product: Option<&str>,
    product_role: Option<&str>,
) -> ProfileKey {
    let compiler_options =
        profile_compiler_options_for_target(target, compiler_options, profile_config, product_role);
    let conditions = condition_set_from_compiler_options(
        target_name,
        target,
        &compiler_options,
        config,
        &environment.selection,
        product,
    );
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
        conditions.modes.iter().cloned().collect(),
        conditions.roles.iter().cloned().collect(),
        conditions.features.iter().cloned().collect(),
        conditions.tags.iter().cloned().collect(),
        conditions.target.clone(),
        conditions.product.clone(),
        env,
        flags,
    )
}

/// Build one condition set from already resolved compiler options.
fn condition_set_from_compiler_options(
    target_name: &str,
    target: &Target,
    compiler_options: &CompilerOptions,
    config: Option<&Destack>,
    selection: &ConditionSelection,
    product: Option<&str>,
) -> ConditionSet {
    ConditionSet {
        modes: inherited_conditions(
            &compiler_options.modes,
            &selection.modes,
            |config, name| {
                config
                    .conditions
                    .modes
                    .get(name)
                    .map(|options| options.extends.as_slice())
            },
            config,
        ),
        roles: inherited_conditions(
            &compiler_options.roles,
            &selection.roles,
            |config, name| {
                config
                    .conditions
                    .roles
                    .get(name)
                    .map(|options| options.extends.as_slice())
            },
            config,
        ),
        features: inherited_conditions(
            &compiler_options.features,
            &selection.features,
            |config, name| {
                config
                    .conditions
                    .features
                    .get(name)
                    .map(|options| options.extends.as_slice())
            },
            config,
        ),
        tags: inherited_conditions(
            &compiler_options.tags,
            &selection.tags,
            |config, name| {
                config
                    .conditions
                    .tags
                    .get(name)
                    .map(|options| options.extends.as_slice())
            },
            config,
        ),
        target: Some(target_name.to_string()),
        product: product.map(str::to_string),
        platform: Some(target.platform),
        host: Some(target.host),
        runtime: Some(target.runtime),
    }
}

/// Build compiler options after profile, product, and target modifiers.
fn profile_compiler_options_for_target(
    target: &Target,
    compiler_options: &CompilerOptions,
    profile_config: Option<&ProfileOptions>,
    product_role: Option<&str>,
) -> CompilerOptions {
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

    target.compiler_options(&compiler_options)
}

/// Expand selected source graph names through declared parents.
fn inherited_conditions(
    declared: &[String],
    environment: &[String],
    parents_for: impl for<'a> Fn(&'a Destack, &str) -> Option<&'a [String]>,
    config: Option<&Destack>,
) -> indexmap::IndexSet<String> {
    let mut conditions = indexmap::IndexSet::new();

    for name in declared.iter().chain(environment) {
        // declared parent conditions
        if let Some(parents) = config.and_then(|config| parents_for(config, name)) {
            conditions.extend(parents.iter().cloned());
        }

        // selected condition
        conditions.insert(name.clone());
    }

    conditions
}
