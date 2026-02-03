use destack_base::StringPool;
use destack_builtin::LanguageSymbol;
use destack_dir::{
    self as dir, Annotation, Argument, Declaration, Expression, GlobalSymbolId, ScalarLiteral,
};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Program};

use crate::format::{
    binding_type_symbols, collect_binding_params, collect_binding_return, format_declared_signature,
};
use crate::model::{
    BindingCatalog, BindingEntry, BindingReplayKind, BindingReturn, EffectClass, RandomEventKind,
    ReplayPayload, ReplayPolicy, TimeEventKind,
};

/// Binding metadata extracted from a declaration node.
#[derive(Debug, Clone)]
struct BindingRecord {
    /// Fully qualified extern binding name.
    extern_name: String,
    /// Canonical signature string for stability checks.
    signature: String,
    /// Parameter metadata payload.
    params: Vec<crate::model::BindingParameter>,
    /// Return binding type for generated wrappers.
    return_binding: BindingReturn,
    /// Effect classification for replay and policy.
    effect_class: EffectClass,
    /// Replay routing for the binding.
    replay_kind: BindingReplayKind,
    /// Replay payload policy for recorded bindings.
    replay_payload: ReplayPayload,
}

/// Binding decorator payload extracted from an annotation.
#[derive(Debug, Clone)]
struct BindingDecorator {
    /// Optional binding name override.
    extern_name: Option<String>,
    /// Optional effect class override.
    effect_class: EffectClass,
    /// Optional replay payload override.
    replay_payload: ReplayPayload,
}

/// Resolve replay routing for a binding name.
fn binding_replay_kind_for_name(name: &str) -> BindingReplayKind {
    match name {
        "destack.time.wallNs" => BindingReplayKind::Time(TimeEventKind::WallClockRead),
        "destack.time.monoNs" => BindingReplayKind::Time(TimeEventKind::MonotonicSample),
        "destack.random.stream" => BindingReplayKind::Random(RandomEventKind::Stream),
        "destack.random.nextU64" => BindingReplayKind::Random(RandomEventKind::NextU64),
        "destack.random.nextU64From" => BindingReplayKind::Random(RandomEventKind::NextU64),
        "destack.random.fillBytes" => BindingReplayKind::Random(RandomEventKind::Bytes),
        "destack.random.fillBytesFrom" => BindingReplayKind::Random(RandomEventKind::Bytes),
        "destack.random.secureBytes" => BindingReplayKind::Random(RandomEventKind::Bytes),
        _ => BindingReplayKind::Regular,
    }
}

/// Collect platform bindings from builtin modules.
pub(crate) fn collect_platform_bindings(
    program: &Program,
    strings: &StringPool,
    profile_id: ProfileId,
    platform_modules: &[ModuleId],
) -> BindingCatalog {
    // resolve the canonical binding decorator symbol
    let binding_decorator_symbol = binding_decorator_symbol_id(program, profile_id);

    // collect binding type symbols
    let binding_symbols = binding_type_symbols(program, profile_id);

    // collect bindings by domain
    let mut domains: BindingCatalog = BindingCatalog::default();

    // visit builtin modules and extract binding annotations
    for module_id in platform_modules {
        // load module metadata
        let module = program.modules.get(*module_id);
        let module = module.read();
        let dir = module.dir(profile_id);
        let tree = dir.tree.read();
        let types = dir.types.read();
        let symbols = dir.symbols.read();

        // scan expressions for binding declarations
        for (expression_id, expression) in tree.iter_nodes_of_type::<Expression>() {
            // load binding decorator payload
            let binding = binding_decorator_value(
                &tree,
                expression_id.into_any(),
                strings,
                binding_decorator_symbol,
            );
            let Some(binding) = binding else {
                continue;
            };

            // resolve the declaration referenced by the expression
            let declaration_id = declaration_from_expression(&tree, expression_id, expression);
            let Some(declaration_id) = declaration_id else {
                continue;
            };

            // filter to function declarations
            let declaration = tree.get::<Declaration>(declaration_id);
            let Declaration::Function { signature, .. } = declaration else {
                continue;
            };

            // resolve the extern binding name
            let symbol = symbols.get_symbol(declaration.symbol());
            let extern_name = binding
                .extern_name
                .or_else(|| symbol.name().map(|name| strings.get(name).to_string()));

            // format the canonical signature for the declaration
            let signature_text = format_declared_signature(
                declaration_id,
                declaration,
                &module,
                &program.modules,
                strings,
                profile_id,
            );

            // collect parameter and return metadata from the AST
            let domain = extern_name
                .as_deref()
                .map(binding_domain)
                .unwrap_or_else(|| "global".to_string());

            let params = collect_binding_params(
                signature,
                module.id,
                &tree,
                &types,
                &program.modules,
                strings,
                profile_id,
                &binding_symbols,
                &domain,
            );
            let return_binding = collect_binding_return(
                declaration_id,
                signature,
                module.id,
                &tree,
                &types,
                &program.modules,
                strings,
                profile_id,
                &binding_symbols,
                &domain,
            );

            // insert parsed binding metadata into the catalog
            if let Some(entry) = binding_from_node(
                extern_name,
                signature_text,
                params,
                return_binding,
                binding.effect_class,
                binding.replay_payload,
            ) {
                insert_binding(&mut domains, entry);
            }
        }
    }

    domains
}

/// Resolve the canonical binding decorator symbol for the profile.
fn binding_decorator_symbol_id(program: &Program, profile_id: ProfileId) -> GlobalSymbolId {
    let builtins = program
        .builtins
        .as_ref()
        .expect("builtins must be loaded for binding generation");
    builtins
        .items
        .get(&(profile_id, LanguageSymbol::Binding))
        .map(|item| *item)
        .unwrap_or_else(|| panic!("missing binding decorator symbol for profile {profile_id:?}"))
}

/// Return the domain portion of a binding name.
fn binding_domain(extern_name: &str) -> String {
    let mut parts = extern_name.split('.');
    let _prefix = parts.next();
    parts.next().unwrap_or("global").to_string()
}

/// Build a binding record from parsed metadata.
fn binding_from_node(
    extern_name: Option<String>,
    signature: String,
    params: Vec<crate::model::BindingParameter>,
    return_binding: BindingReturn,
    effect_class: EffectClass,
    replay_payload: ReplayPayload,
) -> Option<BindingRecord> {
    let extern_name = extern_name?;
    if !extern_name.starts_with("destack.") {
        return None;
    }

    Some(BindingRecord {
        replay_kind: binding_replay_kind_for_name(&extern_name),
        extern_name,
        signature,
        params,
        return_binding,
        effect_class,
        replay_payload,
    })
}

/// Insert a binding record into the domain catalog.
fn insert_binding(domains: &mut BindingCatalog, record: BindingRecord) {
    // resolve the binding domain
    let domain = binding_domain(&record.extern_name);
    let domain_bindings = domains.entry(domain).or_default();

    // build a canonical entry for comparisons
    let entry = BindingEntry {
        signature: record.signature,
        parameters: record.params,
        return_binding: record.return_binding.binding_type,
        return_is_result: record.return_binding.is_result,
        effect_class: record.effect_class,
        replay_kind: record.replay_kind,
        replay_payload: record.replay_payload,
    };

    // insert the entry and validate signature stability
    if let Some(existing) = domain_bindings.insert(record.extern_name.clone(), entry.clone())
        && (existing.signature != entry.signature
            || existing.effect_class != entry.effect_class
            || existing.replay_kind != entry.replay_kind
            || existing.replay_payload != entry.replay_payload)
    {
        panic!(
            "binding signature mismatch for {}: {:?} vs {:?}",
            record.extern_name, existing, entry
        );
    }
}

/// Extract the binding decorator value from a declaration expression.
fn binding_decorator_value(
    tree: &dir::NodeTree,
    node_id: dir::LocalNodeIdAny,
    strings: &StringPool,
    binding_decorator_symbol: GlobalSymbolId,
) -> Option<BindingDecorator> {
    // scan annotations for the binding decorator
    let annotations = tree.get_annotations(node_id.id);
    for annotation_id in annotations {
        let annotation = tree.get::<Annotation>(annotation_id);
        let Annotation::Decorator {
            left, arguments, ..
        } = annotation
        else {
            continue;
        };
        let decorator_symbol = decorator_symbol_from_expression(tree, *left)?;
        if decorator_symbol != binding_decorator_symbol {
            continue;
        }

        // resolve decorator arguments
        return Some(decorator_binding_argument(
            tree,
            arguments.as_ref(),
            strings,
        ));
    }
    None
}

/// Resolve the decorator symbol from an expression node.
fn decorator_symbol_from_expression(
    tree: &dir::NodeTree,
    expr_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    let expression = tree.get::<Expression>(expr_id);
    expression.target_symbol()
}

/// Resolve a declaration node from a binding expression.
fn declaration_from_expression(
    tree: &dir::NodeTree,
    _expression_id: dir::LocalNodeId<Expression>,
    expression: &Expression,
) -> Option<dir::LocalNodeId<Declaration>> {
    match expression {
        Expression::Declaration { declaration } => Some(*declaration),
        Expression::Statement { statement } => {
            let inner = tree.get::<Expression>(*statement);
            declaration_from_expression(tree, *statement, inner)
        }
        _ => None,
    }
}

/// Extract the binding decorator arguments.
fn decorator_binding_argument(
    tree: &dir::NodeTree,
    arguments: Option<&Vec<dir::LocalNodeId<Argument>>>,
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
    }

    let argument = tree.get::<Argument>(arguments[1]);
    let value_id = argument.value();
    let spec = parse_effect_spec(tree, value_id, strings);

    // return the payload
    BindingDecorator {
        extern_name,
        effect_class: spec.effect_class,
        replay_payload: spec.replay_payload,
    }
}

/// Parsed effect options for bindings.
struct BindingEffectSpec {
    /// Effect classification for the binding.
    effect_class: EffectClass,
    /// Replay payload policy for recorded bindings.
    replay_payload: ReplayPayload,
}

/// Parse effect options from a binding decorator.
fn parse_effect_spec(
    tree: &dir::NodeTree,
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
        let Some(value) = scalar_string_literal(tree, *value_id, strings) else {
            continue;
        };
        match key.as_str() {
            "effect" => effect = Some(value),
            "replay" => replay = Some(value),
            "log" => log = Some(value),
            "payload" => payload = Some(value),
            _ => {
                panic!("unsupported @binding option {key}");
            }
        }
    }

    // require explicit effect classification
    if effect.is_none() {
        panic!("@binding requires an explicit effect classification");
    }

    // build the effect classification
    let effect_class = build_effect_class(effect.as_deref(), replay.as_deref());
    let replay_payload = parse_replay_payload(payload.as_deref());

    if log.is_some() {
        panic!("@binding log is runtime-owned and should not be specified");
    }
    if payload.is_some() && !matches!(effect_class, EffectClass::External { .. }) {
        panic!("@binding payload requires an external effect");
    }

    BindingEffectSpec {
        effect_class,
        replay_payload,
    }
}

/// Build an effect class from optional effect and replay names.
fn build_effect_class(effect: Option<&str>, replay: Option<&str>) -> EffectClass {
    // parse replay policy
    let replay = match replay {
        None => None,
        Some("recordable") => Some(ReplayPolicy::Recordable),
        Some("nonrecordable") => Some(ReplayPolicy::NonRecordable),
        Some(value) => {
            panic!("unsupported @binding replay policy {value}");
        }
    };

    // map effect to the classification
    match effect {
        None => {
            panic!("@binding requires an explicit effect classification");
        }
        Some("pure") => {
            if replay.is_some() {
                panic!("pure bindings cannot specify replay policy");
            }
            EffectClass::Pure
        }
        Some("deterministic") => {
            if replay.is_some() {
                panic!("deterministic bindings cannot specify replay policy");
            }
            EffectClass::Deterministic
        }
        Some("external" | "io") => {
            let replay = replay.unwrap_or_else(|| {
                panic!("@binding external effects must specify replay policy");
            });
            EffectClass::External { replay }
        }
        Some(value) => {
            panic!("unsupported @binding effect {value}");
        }
    }
}

/// Parse a replay payload policy from a string.
fn parse_replay_payload(value: Option<&str>) -> ReplayPayload {
    match value {
        None => ReplayPayload::ResultsOnly,
        Some("results") => ReplayPayload::ResultsOnly,
        Some("argumentsAndResults") => ReplayPayload::ArgumentsAndResults,
        Some(value) => {
            panic!("unsupported @binding payload {value}");
        }
    }
}

// log kind validation happens in parse_effect_spec

/// Parse a property key string from a binding options object.
fn parse_option_key(
    tree: &dir::NodeTree,
    key: Option<&dir::DynamicKey>,
    strings: &StringPool,
) -> Option<String> {
    // decode supported key kinds
    let key = key?;
    match key {
        dir::DynamicKey::Name(name) | dir::DynamicKey::Number(name) => {
            Some(strings.get(*name).to_string())
        }
        dir::DynamicKey::Expression(expression) => {
            scalar_string_literal(tree, *expression, strings)
        }
        dir::DynamicKey::NamedExpression { name, .. } => Some(strings.get(*name).to_string()),
    }
}

/// Parse a string literal from a scalar expression.
fn scalar_string_literal(
    tree: &dir::NodeTree,
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
