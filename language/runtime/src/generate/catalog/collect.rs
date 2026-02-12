use std::collections::BTreeSet;

use destack_base::StringPool;
use destack_builtin::LanguageSymbol;
use destack_dir::{
    self as dir, Annotation, Argument, Declaration, Expression, GlobalSymbolId, ScalarLiteral,
};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Program};

use super::format::{
    binding_type_symbols, collect_binding_params, collect_binding_return, format_declared_signature,
};
use crate::model::{
    BindingBlocking, BindingCatalog, BindingEntry, BindingReplayKind, BindingReturn, BindingScope,
    EffectClass, RandomEventKind, ReplayPayload, ReplayPolicy, TimeEventKind,
};

/// Binding metadata extracted from a declaration node.
#[derive(Debug, Clone)]
struct BindingRecord {
    /// Declaration name used by runtime implementation functions.
    implementation_name: String,
    /// Declaration documentation extracted from builtin sources.
    documentation: Option<String>,
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
    /// Required platform capabilities for this binding.
    requires: Vec<String>,
    /// Platform scope for this binding.
    scope: BindingScope,
    /// Blocking behavior for this binding.
    blocking: BindingBlocking,
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
    /// Required platform capabilities for this binding.
    requires: Vec<String>,
    /// Platform scope for this binding.
    scope: BindingScope,
    /// Blocking behavior for this binding.
    blocking: BindingBlocking,
}

/// Resolve replay routing for a binding name.
fn binding_replay_kind_for_name(name: &str) -> BindingReplayKind {
    let mut segments = name.split('.');
    let Some(prefix) = segments.next() else {
        return BindingReplayKind::Regular;
    };
    if prefix != "destack" {
        return BindingReplayKind::Regular;
    }

    let Some(domain) = segments.next() else {
        return BindingReplayKind::Regular;
    };
    let operation = segments.next_back().unwrap_or_default();

    if domain == "time" {
        if operation == "wallNs" {
            return BindingReplayKind::Time(TimeEventKind::WallClockRead);
        }

        if operation == "monoNs" {
            return BindingReplayKind::Time(TimeEventKind::MonotonicSample);
        }

        return BindingReplayKind::Regular;
    }

    if domain == "random" {
        if operation == "stream" || operation == "streamIn" {
            return BindingReplayKind::Random(RandomEventKind::Stream);
        }

        if operation == "nextU64" || operation == "nextU64From" {
            return BindingReplayKind::Random(RandomEventKind::NextU64);
        }

        if operation == "fillBytes"
            || operation == "fillBytesFrom"
            || operation == "secureBytes"
            || operation == "bytes"
        {
            return BindingReplayKind::Random(RandomEventKind::Bytes);
        }
    }

    BindingReplayKind::Regular
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

            // resolve declaration documentation comments
            let documentation =
                binding_documentation(&tree, expression_id, declaration_id, strings);

            // resolve the extern binding name
            let symbol = symbols.get_symbol(declaration.symbol());
            let implementation_name = symbol.name().map(|name| strings.get(name).to_string());
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
                implementation_name,
                documentation,
                extern_name,
                signature_text,
                params,
                return_binding,
                binding.effect_class,
                binding.replay_payload,
                binding.requires,
                binding.scope,
                binding.blocking,
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
    implementation_name: Option<String>,
    documentation: Option<String>,
    extern_name: Option<String>,
    signature: String,
    params: Vec<crate::model::BindingParameter>,
    return_binding: BindingReturn,
    effect_class: EffectClass,
    replay_payload: ReplayPayload,
    requires: Vec<String>,
    scope: BindingScope,
    blocking: BindingBlocking,
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
        scope,
        blocking,
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
        scope: record.scope,
        blocking: record.blocking,
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
            || existing.scope != entry.scope
            || existing.blocking != entry.blocking)
    {
        panic!(
            "binding signature mismatch for {}: {:?} vs {:?}",
            record.extern_name, existing, entry
        );
    }
}

/// Collect binding documentation comments from DIR annotations.
fn binding_documentation(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<Expression>,
    declaration_id: dir::LocalNodeId<Declaration>,
    strings: &StringPool,
) -> Option<String> {
    // prefer docs attached to the declaration node
    let declaration_docs = annotation_docs(tree, declaration_id.id, strings);
    if declaration_docs.is_some() {
        return declaration_docs;
    }

    // fall back to docs attached to the declaration expression wrapper
    annotation_docs(tree, expression_id.id, strings)
}

/// Collect documentation comments from one annotated node.
fn annotation_docs(tree: &dir::NodeTree, node_id: u32, strings: &StringPool) -> Option<String> {
    let mut docs = Vec::new();
    let annotations = tree.get_annotations(node_id);

    for annotation_id in annotations {
        let annotation = tree.get::<Annotation>(annotation_id);
        let Annotation::Doc { string, .. } = annotation else {
            continue;
        };
        let text = strings.get(*string);
        let content = text.trim();
        if content.is_empty() {
            continue;
        }
        docs.push(content.to_string());
    }

    if docs.is_empty() {
        return None;
    }

    Some(docs.join("\n"))
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
        let Annotation::Decorator { expression, .. } = annotation else {
            continue;
        };

        // resolve decorator shape: @binding(...) is represented as a call expression
        let decorator_expression = *expression;
        let expression = tree.get::<Expression>(decorator_expression);
        let (decorator_expression, arguments) = match expression {
            Expression::Call {
                left,
                dynamic_arguments,
                ..
            } => (*left, Some(dynamic_arguments.as_slice())),
            _ => (decorator_expression, None),
        };

        let decorator_symbol = decorator_symbol_from_expression(tree, decorator_expression)?;
        if decorator_symbol != binding_decorator_symbol {
            continue;
        }

        // resolve decorator arguments
        return Some(decorator_binding_argument(tree, arguments, strings));
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
        scope: spec.scope,
        blocking: spec.blocking,
    }
}

/// Parsed effect options for bindings.
struct BindingEffectSpec {
    /// Effect classification for the binding.
    effect_class: EffectClass,
    /// Replay payload policy for recorded bindings.
    replay_payload: ReplayPayload,
    /// Required platform capabilities for this binding.
    requires: Vec<String>,
    /// Platform scope for this binding.
    scope: BindingScope,
    /// Blocking behavior for this binding.
    blocking: BindingBlocking,
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
    let mut requires = Vec::new();
    let mut scope = None;
    let mut blocking = None;

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
            "requires" => {
                requires = parse_requires_list(tree, *value_id, strings);
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

    if log.is_some() {
        panic!("@binding log is runtime-owned and should not be specified");
    }
    if payload.is_some()
        && !matches!(
            effect_class,
            EffectClass::External {
                replay: ReplayPolicy::Recordable
            }
        )
    {
        panic!("@binding payload requires a recordable external effect");
    }
    if requires.is_empty() {
        panic!("@binding requires at least one capability");
    }

    BindingEffectSpec {
        effect_class,
        replay_payload,
        requires,
        scope,
        blocking,
    }
}

/// Parse a binding scope from a string.
fn parse_binding_scope(value: Option<&str>) -> BindingScope {
    match value {
        Some("os") => BindingScope::Os,
        Some("runtime") => BindingScope::Runtime,
        Some("hybrid") => BindingScope::Hybrid,
        Some(value) => {
            panic!("unsupported @binding scope {value}");
        }
        None => {
            panic!("@binding requires an explicit scope classification");
        }
    }
}

/// Parse a binding blocking behavior from a string.
fn parse_binding_blocking(value: Option<&str>) -> BindingBlocking {
    match value {
        Some("always") => BindingBlocking::Always,
        Some("never") => BindingBlocking::Never,
        Some("sometimes") => BindingBlocking::Sometimes,
        Some(value) => {
            panic!("unsupported @binding blocking value {value}");
        }
        None => {
            panic!("@binding requires an explicit blocking classification");
        }
    }
}

/// Build an effect class from optional effect and replay names.
fn build_effect_class(effect: Option<&str>, replay: &str) -> EffectClass {
    // parse replay policy
    let replay = match replay {
        "recordable" => ReplayPolicy::Recordable,
        "nonrecordable" => ReplayPolicy::NonRecordable,
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
            if replay != ReplayPolicy::NonRecordable {
                panic!("pure bindings must use replay: nonrecordable");
            }
            EffectClass::Pure
        }
        Some("deterministic") => {
            if replay != ReplayPolicy::NonRecordable {
                panic!("deterministic bindings must use replay: nonrecordable");
            }
            EffectClass::Deterministic
        }
        Some("external" | "io") => EffectClass::External { replay },
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

/// Parse required platform capabilities from a decorator value.
fn parse_requires_list(
    tree: &dir::NodeTree,
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

    // parse and normalize capability names
    let mut parsed = Vec::with_capacity(elements.len());
    for argument_id in elements {
        let argument = tree.get::<Argument>(*argument_id);
        let capability_id = argument.value();
        let capability = scalar_string_literal(tree, capability_id, strings).unwrap_or_else(|| {
            panic!("@binding requires must contain only string literals");
        });
        validate_capability_name(&capability);
        parsed.push(capability);
    }

    // ensure deterministic ordering and remove duplicates
    let unique = parsed.into_iter().collect::<BTreeSet<_>>();
    unique.into_iter().collect()
}

/// Validate one capability name in canonical dotted form.
fn validate_capability_name(capability: &str) {
    if capability.trim().is_empty() {
        panic!("@binding requires capability names cannot be empty");
    }

    let segments = capability.split('.').collect::<Vec<_>>();
    if segments.len() < 2 {
        panic!("@binding requires capability names must use dotted hierarchy");
    }

    for segment in segments {
        if segment.is_empty() {
            panic!("@binding requires capability names cannot contain empty segments");
        }

        let mut chars = segment.chars();
        let Some(first) = chars.next() else {
            panic!("@binding requires capability names cannot contain empty segments");
        };
        if !first.is_ascii_lowercase() {
            panic!("@binding requires capability segments must start with lowercase letters");
        }

        for character in chars {
            if character.is_ascii_alphanumeric() || character == '_' {
                continue;
            }
            panic!(
                "@binding requires capability names support only alphanumeric and underscore characters"
            );
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
