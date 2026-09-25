use std::collections::BTreeMap;

use indexmap::{IndexMap, IndexSet};
use tspp_artifact::{Host, Platform, ProfileKey, Runtime, Stability};

use crate::{
    CompilerOptions, Condition, ConditionAxis, ConditionCatalog, ConditionSelection, ConditionSet,
    Environment, Manifest, ManifestFile, Product, ProfileOptions, RepositoryError, Target,
};

/// The builtin prelude global grounding every profile.
const PRELUDE_GLOBAL: &str = "tspp:prelude";

/// Build one profile key for one target.
pub(crate) fn profile_key_for_target(
    target_name: &str,
    target: &Target,
    compiler_options: &CompilerOptions,
    profile_config: Option<&ProfileOptions>,
    config: Option<&ManifestFile>,
    environment: &Environment,
    product: Option<&str>,
    product_config: Option<&Product>,
    product_role: Option<&str>,
) -> Result<ProfileKey, RepositoryError> {
    // resolve runtime axes
    let runtime = target.runtime();
    let platform = profile_config
        .and_then(|profile| profile.platform)
        .unwrap_or(target.platform);
    let host = profile_config
        .and_then(|profile| profile.host)
        .unwrap_or(target.host);

    // resolve compiler options and conditions
    let compiler_options = profile_compiler_options_for_target(
        target,
        compiler_options,
        profile_config,
        product_config,
        product_role,
    );
    let conditions = build_profile_conditions(
        target_name,
        &compiler_options,
        config,
        &environment.selection,
        product,
        product_role,
        platform,
        host,
        runtime,
    )?;
    let output = target.output;

    // const evaluation environment
    let env = profile_config
        .and_then(|profile| profile.const_env.as_ref())
        .or(compiler_options.const_env.as_ref())
        .map(|keys| environment.key_whitelist(keys))
        .unwrap_or_else(|| environment.key_all());

    // compiler identity
    let restrictions = &compiler_options.restrictions;
    let no_managed = restrictions.no_managed;
    let no_heap = restrictions.no_heap;
    let no_runtime = restrictions.no_runtime;
    let no_dynamic_dispatch = restrictions.no_dynamic_dispatch;
    let no_unsafe = restrictions.no_unsafe;
    let no_reflection = restrictions.no_reflection;
    let no_unwind = restrictions.no_unwind;
    let no_aliasing_mutable_borrows = restrictions.no_aliasing_mutable_borrows;
    let no_implicit_receivers = restrictions.no_implicit_receivers;
    let globals = normalize_profile_names(profile_globals(&compiler_options));
    let tree = compiler_options.tree.clone();
    let derive = normalize_profile_names(
        compiler_options
            .derive
            .iter()
            .map(|derive| derive.key().to_string())
            .collect(),
    );

    Ok(ProfileKey {
        output,
        stability: resolved_stability(config.map(|config| &config.manifest), product_config),
        conditions,
        architecture: target.architecture.clone(),
        vendor: target.vendor.clone(),
        abi: target.abi.clone(),
        globals,
        tree,
        derive,
        env,
        no_managed,
        no_heap,
        no_runtime,
        no_dynamic_dispatch,
        no_unsafe,
        no_reflection,
        no_unwind,
        no_aliasing_mutable_borrows,
        no_implicit_receivers,
    })
}

/// Build the active conditions for one compiler profile.
fn build_profile_conditions(
    target_name: &str,
    compiler_options: &CompilerOptions,
    config: Option<&ManifestFile>,
    selection: &ConditionSelection,
    product: Option<&str>,
    product_role: Option<&str>,
    platform: Platform,
    host: Host,
    runtime: Runtime,
) -> Result<ConditionSet, RepositoryError> {
    let modes = expand_conditions(
        config,
        ConditionAxis::Mode,
        &compiler_options.modes,
        &selection.modes,
    )?;
    let roles = expand_conditions(
        config,
        ConditionAxis::Role,
        &compiler_options.roles,
        &selection.roles,
    )?;
    let features = expand_conditions(
        config,
        ConditionAxis::Feature,
        &compiler_options.features,
        &selection.features,
    )?;
    let tags = expand_conditions(
        config,
        ConditionAxis::Tag,
        &compiler_options.tags,
        &selection.tags,
    )?;
    let labels = if let Some(config) = config {
        condition_labels(&config.conditions, &modes, &roles, &features, &tags)
    } else {
        BTreeMap::new()
    };
    Ok(ConditionSet {
        modes,
        roles,
        features,
        tags,
        target: Some(target_name.to_string()),
        product: product.map(str::to_string),
        role: product_role.map(str::to_string),
        labels,
        platform,
        host,
        runtime,
    })
}

/// Build one profile's global modules.
fn profile_globals(compiler_options: &CompilerOptions) -> Vec<String> {
    let mut globals = vec![PRELUDE_GLOBAL.to_string()];
    globals.extend(
        compiler_options
            .globals
            .iter()
            .map(|path| path.display().to_string()),
    );

    globals
}

/// Normalize profile key names.
fn normalize_profile_names(mut names: Vec<String>) -> Vec<String> {
    names.sort();
    names.dedup();

    names
}

/// Build compiler options after profile, product, and target modifiers.
fn profile_compiler_options_for_target(
    target: &Target,
    compiler_options: &CompilerOptions,
    profile_config: Option<&ProfileOptions>,
    product_config: Option<&Product>,
    product_role: Option<&str>,
) -> CompilerOptions {
    let mut compiler_options = compiler_options.clone();

    // profile compiler overrides
    if let Some(profile_config) = profile_config {
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
        compiler_options
            .restrictions
            .tighten_with(&profile_config.restrictions);
    }
    if let Some(product_config) = product_config {
        compiler_options.modes.extend(product_config.modes.clone());
        compiler_options.roles.extend(product_config.roles.clone());
        compiler_options
            .features
            .extend(product_config.features.clone());
        compiler_options.tags.extend(product_config.tags.clone());
    }
    if let Some(product_role) = product_role {
        compiler_options.roles.push(product_role.to_string());
    }

    target.compiler_options(&compiler_options)
}

/// Resolve the stability promised by the selected package or product.
fn resolved_stability(
    config: Option<&Manifest>,
    product_config: Option<&Product>,
) -> Option<Stability> {
    product_config
        .and_then(|product| product.stability)
        .or_else(|| config.and_then(|config| config.stability))
}

/// Expand selected source graph names through declared parents.
fn expand_conditions(
    config: Option<&ManifestFile>,
    axis: ConditionAxis,
    declared: &[String],
    environment: &[String],
) -> Result<IndexSet<String>, RepositoryError> {
    let selected = declared.iter().chain(environment).map(String::as_str);
    let Some(config) = config else {
        return Ok(selected.map(str::to_string).collect());
    };

    config
        .conditions
        .expand(axis, selected)
        .map_err(|error| RepositoryError::InvalidConfig {
            file: config.file_id,
            message: error.to_string(),
        })
}

/// Collect labels from every active source graph condition.
fn condition_labels(
    conditions: &ConditionCatalog,
    modes: &IndexSet<String>,
    roles: &IndexSet<String>,
    features: &IndexSet<String>,
    tags: &IndexSet<String>,
) -> BTreeMap<String, Vec<String>> {
    let mut labels = BTreeMap::<String, IndexSet<String>>::new();

    // collect every label contributed by each active condition axis
    extend_condition_labels(modes, &conditions.modes, &mut labels);
    extend_condition_labels(roles, &conditions.roles, &mut labels);
    extend_condition_labels(features, &conditions.features, &mut labels);
    extend_condition_labels(tags, &conditions.tags, &mut labels);

    labels
        .into_iter()
        .map(|(name, values)| (name, values.into_iter().collect()))
        .collect()
}

/// Extend collected labels with one active condition axis.
fn extend_condition_labels(
    names: &IndexSet<String>,
    declarations: &IndexMap<String, Condition>,
    labels: &mut BTreeMap<String, IndexSet<String>>,
) {
    for name in names {
        let Some(condition) = declarations.get(name) else {
            continue;
        };

        // merge each label's values in active condition order
        for (label, value) in &condition.labels {
            labels
                .entry(label.clone())
                .or_default()
                .insert(value.clone());
        }
    }
}
