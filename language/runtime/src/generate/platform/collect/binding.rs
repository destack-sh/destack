use std::collections::{BTreeSet, HashSet};
use std::str::FromStr;

use destack_artifact::{Host, Platform};
use destack_compiler::Compiler;
use destack_core::StringPool;
use destack_dir::{
    self as dir, Annotation, Argument, Declaration, Expression, GlobalSymbolId, LanguageItem,
    ScalarLiteral,
};
use destack_source::ModuleId;
use destack_workspace::ProfileId;

use super::domain::qualify_platform_implementation_name;
use crate::context::GeneratorContext;
use crate::platform::model::{
    BindingCatalog, BindingEntry, BindingParameter, BindingReturn, BindingTypeContext,
    CatalogBindingAffinity, CatalogBindingProvider, CatalogBindingReplayKind,
    CatalogBindingSimulation, CatalogEffect, CatalogEntropyKind, CatalogReplayPayload,
    CatalogReplayPolicy, binding_type_symbols, collect_binding_params, collect_binding_return,
    format_declared_signature,
};

/// Binding metadata extracted from a declaration node.
#[derive(Debug, Clone)]
struct BindingRecord {
    /// Declaration name used by runtime implementation functions.
    implementation_name: String,
    /// Declaration documentation extracted from library sources.
    documentation: Option<String>,
    /// Fully qualified extern binding name.
    extern_name: String,
    /// Canonical signature string for stability checks.
    signature: String,
    /// Parameter metadata payload.
    params: Vec<BindingParameter>,
    /// Return binding type for generated wrappers.
    return_binding: BindingReturn,
    /// Effect kind for replay and policy.
    effect: CatalogEffect,
    /// Replay routing for the binding.
    replay_kind: CatalogBindingReplayKind,
    /// Replay payload policy for recorded bindings.
    replay_payload: CatalogReplayPayload,
    /// Required host actions for this binding.
    requires: Vec<String>,
    /// Platforms where this binding is supported.
    platforms: Vec<String>,
    /// Hosts where this binding is supported.
    hosts: Vec<String>,
    /// Provider that implements this binding.
    provider: CatalogBindingProvider,
    /// Execution context required by this binding.
    affinity: CatalogBindingAffinity,
    /// Simulation support for this binding.
    simulation: CatalogBindingSimulation,
}

/// Binding decorator payload extracted from an annotation.
#[derive(Debug, Clone)]
struct BindingDecorator {
    /// Optional binding name override.
    extern_name: Option<String>,
    /// Optional effect class override.
    effect: CatalogEffect,
    /// Optional replay payload override.
    replay_payload: CatalogReplayPayload,
    /// Required host actions for this binding.
    requires: Vec<String>,
    /// Platforms where this binding is supported.
    platforms: Vec<String>,
    /// Hosts where this binding is supported.
    hosts: Vec<String>,
    /// Provider that implements this binding.
    provider: CatalogBindingProvider,
    /// Execution context required by this binding.
    affinity: CatalogBindingAffinity,
    /// Simulation support for this binding.
    simulation: CatalogBindingSimulation,
}

/// Resolve replay routing for a binding name.
fn binding_replay_kind_for_name(name: &str) -> CatalogBindingReplayKind {
    match name {
        "destack.time.clock.wallNs" => {
            CatalogBindingReplayKind::Entropy(CatalogEntropyKind::TimeReadWall)
        }
        "destack.time.clock.monoNs" => {
            CatalogBindingReplayKind::Entropy(CatalogEntropyKind::TimeReadMonotonic)
        }
        "destack.random.stream.create" | "destack.random.stream.in" => {
            CatalogBindingReplayKind::Entropy(CatalogEntropyKind::RandomStreamCreate)
        }
        "destack.random.stream.nextU64" | "destack.random.stream.nextU64From" => {
            CatalogBindingReplayKind::Entropy(CatalogEntropyKind::RandomReadU64)
        }
        "destack.random.stream.fillBytes"
        | "destack.random.stream.fillBytesFrom"
        | "destack.random.secure.bytes"
        | "destack.random.secure.bytesTry" => {
            CatalogBindingReplayKind::Entropy(CatalogEntropyKind::RandomReadBytes)
        }
        _ => CatalogBindingReplayKind::BindingCall,
    }
}

/// Collect platform bindings from library modules.
pub(crate) fn collect_platform_bindings(
    compiler: &Compiler,
    context: &GeneratorContext,
    strings: &StringPool,
    profile_id: ProfileId,
    platform_modules: &[ModuleId],
) -> BindingCatalog {
    // collect binding type symbols
    let binding_symbols = binding_type_symbols(context, profile_id);
    let binding_decorator_symbol = context
        .language_environment(profile_id)
        .item(LanguageItem::Binding)
        .unwrap_or_else(|| panic!("missing Binding symbol for profile {profile_id:?}"));

    // collect bindings by domain
    let mut domains: BindingCatalog = BindingCatalog::default();

    // visit library modules and extract binding annotations
    for module_id in platform_modules {
        // load module metadata
        let module = context.get(*module_id);
        let module = module.as_ref();
        let dir = context.dir(module.id, profile_id);
        let tree = dir.tree();
        let types = dir.types();
        let symbols = dir.symbols();
        let mut seen_declarations = HashSet::new();

        // scan expressions for binding declarations
        for (expression_id, expression) in tree.iter_nodes_of_type::<Expression>() {
            // resolve the declaration referenced by the expression
            let declaration_id = declaration_from_expression(&tree, expression_id, expression);
            let Some(declaration_id) = declaration_id else {
                continue;
            };
            if !seen_declarations.insert(declaration_id.id) {
                continue;
            }

            // filter to function declarations
            let declaration = tree.get::<Declaration>(declaration_id);
            let Declaration::Function { signature, .. } = declaration else {
                continue;
            };

            // resolve declaration documentation comments
            let documentation =
                binding_documentation(strings, &tree, expression_id, declaration_id);

            let symbol = symbols.get_symbol(declaration.symbol());
            if symbol.decorators.binding.is_none() {
                continue;
            }

            // resolve the extern binding name and payload
            let binding = binding_decorator_value(
                compiler,
                context,
                profile_id,
                binding_decorator_symbol,
                &tree,
                expression_id.into_any(),
                strings,
            )
            .or_else(|| {
                binding_decorator_value(
                    compiler,
                    context,
                    profile_id,
                    binding_decorator_symbol,
                    &tree,
                    declaration_id.into_any(),
                    strings,
                )
            })
            .unwrap_or_else(|| panic!("binding-decorated symbol is missing @binding payload"));
            let implementation_name = symbol.name().map(|name| strings.get(name).to_string());
            let implementation_name = implementation_name.map(|implementation_name| {
                qualify_platform_implementation_name(
                    context.repository(),
                    module,
                    &implementation_name,
                )
            });
            let extern_name = binding
                .extern_name
                .or_else(|| symbol.name().map(|name| strings.get(name).to_string()));

            // format the canonical signature for the declaration
            let signature_text = format_declared_signature(
                context,
                compiler,
                declaration_id,
                declaration,
                module,
                strings,
                profile_id,
            );

            // collect parameter and return metadata from the AST
            let domain = extern_name
                .as_deref()
                .map(binding_domain)
                .unwrap_or_else(|| "global".to_string());

            // binding analysis context
            let binding_context = BindingTypeContext::new(
                compiler,
                context,
                tree,
                types,
                symbols,
                context,
                strings,
                profile_id,
                &binding_symbols,
                &domain,
            );

            let params = collect_binding_params(&binding_context, signature);
            let return_binding =
                collect_binding_return(&binding_context, declaration_id, signature);

            // insert parsed binding metadata into the catalog
            if let Some(entry) = binding_from_node(
                implementation_name,
                documentation,
                extern_name,
                signature_text,
                params,
                return_binding,
                binding.effect,
                binding.replay_payload,
                binding.requires,
                binding.platforms,
                binding.hosts,
                binding.provider,
                binding.affinity,
                binding.simulation,
            ) {
                insert_binding(&mut domains, entry);
            }
        }
    }

    domains
}
/// Return the domain portion of a binding name.
fn binding_domain(extern_name: &str) -> String {
    let mut parts = extern_name.split('.');
    let _prefix = parts.next();
    parts.next().unwrap_or("global").to_string()
}

/// Build a binding record from parsed metadata.
fn binding_from_node(
    implementation_name: Option<String>,
    documentation: Option<String>,
    extern_name: Option<String>,
    signature: String,
    params: Vec<BindingParameter>,
    return_binding: BindingReturn,
    effect: CatalogEffect,
    replay_payload: CatalogReplayPayload,
    requires: Vec<String>,
    platforms: Vec<String>,
    hosts: Vec<String>,
    provider: CatalogBindingProvider,
    affinity: CatalogBindingAffinity,
    simulation: CatalogBindingSimulation,
) -> Option<BindingRecord> {
    let implementation_name = implementation_name?;
    let extern_name = extern_name?;
    if !extern_name.starts_with("destack.") {
        return None;
    }

    Some(BindingRecord {
        implementation_name,
        documentation,
        replay_kind: binding_replay_kind_for_name(&extern_name),
        extern_name,
        signature,
        params,
        return_binding,
        effect,
        replay_payload,
        requires,
        platforms,
        hosts,
        provider,
        affinity,
        simulation,
    })
}

/// Insert a binding record into the domain catalog.
fn insert_binding(domains: &mut BindingCatalog, record: BindingRecord) {
    // resolve the binding domain
    let domain = binding_domain(&record.extern_name);
    let domain_bindings = domains.entry(domain).or_default();

    // build a canonical entry for comparisons
    let entry = BindingEntry {
        implementation_name: record.implementation_name,
        documentation: record.documentation,
        signature: record.signature,
        parameters: record.params,
        return_binding: record.return_binding.binding_type,
        return_is_result: record.return_binding.is_result,
        effect: record.effect,
        replay_kind: record.replay_kind,
        replay_payload: record.replay_payload,
        requires: record.requires,
        platforms: record.platforms,
        hosts: record.hosts,
        provider: record.provider,
        affinity: record.affinity,
        simulation: record.simulation,
    };

    // insert the entry and validate signature stability
    if let Some(existing) = domain_bindings.insert(record.extern_name.clone(), entry.clone())
        && (existing.implementation_name != entry.implementation_name
            || existing.documentation != entry.documentation
            || existing.signature != entry.signature
            || existing.effect != entry.effect
            || existing.replay_kind != entry.replay_kind
            || existing.replay_payload != entry.replay_payload
            || existing.requires != entry.requires
            || existing.platforms != entry.platforms
            || existing.hosts != entry.hosts
            || existing.provider != entry.provider
            || existing.affinity != entry.affinity
            || existing.simulation != entry.simulation)
    {
        panic!(
            "binding signature mismatch for {}: {:?} vs {:?}",
            record.extern_name, existing, entry
        );
    }
}

/// Collect binding documentation from semantic DIR metadata.
fn binding_documentation(
    strings: &StringPool,
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<Expression>,
    declaration_id: dir::LocalNodeId<Declaration>,
) -> Option<String> {
    // prefer docs attached to the declaration node
    let declaration_docs = node_documentation(strings, tree, declaration_id.id);
    if declaration_docs.is_some() {
        return declaration_docs;
    }

    // fall back to docs attached to the declaration expression wrapper
    node_documentation(strings, tree, expression_id.id)
}

/// Collect semantic documentation from one DIR node.
fn node_documentation(strings: &StringPool, tree: &dir::Tree, node_id: u32) -> Option<String> {
    let documentation = tree.get_documentation(node_id)?;

    Some(strings.get(documentation.text).to_string())
}

/// Extract the binding decorator value from a declaration expression.
fn binding_decorator_value(
    compiler: &Compiler,
    context: &GeneratorContext,
    profile_id: ProfileId,
    binding_decorator_symbol: GlobalSymbolId,
    tree: &dir::Tree,
    node_id: dir::LocalNodeIdAny,
    strings: &StringPool,
) -> Option<BindingDecorator> {
    // scan annotations for the binding decorator
    let annotations = tree.get_annotations(node_id.id);
    for annotation_id in annotations {
        let annotation = tree.get::<Annotation>(annotation_id);
        let Annotation::Decorator { expression, .. } = annotation else {
            continue;
        };

        // resolve decorator shape: @binding(...) is represented as a call expression
        let decorator_expression = *expression;
        let expression = tree.get::<Expression>(decorator_expression);
        let (decorator_expression, arguments) = match expression {
            Expression::Call {
                left, arguments, ..
            } => (*left, Some(arguments.as_slice())),
            _ => (decorator_expression, None),
        };

        if !expression_is_binding_decorator(
            compiler,
            context,
            profile_id,
            binding_decorator_symbol,
            tree,
            decorator_expression,
        ) {
            continue;
        }

        // resolve decorator arguments
        return Some(decorator_binding_argument(tree, arguments, strings));
    }
    None
}

/// Return true when one decorator expression names `binding`.
fn expression_is_binding_decorator(
    compiler: &Compiler,
    context: &GeneratorContext,
    profile_id: ProfileId,
    binding_decorator_symbol: GlobalSymbolId,
    tree: &dir::Tree,
    expr_id: dir::LocalNodeId<Expression>,
) -> bool {
    let expression = tree.get::<Expression>(expr_id);
    expression.target_symbol().is_some_and(|target_symbol| {
        compiler.canonical_declared_artifact_symbol_for_revision(
            context.revision(),
            profile_id,
            target_symbol,
        ) == binding_decorator_symbol
    })
}

/// Resolve a declaration node from a binding expression.
fn declaration_from_expression(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<Expression>,
    expression: &Expression,
) -> Option<dir::LocalNodeId<Declaration>> {
    match expression {
        Expression::Declaration { declaration } => Some(*declaration),
        Expression::Labelled { body, .. } => {
            let inner = tree.get::<Expression>(*body);
            declaration_from_expression(tree, *body, inner)
        }
        Expression::Parenthesized { expression } => {
            let inner = tree.get::<Expression>(*expression);
            declaration_from_expression(tree, *expression, inner)
        }
        Expression::Block { .. } => None,
        _ => {
            let _ = expression_id;
            None
        }
    }
}

/// Extract the binding decorator arguments.
fn decorator_binding_argument(
    tree: &dir::Tree,
    arguments: Option<&[dir::LocalNodeId<Argument>]>,
    strings: &StringPool,
) -> BindingDecorator {
    let Some(arguments) = arguments else {
        panic!("@binding requires an options object");
    };

    if arguments.len() < 2 {
        panic!("@binding requires an options object");
    }

    // initialize the decorator payload
    let mut extern_name = None;

    // parse argument values in order
    if let Some(argument_id) = arguments.first() {
        let argument = tree.get::<Argument>(*argument_id);
        let value_id = argument.value();
        extern_name = scalar_string_literal(tree, value_id, strings);
        if let Some(extern_name) = extern_name.as_ref() {
            validate_binding_extern_name(extern_name);
        }
    }

    let argument = tree.get::<Argument>(arguments[1]);
    let value_id = argument.value();
    let spec = parse_binding_spec(tree, value_id, strings);

    // return the payload
    BindingDecorator {
        extern_name,
        effect: spec.effect,
        replay_payload: spec.replay_payload,
        requires: spec.requires,
        platforms: spec.platforms,
        hosts: spec.hosts,
        provider: spec.provider,
        affinity: spec.affinity,
        simulation: spec.simulation,
    }
}

/// Parsed options for bindings.
struct BindingSpec {
    /// Effect kind for the binding.
    effect: CatalogEffect,
    /// Replay payload policy for recorded bindings.
    replay_payload: CatalogReplayPayload,
    /// Required host actions for this binding.
    requires: Vec<String>,
    /// Platforms where this binding is supported.
    platforms: Vec<String>,
    /// Hosts where this binding is supported.
    hosts: Vec<String>,
    /// Provider that implements this binding.
    provider: CatalogBindingProvider,
    /// Execution context required by this binding.
    affinity: CatalogBindingAffinity,
    /// Simulation support for this binding.
    simulation: CatalogBindingSimulation,
}

/// Parse binding options from a binding decorator.
fn parse_binding_spec(
    tree: &dir::Tree,
    value_id: dir::LocalNodeId<Expression>,
    strings: &StringPool,
) -> BindingSpec {
    // require an object literal payload
    let expression = tree.get::<Expression>(value_id);
    let Expression::ObjectExpression { properties } = expression else {
        panic!("@binding options must be an object literal");
    };

    // collect binding properties
    let mut provider = None;
    let mut effect = None;
    let mut replay = None;
    let mut requires = Vec::new();
    let mut platforms = Vec::new();
    let mut families = Vec::new();
    let mut hosts = Vec::new();
    let mut affinity = None;
    let mut simulation = None;

    // read each property value
    for property_id in properties {
        let property = tree.get::<dir::Property>(*property_id);
        let dir::Property::Field { key, value, .. } = property else {
            continue;
        };
        let Some(key) = parse_option_key(tree, key.as_ref(), strings) else {
            continue;
        };
        let Some(value_id) = value else {
            continue;
        };
        match key.as_str() {
            "provider" => {
                let Some(value) = scalar_string_literal(tree, *value_id, strings) else {
                    panic!("@binding provider must be a string literal");
                };
                provider = Some(value);
            }
            "effect" => {
                let Some(value) = scalar_string_literal(tree, *value_id, strings) else {
                    panic!("@binding effect must be a string literal");
                };
                effect = Some(value);
            }
            "replay" => {
                let Some(value) = scalar_string_literal(tree, *value_id, strings) else {
                    panic!("@binding replay must be a string literal");
                };
                replay = Some(value);
            }
            "requires" => {
                requires = parse_required_actions(tree, *value_id, strings);
            }
            "platforms" => {
                platforms = parse_platforms(tree, *value_id, strings);
            }
            "families" => {
                families = parse_platform_families(tree, *value_id, strings);
            }
            "hosts" => {
                hosts = parse_hosts(tree, *value_id, strings);
            }
            "affinity" => {
                let Some(value) = scalar_string_literal(tree, *value_id, strings) else {
                    panic!("@binding affinity must be a string literal");
                };
                affinity = Some(value);
            }
            "simulation" => {
                let Some(value) = scalar_bool_literal(tree, *value_id) else {
                    panic!("@binding simulation must be a boolean literal");
                };
                simulation = Some(value);
            }
            _ => {
                panic!("unsupported @binding option {key}");
            }
        }
    }

    // require explicit effect kind
    if effect.is_none() {
        panic!("@binding requires an explicit effect");
    }

    // build the effect kind
    let (effect, replay_payload) = build_effect(effect.as_deref(), replay.as_deref());
    let provider = parse_binding_provider(provider.as_deref());
    let affinity = parse_binding_affinity(affinity.as_deref());
    let simulation = parse_binding_simulation(simulation);

    if requires.is_empty() {
        panic!("@binding requires must include at least one action");
    }

    // merge concrete platforms and family selectors
    let platforms = merge_platforms_and_families(platforms, families);

    BindingSpec {
        effect,
        replay_payload,
        requires,
        platforms,
        hosts,
        provider,
        affinity,
        simulation,
    }
}

/// Parse a binding provider from a string.
fn parse_binding_provider(value: Option<&str>) -> CatalogBindingProvider {
    match value {
        Some("runtime") => CatalogBindingProvider::Runtime,
        Some(value) => {
            panic!("unsupported @binding provider {value}");
        }
        None => CatalogBindingProvider::Host,
    }
}

/// Parse a binding affinity from a string.
fn parse_binding_affinity(value: Option<&str>) -> CatalogBindingAffinity {
    match value {
        Some("worker") => CatalogBindingAffinity::Worker,
        Some("main") => CatalogBindingAffinity::Main,
        Some(value) => {
            panic!("unsupported @binding affinity value {value}");
        }
        None => CatalogBindingAffinity::None,
    }
}

/// Parse simulation support from a binding option.
fn parse_binding_simulation(value: Option<bool>) -> CatalogBindingSimulation {
    match value {
        Some(true) => CatalogBindingSimulation::Supported,
        Some(false) | None => CatalogBindingSimulation::Unsupported,
    }
}

/// Build an effect class and replay payload from binding effect facts.
fn build_effect(
    effect: Option<&str>,
    replay: Option<&str>,
) -> (CatalogEffect, CatalogReplayPayload) {
    match effect {
        None => {
            panic!("@binding requires an explicit effect");
        }
        Some("pure") => {
            if replay.is_some() {
                panic!("pure bindings cannot define replay");
            }
            (CatalogEffect::Pure, CatalogReplayPayload::ResultsOnly)
        }
        Some("deterministic") => {
            if replay.is_some() {
                panic!("deterministic bindings cannot define replay");
            }
            (
                CatalogEffect::Deterministic,
                CatalogReplayPayload::ResultsOnly,
            )
        }
        Some("external") => build_external_effect(replay),
        Some(value) => {
            panic!("unsupported @binding effect {value}");
        }
    }
}

/// Build an external effect class from replay facts.
fn build_external_effect(replay: Option<&str>) -> (CatalogEffect, CatalogReplayPayload) {
    match replay {
        Some("forbidden") => (
            CatalogEffect::External {
                replay: CatalogReplayPolicy::NonRecordable,
            },
            CatalogReplayPayload::ResultsOnly,
        ),
        None => (
            CatalogEffect::External {
                replay: CatalogReplayPolicy::Recordable,
            },
            CatalogReplayPayload::ResultsOnly,
        ),
        Some(value) => {
            panic!("unsupported @binding replay {value}");
        }
    }
}

/// Parse required host actions from a decorator value.
fn parse_required_actions(
    tree: &dir::Tree,
    value_id: dir::LocalNodeId<Expression>,
    strings: &StringPool,
) -> Vec<String> {
    // accept a single string as shorthand
    if let Some(value) = scalar_string_literal(tree, value_id, strings) {
        return vec![value];
    }

    // decode array and tuple forms
    let value = tree.get::<Expression>(value_id);
    let elements = match value {
        Expression::ArrayExpression { elements } | Expression::TupleExpression { elements } => {
            elements
        }
        _ => {
            panic!("@binding requires must be a string or an array of strings");
        }
    };

    // parse action names and reject duplicates
    let mut parsed = Vec::with_capacity(elements.len());
    let mut seen = BTreeSet::new();
    for argument_id in elements {
        let argument = tree.get::<Argument>(*argument_id);
        let action_id = argument.value();
        let action = scalar_string_literal(tree, action_id, strings).unwrap_or_else(|| {
            panic!("@binding requires must contain only string literals");
        });
        validate_action_name(&action);
        if !seen.insert(action.clone()) {
            panic!("@binding requires cannot contain duplicate names: {action}");
        }
        parsed.push(action);
    }

    parsed
}

/// Parse platforms from a decorator value.
fn parse_platforms(
    tree: &dir::Tree,
    value_id: dir::LocalNodeId<Expression>,
    strings: &StringPool,
) -> Vec<String> {
    // accept a single string as shorthand
    if let Some(value) = scalar_string_literal(tree, value_id, strings) {
        let mut parsed = BTreeSet::new();
        parse_platform_name(&value, &mut parsed);
        return parsed.into_iter().collect();
    }

    // decode array and tuple forms
    let value = tree.get::<Expression>(value_id);
    let elements = match value {
        Expression::ArrayExpression { elements } | Expression::TupleExpression { elements } => {
            elements
        }
        _ => {
            panic!("@binding platforms must be a string or an array of strings");
        }
    };

    // parse and normalize platform names
    let mut parsed = BTreeSet::new();
    for argument_id in elements {
        let argument = tree.get::<Argument>(*argument_id);
        let platform_id = argument.value();
        let platform = scalar_string_literal(tree, platform_id, strings).unwrap_or_else(|| {
            panic!("@binding platforms must contain only string literals");
        });
        parse_platform_name(&platform, &mut parsed);
    }

    parsed.into_iter().collect()
}

/// Parse one platform tag into canonical platform names.
fn parse_platform_name(name: &str, parsed: &mut BTreeSet<String>) {
    let normalized = name.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        panic!("@binding platform names cannot be empty");
    }

    let platform = Platform::from_str(&normalized)
        .unwrap_or_else(|_| panic!("unsupported @binding platforms value {name}"));
    if matches!(platform, Platform::Unknown | Platform::None) {
        panic!("unsupported @binding platforms value {name}: use a concrete platform tag");
    }

    // require canonical tags only: aliases are rejected with a correction
    let canonical = platform.canonical_tag();
    if canonical != normalized {
        panic!("unsupported @binding platforms value {name}: use canonical tag {canonical}");
    }

    parsed.insert(canonical.to_string());
}

/// Parse platform families from a decorator value.
fn parse_platform_families(
    tree: &dir::Tree,
    value_id: dir::LocalNodeId<Expression>,
    strings: &StringPool,
) -> Vec<String> {
    let mut families = BTreeSet::new();
    parse_string_list(tree, value_id, strings, "@binding families", |family| {
        parse_platform_family_name(&family, &mut families)
    });

    families.into_iter().collect()
}

/// Parse one platform family selector.
fn parse_platform_family_name(name: &str, parsed: &mut BTreeSet<String>) {
    let normalized = name.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        panic!("@binding family names cannot be empty");
    }

    match normalized.as_str() {
        "windows" => {
            parsed.insert(normalized);
        }
        "unix" => {
            parsed.insert(normalized);
        }
        value => {
            panic!("unsupported @binding families value {value}");
        }
    }
}

/// Append all canonical platform tags in one family.
fn append_platform_family(family: &str, parsed: &mut BTreeSet<String>) {
    match family {
        "windows" => append_windows_platform_selector(parsed),
        "unix" => append_unix_platform_selector(parsed),
        value => {
            panic!("unsupported @binding families value {value}");
        }
    }
}

/// Merge platform tags with expanded platform family tags.
fn merge_platforms_and_families(platforms: Vec<String>, families: Vec<String>) -> Vec<String> {
    let mut parsed = BTreeSet::from_iter(platforms);
    for family in families {
        append_platform_family(&family, &mut parsed);
    }

    parsed.into_iter().collect()
}

/// Append all canonical supported Windows platform tags.
fn append_windows_platform_selector(parsed: &mut BTreeSet<String>) {
    parsed.insert(Platform::Windows.canonical_tag().to_string());
}

/// Append all canonical supported Unix platform tags.
fn append_unix_platform_selector(parsed: &mut BTreeSet<String>) {
    parsed.insert(Platform::MacOS.canonical_tag().to_string());
    parsed.insert(Platform::Linux.canonical_tag().to_string());
    parsed.insert(Platform::IOS.canonical_tag().to_string());
    parsed.insert(Platform::Android.canonical_tag().to_string());
}

/// Parse hosts from a decorator value.
fn parse_hosts(
    tree: &dir::Tree,
    value_id: dir::LocalNodeId<Expression>,
    strings: &StringPool,
) -> Vec<String> {
    let mut parsed = BTreeSet::new();
    parse_string_list(tree, value_id, strings, "@binding hosts", |host| {
        parse_host_name(&host, &mut parsed)
    });

    parsed.into_iter().collect()
}

/// Parse one host tag into its canonical name.
fn parse_host_name(name: &str, parsed: &mut BTreeSet<String>) {
    let normalized = name.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        panic!("@binding host names cannot be empty");
    }

    let host = Host::from_str(&normalized)
        .unwrap_or_else(|_| panic!("unsupported @binding hosts value {name}"));

    let canonical = host.canonical_tag();
    if canonical != normalized {
        panic!("unsupported @binding hosts value {name}: use canonical tag {canonical}");
    }

    parsed.insert(canonical.to_string());
}

/// Parse a string or string array decorator value.
fn parse_string_list(
    tree: &dir::Tree,
    value_id: dir::LocalNodeId<Expression>,
    strings: &StringPool,
    label: &str,
    mut parse: impl FnMut(String),
) {
    if let Some(value) = scalar_string_literal(tree, value_id, strings) {
        parse(value);
        return;
    }

    let value = tree.get::<Expression>(value_id);
    let elements = match value {
        Expression::ArrayExpression { elements } | Expression::TupleExpression { elements } => {
            elements
        }
        _ => {
            panic!("{label} must be a string or an array of strings");
        }
    };

    for argument_id in elements {
        let argument = tree.get::<Argument>(*argument_id);
        let value_id = argument.value();
        let value = scalar_string_literal(tree, value_id, strings)
            .unwrap_or_else(|| panic!("{label} must contain only string literals"));
        parse(value);
    }
}

/// Validate one action name in canonical dotted form.
fn validate_action_name(action: &str) {
    if action.trim().is_empty() {
        panic!("@binding action names cannot be empty");
    }

    let segments = action.split('.').collect::<Vec<_>>();
    if segments.len() < 2 {
        panic!("@binding action names must use dotted hierarchy");
    }

    for segment in segments {
        if segment.is_empty() {
            panic!("@binding action names cannot contain empty segments");
        }

        let mut chars = segment.chars();
        let Some(first) = chars.next() else {
            panic!("@binding action names cannot contain empty segments");
        };
        if !first.is_ascii_lowercase() {
            panic!("@binding action segments must start with lowercase letters");
        }

        for character in chars {
            if character.is_ascii_alphanumeric() {
                continue;
            }
            panic!("@binding action names support only alphanumeric characters");
        }
    }
}

/// Validate one binding extern name in canonical dotted form.
fn validate_binding_extern_name(extern_name: &str) {
    if extern_name.trim().is_empty() {
        panic!("@binding id cannot be empty");
    }

    let segments = extern_name.split('.').collect::<Vec<_>>();
    if segments.len() < 4 {
        panic!("@binding id must include at least 4 dotted segments");
    }
    if segments[0] != "destack" {
        panic!("@binding id must start with destack");
    }

    for segment in segments {
        if segment.is_empty() {
            panic!("@binding id cannot contain empty segments");
        }

        let mut chars = segment.chars();
        let Some(first) = chars.next() else {
            panic!("@binding id cannot contain empty segments");
        };
        if !first.is_ascii_lowercase() {
            panic!("@binding id segments must start with lowercase letters");
        }

        for character in chars {
            if character.is_ascii_alphanumeric() {
                continue;
            }
            panic!("@binding id segments support only alphanumeric characters");
        }
    }
}

/// Parse a property key string from a binding options object.
fn parse_option_key(
    tree: &dir::Tree,
    key: Option<&dir::DynamicKey>,
    strings: &StringPool,
) -> Option<String> {
    // decode supported key kinds
    let key = key?;
    match key {
        dir::DynamicKey::Name(name)
        | dir::DynamicKey::Private(name)
        | dir::DynamicKey::Number(name) => Some(strings.get(*name).to_string()),
        dir::DynamicKey::Expression(expression) => {
            scalar_string_literal(tree, *expression, strings)
        }
        dir::DynamicKey::NamedExpression { name, .. } => Some(strings.get(*name).to_string()),
    }
}

/// Parse a string literal from a scalar expression.
fn scalar_string_literal(
    tree: &dir::Tree,
    value_id: dir::LocalNodeId<Expression>,
    strings: &StringPool,
) -> Option<String> {
    // decode string literal values
    let value = tree.get::<Expression>(value_id);
    let Expression::ScalarLiteral { value } = value else {
        return None;
    };
    let ScalarLiteral::String(value_id) = value else {
        return None;
    };
    Some(strings.get(*value_id).to_string())
}

/// Parse a boolean literal from a scalar expression.
fn scalar_bool_literal(tree: &dir::Tree, value_id: dir::LocalNodeId<Expression>) -> Option<bool> {
    let value = tree.get::<Expression>(value_id);
    let Expression::ScalarLiteral { value } = value else {
        return None;
    };
    let ScalarLiteral::Boolean(value) = value else {
        return None;
    };

    Some(*value)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::{BindingType, CatalogBindingReplayKind, CatalogEntropyKind};

    use super::{
        binding_replay_kind_for_name, binding_type_supports_integer_constants,
        module_platform_domain, module_platform_implementation_prefix,
        module_platform_implementation_prefix_from_path, qualify_platform_implementation_name,
    };

    /// Parse platform domain names from platform module uris.
    #[test]
    fn test_module_platform_domain_parses_library_uri() {
        assert_eq!(
            module_platform_domain("library://platform/display/window.ds"),
            Some("display".to_string())
        );
        assert_eq!(
            module_platform_domain("library://platform/crypto/key.ds"),
            Some("crypto".to_string())
        );
        assert_eq!(
            module_platform_domain("library://library/other/window.ds"),
            None
        );
    }

    /// Preserve nested platform paths for implementation naming.
    #[test]
    fn test_module_platform_implementation_prefix_parses_nested_paths() {
        assert_eq!(
            module_platform_implementation_prefix("library://platform/device/midi/backend.ds"),
            Some("midi".to_string())
        );
        assert_eq!(
            module_platform_implementation_prefix("library://platform/display/window.ds"),
            None
        );
    }

    /// Preserve nested platform paths from filesystem locations.
    #[test]
    fn test_module_platform_implementation_prefix_from_path_parses_nested_paths() {
        assert_eq!(
            module_platform_implementation_prefix_from_path(Path::new(
                "/tmp/destack/language/library/platform/device/midi/backend.ds"
            )),
            Some("midi".to_string())
        );
        assert_eq!(
            module_platform_implementation_prefix_from_path(Path::new(
                "/tmp/destack/language/library/platform/display/window.ds"
            )),
            None
        );
    }

    /// Prefix implementation names with nested platform path segments once.
    #[test]
    fn test_qualify_platform_implementation_name_keeps_nested_path_segments() {
        assert_eq!(
            qualify_platform_implementation_name(
                Some(Path::new(
                    "/tmp/destack/language/library/platform/device/midi/backend.ds"
                )),
                "library://platform/device/midi/backend.ds",
                "backend.list"
            ),
            "midi.backend.list".to_string()
        );
        assert_eq!(
            qualify_platform_implementation_name(
                Some(Path::new(
                    "/tmp/destack/language/library/platform/device/midi/backend.ds"
                )),
                "library://platform/device/midi/backend.ds",
                "midi.backend.list"
            ),
            "midi.backend.list".to_string()
        );
        assert_eq!(
            qualify_platform_implementation_name(
                Some(Path::new(
                    "/tmp/destack/language/library/platform/device/bluetooth.ds"
                )),
                "library://platform/device/bluetooth.ds",
                "bluetooth.session.open"
            ),
            "bluetooth.session.open".to_string()
        );
    }

    /// Accept integer newtypes and reject non-integer constants.
    #[test]
    fn test_binding_type_supports_integer_constants_for_integer_newtypes() {
        let integer_newtype = BindingType::Newtype {
            name: "WindowEventKindMask".to_string(),
            domain: "display".to_string(),
            inner: Box::new(BindingType::UInt(64)),
        };
        assert!(binding_type_supports_integer_constants(&integer_newtype));
        assert!(!binding_type_supports_integer_constants(&BindingType::Bool));
    }

    /// Route random stream allocation bindings through stream replay events.
    #[test]
    fn test_binding_replay_kind_for_name_maps_random_stream_allocations() {
        assert_eq!(
            binding_replay_kind_for_name("destack.random.stream.create"),
            CatalogBindingReplayKind::Entropy(CatalogEntropyKind::RandomStreamCreate)
        );
        assert_eq!(
            binding_replay_kind_for_name("destack.random.stream.in"),
            CatalogBindingReplayKind::Entropy(CatalogEntropyKind::RandomStreamCreate)
        );
    }

    /// Route secure bytesTry bindings through random bytes replay events.
    #[test]
    fn test_binding_replay_kind_for_name_maps_secure_bytes_try() {
        assert_eq!(
            binding_replay_kind_for_name("destack.random.secure.bytesTry"),
            CatalogBindingReplayKind::Entropy(CatalogEntropyKind::RandomReadBytes)
        );
    }
}
