use std::collections::{BTreeSet, HashSet};
use std::str::FromStr;

use destack_artifact::Platform;
use destack_compiler::Compiler;
use destack_core::StringPool;
use destack_dir::{
    self as dir, Annotation, Argument, Declaration, Expression, GlobalSymbolId, LanguageSymbol,
    ScalarLiteral,
};
use destack_source::ModuleId;
use destack_workspace::ProfileId;

use super::domain::qualify_platform_implementation_name;
use crate::context::GeneratorContext;
use crate::platform::model::{
    BindingCatalog, BindingEntry, BindingParameter, BindingReturn, BindingTypeContext,
    CatalogBindingAffinity, CatalogBindingBlocking, CatalogBindingReplayKind, CatalogBindingScope,
    CatalogBindingSimulation, CatalogEffectClass, CatalogEntropyKind, CatalogReplayPayload,
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
    /// Effect classification for replay and policy.
    effect_class: CatalogEffectClass,
    /// Replay routing for the binding.
    replay_kind: CatalogBindingReplayKind,
    /// Replay payload policy for recorded bindings.
    replay_payload: CatalogReplayPayload,
    /// Required platform capabilities for this binding.
    requires: Vec<String>,
    /// Host platforms where this binding is supported.
    host_platforms: Vec<String>,
    /// Platform scope for this binding.
    scope: CatalogBindingScope,
    /// Blocking behavior for this binding.
    blocking: CatalogBindingBlocking,
    /// Affinity behavior for this binding.
    affinity: CatalogBindingAffinity,
    /// Simulation capability for this binding.
    simulation: CatalogBindingSimulation,
}

/// Binding decorator payload extracted from an annotation.
#[derive(Debug, Clone)]
struct BindingDecorator {
    /// Optional binding name override.
    extern_name: Option<String>,
    /// Optional effect class override.
    effect_class: CatalogEffectClass,
    /// Optional replay payload override.
    replay_payload: CatalogReplayPayload,
    /// Required platform capabilities for this binding.
    requires: Vec<String>,
    /// Host platforms where this binding is supported.
    host_platforms: Vec<String>,
    /// Platform scope for this binding.
    scope: CatalogBindingScope,
    /// Blocking behavior for this binding.
    blocking: CatalogBindingBlocking,
    /// Affinity behavior for this binding.
    affinity: CatalogBindingAffinity,
    /// Simulation capability for this binding.
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
        .item(LanguageSymbol::Binding)
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
                binding.effect_class,
                binding.replay_payload,
                binding.requires,
                binding.host_platforms,
                binding.scope,
                binding.blocking,
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
    effect_class: CatalogEffectClass,
    replay_payload: CatalogReplayPayload,
    requires: Vec<String>,
    host_platforms: Vec<String>,
    scope: CatalogBindingScope,
    blocking: CatalogBindingBlocking,
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
        effect_class,
        replay_payload,
        requires,
        host_platforms,
        scope,
        blocking,
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
        effect_class: record.effect_class,
        replay_kind: record.replay_kind,
        replay_payload: record.replay_payload,
        requires: record.requires,
        host_platforms: record.host_platforms,
        scope: record.scope,
        blocking: record.blocking,
        affinity: record.affinity,
        simulation: record.simulation,
    };

    // insert the entry and validate signature stability
    if let Some(existing) = domain_bindings.insert(record.extern_name.clone(), entry.clone())
        && (existing.implementation_name != entry.implementation_name
            || existing.documentation != entry.documentation
            || existing.signature != entry.signature
            || existing.effect_class != entry.effect_class
            || existing.replay_kind != entry.replay_kind
            || existing.replay_payload != entry.replay_payload
            || existing.requires != entry.requires
            || existing.host_platforms != entry.host_platforms
            || existing.scope != entry.scope
            || existing.blocking != entry.blocking
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
    let spec = parse_effect_spec(tree, value_id, strings);

    // return the payload
    BindingDecorator {
        extern_name,
        effect_class: spec.effect_class,
        replay_payload: spec.replay_payload,
        requires: spec.requires,
        host_platforms: spec.host_platforms,
        scope: spec.scope,
        blocking: spec.blocking,
        affinity: spec.affinity,
        simulation: spec.simulation,
    }
}

/// Parsed effect options for bindings.
struct BindingEffectSpec {
    /// Effect classification for the binding.
    effect_class: CatalogEffectClass,
    /// Replay payload policy for recorded bindings.
    replay_payload: CatalogReplayPayload,
    /// Required platform capabilities for this binding.
    requires: Vec<String>,
    /// Host platforms where this binding is supported.
    host_platforms: Vec<String>,
    /// Platform scope for this binding.
    scope: CatalogBindingScope,
    /// Blocking behavior for this binding.
    blocking: CatalogBindingBlocking,
    /// Affinity behavior for this binding.
    affinity: CatalogBindingAffinity,
    /// Simulation capability for this binding.
    simulation: CatalogBindingSimulation,
}

/// Parse effect options from a binding decorator.
fn parse_effect_spec(
    tree: &dir::Tree,
    value_id: dir::LocalNodeId<Expression>,
    strings: &StringPool,
) -> BindingEffectSpec {
    // require an object literal payload
    let expression = tree.get::<Expression>(value_id);
    let Expression::ObjectExpression { properties } = expression else {
        panic!("@binding options must be an object literal");
    };

    // collect effect properties
    let mut effect = None;
    let mut replay = None;
    let mut log = None;
    let mut payload = None;
    let mut capabilities = Vec::new();
    let mut host_platforms = Vec::new();
    let mut scope = None;
    let mut blocking = None;
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
            "capabilities" => {
                capabilities = parse_capabilities_list(tree, *value_id, strings);
            }
            "platforms" => {
                host_platforms = parse_host_platforms_list(tree, *value_id, strings);
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
            "log" => {
                let Some(value) = scalar_string_literal(tree, *value_id, strings) else {
                    panic!("@binding log must be a string literal");
                };
                log = Some(value);
            }
            "payload" => {
                let Some(value) = scalar_string_literal(tree, *value_id, strings) else {
                    panic!("@binding payload must be a string literal");
                };
                payload = Some(value);
            }
            "scope" => {
                let Some(value) = scalar_string_literal(tree, *value_id, strings) else {
                    panic!("@binding scope must be a string literal");
                };
                scope = Some(value);
            }
            "blocking" => {
                let Some(value) = scalar_string_literal(tree, *value_id, strings) else {
                    panic!("@binding blocking must be a string literal");
                };
                blocking = Some(value);
            }
            "affinity" => {
                let Some(value) = scalar_string_literal(tree, *value_id, strings) else {
                    panic!("@binding affinity must be a string literal");
                };
                affinity = Some(value);
            }
            "simulation" => {
                let Some(value) = scalar_string_literal(tree, *value_id, strings) else {
                    panic!("@binding simulation must be a string literal");
                };
                simulation = Some(value);
            }
            _ => {
                panic!("unsupported @binding option {key}");
            }
        }
    }

    // require explicit effect classification and replay policy
    if effect.is_none() {
        panic!("@binding requires an explicit effect classification");
    }
    if replay.is_none() {
        panic!("@binding requires an explicit replay policy");
    }

    // build the effect classification
    let replay = replay.as_deref().unwrap_or_default();
    let effect_class = build_effect_class(effect.as_deref(), replay);
    let replay_payload = parse_replay_payload(payload.as_deref());
    let scope = parse_binding_scope(scope.as_deref());
    let blocking = parse_binding_blocking(blocking.as_deref());
    let affinity = parse_binding_affinity(affinity.as_deref());
    let simulation = parse_binding_simulation(simulation.as_deref());

    if log.is_some() {
        panic!("@binding log is runtime-owned and should not be specified");
    }
    if payload.is_some()
        && !matches!(
            effect_class,
            CatalogEffectClass::External {
                replay: CatalogReplayPolicy::Recordable
            }
        )
    {
        panic!("@binding payload requires a recordable external effect");
    }
    if capabilities.is_empty() {
        panic!("@binding capabilities must include at least one capability");
    }

    BindingEffectSpec {
        effect_class,
        replay_payload,
        requires: capabilities,
        host_platforms,
        scope,
        blocking,
        affinity,
        simulation,
    }
}

/// Parse a binding scope from a string.
fn parse_binding_scope(value: Option<&str>) -> CatalogBindingScope {
    match value {
        Some("host") => CatalogBindingScope::Host,
        Some("runtime") => CatalogBindingScope::Runtime,
        Some(value) => {
            panic!("unsupported @binding scope {value}");
        }
        None => {
            panic!("@binding requires an explicit scope classification");
        }
    }
}

/// Parse a binding blocking behavior from a string.
fn parse_binding_blocking(value: Option<&str>) -> CatalogBindingBlocking {
    match value {
        Some("always") => CatalogBindingBlocking::Always,
        Some("never") => CatalogBindingBlocking::Never,
        Some("sometimes") => CatalogBindingBlocking::Sometimes,
        Some(value) => {
            panic!("unsupported @binding blocking value {value}");
        }
        None => {
            panic!("@binding requires an explicit blocking classification");
        }
    }
}

/// Parse a binding affinity from a string.
fn parse_binding_affinity(value: Option<&str>) -> CatalogBindingAffinity {
    match value {
        Some("any") => CatalogBindingAffinity::Any,
        Some("eventLoop") => CatalogBindingAffinity::EventLoop,
        Some("owner") => CatalogBindingAffinity::Owner,
        Some("processMain") => CatalogBindingAffinity::ProcessMain,
        Some(value) => {
            panic!("unsupported @binding affinity value {value}");
        }
        None => {
            panic!("@binding requires an explicit affinity classification");
        }
    }
}

/// Parse a binding simulation capability from a string.
fn parse_binding_simulation(value: Option<&str>) -> CatalogBindingSimulation {
    match value {
        Some("unsupported") => CatalogBindingSimulation::Unsupported,
        Some("stub") => CatalogBindingSimulation::Stub,
        Some("model") => CatalogBindingSimulation::Model,
        Some(value) => {
            panic!("unsupported @binding simulation value {value}");
        }
        None => {
            panic!("@binding requires an explicit simulation classification");
        }
    }
}

/// Build an effect class from optional effect and replay names.
fn build_effect_class(effect: Option<&str>, replay: &str) -> CatalogEffectClass {
    // parse replay policy
    let replay = match replay {
        "recordable" => CatalogReplayPolicy::Recordable,
        "nonrecordable" => CatalogReplayPolicy::NonRecordable,
        value => {
            panic!("unsupported @binding replay policy {value}");
        }
    };

    // map effect to the classification
    match effect {
        None => {
            panic!("@binding requires an explicit effect classification");
        }
        Some("pure") => {
            if replay != CatalogReplayPolicy::NonRecordable {
                panic!("pure bindings must use replay: nonrecordable");
            }
            CatalogEffectClass::Pure
        }
        Some("deterministic") => {
            if replay != CatalogReplayPolicy::NonRecordable {
                panic!("deterministic bindings must use replay: nonrecordable");
            }
            CatalogEffectClass::Deterministic
        }
        Some("external" | "io") => CatalogEffectClass::External { replay },
        Some(value) => {
            panic!("unsupported @binding effect {value}");
        }
    }
}

/// Parse a replay payload policy from a string.
fn parse_replay_payload(value: Option<&str>) -> CatalogReplayPayload {
    match value {
        None => CatalogReplayPayload::ResultsOnly,
        Some("results") => CatalogReplayPayload::ResultsOnly,
        Some("argumentsAndResults") => CatalogReplayPayload::ArgumentsAndResults,
        Some(value) => {
            panic!("unsupported @binding payload {value}");
        }
    }
}

/// Parse required platform capabilities from a decorator value.
fn parse_capabilities_list(
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
            panic!("@binding capabilities must be a string or an array of strings");
        }
    };

    // parse capability names and reject duplicates
    let mut parsed = Vec::with_capacity(elements.len());
    let mut seen = BTreeSet::new();
    for argument_id in elements {
        let argument = tree.get::<Argument>(*argument_id);
        let capability_id = argument.value();
        let capability = scalar_string_literal(tree, capability_id, strings).unwrap_or_else(|| {
            panic!("@binding capabilities must contain only string literals");
        });
        validate_capability_name(&capability);
        if !seen.insert(capability.clone()) {
            panic!("@binding capabilities cannot contain duplicate names: {capability}");
        }
        parsed.push(capability);
    }

    parsed
}

/// Parse host platforms from a decorator value.
fn parse_host_platforms_list(
    tree: &dir::Tree,
    value_id: dir::LocalNodeId<Expression>,
    strings: &StringPool,
) -> Vec<String> {
    // accept a single string as shorthand
    if let Some(value) = scalar_string_literal(tree, value_id, strings) {
        let mut parsed = BTreeSet::new();
        parse_host_platform_name(&value, &mut parsed);
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

    // parse and normalize host platform names
    let mut parsed = BTreeSet::new();
    for argument_id in elements {
        let argument = tree.get::<Argument>(*argument_id);
        let platform_id = argument.value();
        let platform = scalar_string_literal(tree, platform_id, strings).unwrap_or_else(|| {
            panic!("@binding platforms must contain only string literals");
        });
        parse_host_platform_name(&platform, &mut parsed);
    }

    parsed.into_iter().collect()
}

/// Parse one host platform selector into canonical platform names.
fn parse_host_platform_name(name: &str, parsed: &mut BTreeSet<String>) {
    let normalized = name.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        panic!("@binding platforms names cannot be empty");
    }

    if normalized == "unix" {
        append_unix_platform_selector(parsed);
        return;
    }

    if normalized == "bsd" {
        panic!("unsupported @binding platforms value {name}: bsd hosts are no longer supported");
    }

    // parse the platform selector through the target platform enum
    let platform = Platform::from_str(&normalized)
        .unwrap_or_else(|_| panic!("unsupported @binding platforms value {name}"));

    // require canonical tags only: aliases are rejected with a correction
    let canonical = platform.canonical_tag();
    if canonical != normalized {
        panic!("unsupported @binding platforms value {name}: use canonical tag {canonical}");
    }

    parsed.insert(canonical.to_string());
}

/// Append all canonical supported Unix host platform tags.
fn append_unix_platform_selector(parsed: &mut BTreeSet<String>) {
    parsed.insert(Platform::MacOS.canonical_tag().to_string());
    parsed.insert(Platform::Linux.canonical_tag().to_string());
    parsed.insert(Platform::IOS.canonical_tag().to_string());
    parsed.insert(Platform::Android.canonical_tag().to_string());
}

/// Validate one capability name in canonical dotted form.
fn validate_capability_name(capability: &str) {
    if capability.trim().is_empty() {
        panic!("@binding capability names cannot be empty");
    }

    let segments = capability.split('.').collect::<Vec<_>>();
    if segments.len() < 2 {
        panic!("@binding capability names must use dotted hierarchy");
    }

    for segment in segments {
        if segment.is_empty() {
            panic!("@binding capability names cannot contain empty segments");
        }

        let mut chars = segment.chars();
        let Some(first) = chars.next() else {
            panic!("@binding capability names cannot contain empty segments");
        };
        if !first.is_ascii_lowercase() {
            panic!("@binding capability segments must start with lowercase letters");
        }

        for character in chars {
            if character.is_ascii_alphanumeric() {
                continue;
            }
            panic!("@binding capability names support only alphanumeric characters");
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

// log kind validation happens in parse_effect_spec

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
