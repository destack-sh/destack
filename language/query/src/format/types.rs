use std::collections::HashSet;

use destack_core::StringPool;
use destack_dir as dir;
use destack_workspace::{Module, Package};

use crate::core::ModuleQueryContext;

const DEFAULT_INT_DISPLAY: &str = "int32";
const DEFAULT_FLOAT_DISPLAY: &str = "float64";
const DEFAULT_BOOLEAN_DISPLAY: &str = "boolean";
const DEFAULT_STRING_DISPLAY: &str = "string";
const DEFAULT_BIGINT_DISPLAY: &str = "bigint";
const DEFAULT_CHARACTER_DISPLAY: &str = "character";

/// Format a global type id as a human-readable string.
pub fn format_global_type(ty_id: dir::GlobalTypeId, ctx: &ModuleQueryContext<'_>) -> String {
    let Some(ctx) = ctx.module_context(ty_id.module_id) else {
        return "<missing>".to_string();
    };
    let types = ctx.dir().types();
    let Some(ty) = types.get_type_maybe(ty_id.local_id) else {
        return "<missing>".to_string();
    };

    format_type(ty, types, &ctx)
}

/// Format a type by its id.
pub fn format_local_type(
    ty_id: dir::LocalTypeId,
    types: &dir::TypeTable<'_>,
    ctx: &ModuleQueryContext<'_>,
) -> String {
    let ty = types.get_type(ty_id);
    format_type(ty, types, ctx)
}

/// Format a type as a human-readable string.
pub fn format_type(
    ty: &dir::Type,
    types: &dir::TypeTable<'_>,
    ctx: &ModuleQueryContext<'_>,
) -> String {
    let strings = ctx.dir().strings();

    match ty {
        dir::Type::Error => "<error>".to_string(),
        dir::Type::Never => "never".to_string(),
        dir::Type::Any => "any".to_string(),
        dir::Type::Unknown => "unknown".to_string(),
        dir::Type::Void => "void".to_string(),
        dir::Type::Null => "null".to_string(),
        dir::Type::Undefined => "undefined".to_string(),
        dir::Type::Object => "object".to_string(),
        dir::Type::Primitive(primitive) => format_primitive_type(primitive),
        dir::Type::Literal(literal) => format_scalar_literal(literal, strings),
        dir::Type::Operation(operation) => format_type_operation(operation, types, ctx),
        dir::Type::Parameter(parameter) => format_parameter_type(parameter, ctx),
        dir::Type::This => "this".to_string(),
        dir::Type::Reference(reference) => {
            format_type_reference(reference.symbol, &reference.arguments, types, ctx)
        }
        dir::Type::Member(member) => {
            let owner = format_local_type(member.owner, types, ctx);
            let key = format_static_key(&member.key, strings);
            if member.arguments.is_empty() {
                format!("{owner}.{key}")
            } else {
                let arguments = member
                    .arguments
                    .iter()
                    .map(|argument| format_static_argument(argument, types, ctx))
                    .collect::<Vec<_>>();

                format!("{owner}.{key}<{}>", arguments.join(", "))
            }
        }
        dir::Type::Form(form) => format_form_type(form, types, ctx),
        dir::Type::Dynamic(dynamic) => {
            let constraint = format_local_type(dynamic.constraint, types, ctx);
            format!("Dynamic<{constraint}>")
        }
        dir::Type::Predicate(predicate) => {
            let subject = format_type_predicate_subject(predicate.subject, ctx);
            let target = predicate
                .target
                .map(|target| format_local_type(target, types, ctx));
            match (predicate.asserts, target) {
                (true, Some(target)) => format!("asserts {subject} is {target}"),
                (true, None) => format!("asserts {subject}"),
                (false, Some(target)) => format!("{subject} is {target}"),
                (false, None) => subject,
            }
        }
        dir::Type::FixedArray(array) => {
            let element = format_local_type(array.element, types, ctx);
            let count = format_static_id(array.count, ctx);

            format!("[{element}; {count}]")
        }
        dir::Type::Array(array) => {
            let element = format_local_type(array.element, types, ctx);
            let needs_parens = matches!(types.get_type(array.element), dir::Type::Union(_));
            if needs_parens {
                format!("({element})[]")
            } else {
                format!("{element}[]")
            }
        }
        dir::Type::Range(range) => format_range_type(range, strings),
        dir::Type::Slice(slice) => {
            let element = slice.element;
            let element = format_local_type(element, types, ctx);
            let needs_parens = matches!(types.get_type(slice.element), dir::Type::Union(_));
            if needs_parens {
                format!("({element})[]")
            } else {
                format!("{element}[]")
            }
        }
        dir::Type::Tuple(tuple) => {
            let elements: Vec<_> = tuple
                .elements
                .iter()
                .map(|element| format_type_tuple_element(element, types, ctx))
                .collect();
            format!("({})", elements.join(", "))
        }
        dir::Type::Shape(object) => {
            let mut items: Vec<String> = Vec::new();

            for field in &object.fields {
                let key = format_static_key(&field.key, strings);
                let ty = format_local_type(field.ty, types, ctx);
                let opt = if field.is_optional { "?" } else { "" };
                let readonly = if field.is_readonly { "readonly " } else { "" };
                items.push(format!("{readonly}{key}{opt}: {ty}"));
            }

            for signature in &object.call_signatures {
                let signature = format_local_type(*signature, types, ctx);
                items.push(signature);
            }

            for signature in &object.construct_signatures {
                let signature = format_local_type(*signature, types, ctx);
                items.push(format!("new {signature}"));
            }

            for signature in &object.index_signatures {
                let name = strings.get(signature.name).to_string();
                let key_type = format_local_type(signature.key_type, types, ctx);
                let value_type = format_local_type(signature.value_type, types, ctx);
                let readonly = if signature.is_readonly {
                    "readonly "
                } else {
                    ""
                };
                items.push(format!("{readonly}[{name}: {key_type}]: {value_type}"));
            }

            if items.is_empty() {
                "{}".to_string()
            } else {
                format!("{{ {} }}", items.join(", "))
            }
        }
        dir::Type::Function(function) => {
            let async_str = if function.asynchrony == dir::Asynchrony::Async {
                "async "
            } else {
                ""
            };
            let static_params_str = if function.generic_parameters.is_empty() {
                String::new()
            } else {
                let params: Vec<_> = function
                    .generic_parameters
                    .iter()
                    .map(|p| format_local_type(*p, types, ctx))
                    .collect();
                format!("<{}>", params.join(", "))
            };
            let mut formatted_parameters: Vec<String> = Vec::new();
            if let Some(this_parameter) = function.this_parameter {
                let this_type = format_local_type(this_parameter, types, ctx);
                formatted_parameters.push(format!("this: {this_type}"));
            }
            formatted_parameters.extend(
                function
                    .parameters
                    .iter()
                    .map(|parameter| format_function_parameter(parameter, types, ctx)),
            );
            let ret = if let Some(ret_ty) = function.return_type {
                format!(": {}", format_local_type(ret_ty, types, ctx))
            } else {
                String::new()
            };
            format!(
                "{async_str}{static_params_str}({}){ret}",
                formatted_parameters.join(", ")
            )
        }
        dir::Type::Closure(closure) => {
            let function = format_local_type(closure.function, types, ctx);
            let environment = format_local_type(closure.environment, types, ctx);

            format!("Closure<{function}, {environment}>")
        }
        dir::Type::Union(union) => {
            let mut seen = HashSet::new();
            let mut formatted = Vec::new();
            for element_id in &union.elements {
                if !seen.insert(*element_id) {
                    continue;
                }
                formatted.push(format_local_type(*element_id, types, ctx));
            }
            formatted.join(" | ")
        }
        dir::Type::Intersection(intersection) => {
            let mut seen = HashSet::new();
            let mut formatted = Vec::new();
            for element_id in &intersection.elements {
                if !seen.insert(*element_id) {
                    continue;
                }
                formatted.push(format_local_type(*element_id, types, ctx));
            }
            formatted.join(" & ")
        }
    }
}

/// Format a type-level operation.
pub fn format_type_operation(
    operation: &dir::TypeOperation,
    types: &dir::TypeTable<'_>,
    ctx: &ModuleQueryContext<'_>,
) -> String {
    let strings = ctx.dir().strings();

    match operation {
        dir::TypeOperation::StringMapping { mapping, target } => {
            let target = format_local_type(*target, types, ctx);
            let mapping = format_string_mapping(mapping);

            format!("{mapping}<{target}>")
        }
        dir::TypeOperation::Conditional(conditional) => {
            let left = format_local_type(conditional.left, types, ctx);
            let right = format_local_type(conditional.right, types, ctx);
            let then_type = format_local_type(conditional.then_type, types, ctx);
            let else_type = format_local_type(conditional.else_type, types, ctx);
            format!("{left} extends {right} ? {then_type} : {else_type}")
        }
        dir::TypeOperation::Mapped(mapped) => {
            let name = strings.get(mapped.parameter.name).to_string();
            let constraint = format_local_type(mapped.parameter.constraint, types, ctx);
            let key_remap = mapped
                .parameter
                .key_remap
                .map(|key_remap| format!(" as {}", format_local_type(key_remap, types, ctx)))
                .unwrap_or_default();
            let readonly = format_type_mapped_modifier_prefix(mapped.modifiers.readonly);
            let optional = format_type_mapped_modifier_suffix(mapped.modifiers.optional);
            let value = format_local_type(mapped.value, types, ctx);
            format!("{{ {readonly}[{name} in {constraint}{key_remap}]{optional}: {value} }}")
        }
        dir::TypeOperation::Index(index_type) => {
            let left = format_local_type(index_type.left, types, ctx);
            let index = format_local_type(index_type.index, types, ctx);
            format!("{left}[{index}]")
        }
        dir::TypeOperation::TemplateLiteral(template) => {
            let mut result = String::from("`");
            for (index, string_id) in template.strings.iter().enumerate() {
                result.push_str(strings.get(*string_id));
                if let Some(span_id) = template.spans.get(index) {
                    let span = format_local_type(*span_id, types, ctx);
                    result.push_str("${");
                    result.push_str(&span);
                    result.push('}');
                }
            }
            result.push('`');

            result
        }
        dir::TypeOperation::Infer(infer) => {
            let name = infer.name.map(|name| strings.get(name)).unwrap_or("_");
            let constraint = infer.constraint.map(|constraint| {
                format!(" extends {}", format_local_type(constraint, types, ctx))
            });
            format!("infer {name}{}", constraint.unwrap_or_default())
        }
        dir::TypeOperation::KeyOf(unary) => {
            let target_type = format_local_type(unary.target, types, ctx);
            format!("keyof {target_type}")
        }
    }
}

/// Format a PrimitiveType.
pub fn format_primitive_type(prim: &dir::PrimitiveType) -> String {
    match prim {
        dir::PrimitiveType::Boolean => "boolean".to_string(),
        dir::PrimitiveType::Character => "char".to_string(),
        dir::PrimitiveType::String => "string".to_string(),
        dir::PrimitiveType::Bigint => "bigint".to_string(),
        dir::PrimitiveType::Integer(int_type) => int_type.as_str(),
        dir::PrimitiveType::Float(float_type) => float_type.as_str().to_string(),
        dir::PrimitiveType::Symbol => "symbol".to_string(),
        dir::PrimitiveType::UniqueSymbol => "unique symbol".to_string(),
    }
}

/// Format a compact scalar interval type.
pub fn format_range_type(range: &dir::RangeType, strings: &StringPool) -> String {
    let start = range
        .start
        .as_ref()
        .map(|start| format_scalar_literal(start, strings))
        .unwrap_or_default();
    let end = range
        .end
        .as_ref()
        .map(|end| format_scalar_literal(end, strings))
        .unwrap_or_default();
    let separator = if range.is_inclusive { "..=" } else { ".." };

    format!("{start}{separator}{end}")
}

/// Format a canonical memory or access form.
pub fn format_form_type(
    form: &dir::FormType,
    types: &dir::TypeTable<'_>,
    ctx: &ModuleQueryContext<'_>,
) -> String {
    let value = format_local_type(form.value, types, ctx);

    match &form.form {
        dir::Form::Managed => format!("Managed<{value}>"),
        dir::Form::Owned => format!("Owned<{value}>"),
        dir::Form::Raw => format!("Raw<{value}>"),
        dir::Form::Readonly => format!("Readonly<{value}>"),
        dir::Form::Placed { place } => {
            let place = format_static_id(*place, ctx);
            format!("Placed<{value}, {place}>")
        }
        dir::Form::Borrowed { lifetime, access } => {
            let lifetime = format_static_id(*lifetime, ctx);
            let access = format_static_id(*access, ctx);
            format!("Borrowed<{value}, {lifetime}, {access}>")
        }
    }
}

/// Format a ScalarLiteral.
pub fn format_scalar_literal(scalar: &dir::ScalarLiteral, strings: &StringPool) -> String {
    match scalar {
        dir::ScalarLiteral::Null => "null".to_string(),
        dir::ScalarLiteral::Boolean(b) => b.to_string(),
        dir::ScalarLiteral::Integer(i) => i.to_string(),
        dir::ScalarLiteral::Bigint(i) => format!("{i}n"),
        dir::ScalarLiteral::Float(f) => {
            // ensure float has decimal point for clarity
            let s = f.to_string();
            if s.contains('.') || s.contains('e') || s.contains('E') {
                s
            } else {
                format!("{s}.0")
            }
        }
        dir::ScalarLiteral::Character(c) => format!("'{c}'"),
        dir::ScalarLiteral::String(string_id) => {
            let s = strings.get(*string_id);
            format!("\"{s}\"")
        }
        dir::ScalarLiteral::RegexString { content, flags } => {
            let content_str = strings.get(*content).to_string();
            if let Some(flags_id) = flags {
                let flags_str = strings.get(*flags_id).to_string();
                format!("/{content_str}/{flags_str}")
            } else {
                format!("/{content_str}/")
            }
        }
    }
}

/// Format a type for inlay hints.
pub fn format_inlay_type(
    ty: &dir::Type,
    types: &dir::TypeTable<'_>,
    ctx: &ModuleQueryContext<'_>,
) -> String {
    // widen scalar literal types to their default primitive display types
    if let dir::Type::Literal(value) = ty {
        let widened = widened_scalar_literal_name(value);
        return widened.to_string();
    }

    // otherwise, format the type as usual
    format_type(ty, types, ctx)
}

/// Map a scalar literal type to its default primitive display name.
pub fn widened_scalar_literal_name(value: &dir::ScalarLiteral) -> &'static str {
    match value {
        dir::ScalarLiteral::Null => "null",
        dir::ScalarLiteral::Boolean(_) => DEFAULT_BOOLEAN_DISPLAY,
        dir::ScalarLiteral::String(_) | dir::ScalarLiteral::RegexString { .. } => {
            DEFAULT_STRING_DISPLAY
        }
        dir::ScalarLiteral::Integer(_) => DEFAULT_INT_DISPLAY,
        dir::ScalarLiteral::Float(_) => DEFAULT_FLOAT_DISPLAY,
        dir::ScalarLiteral::Bigint(_) => DEFAULT_BIGINT_DISPLAY,
        dir::ScalarLiteral::Character(_) => DEFAULT_CHARACTER_DISPLAY,
    }
}

/// Format a type reference (symbol with optional static arguments).
pub fn format_type_reference(
    symbol: dir::GlobalSymbolId,
    generic_arguments: &[dir::StaticArgument],
    types: &dir::TypeTable<'_>,
    ctx: &ModuleQueryContext<'_>,
) -> String {
    let name = format_symbol_name(symbol, ctx);
    if generic_arguments.is_empty() {
        name
    } else {
        let argument_strs: Vec<_> = generic_arguments
            .iter()
            .map(|argument| format_static_argument(argument, types, ctx))
            .collect();
        format!("{name}<{}>", argument_strs.join(", "))
    }
}

/// Format one generic parameter reference.
pub fn format_parameter_type(
    parameter: &dir::GenericParameterRef,
    ctx: &ModuleQueryContext<'_>,
) -> String {
    match parameter.key {
        dir::GenericSlotKey::Symbol(symbol) => format_symbol_name(symbol, ctx),
        dir::GenericSlotKey::Generated(name) => {
            let Some(ctx) = ctx.module_context(parameter.owner.module_id) else {
                return "<unknown>".to_string();
            };

            ctx.dir().strings().get(name).to_string()
        }
    }
}

/// Get the name of a symbol from any module.
pub fn format_symbol_name(symbol_id: dir::GlobalSymbolId, ctx: &ModuleQueryContext<'_>) -> String {
    let Some(ctx) = ctx.module_context(symbol_id.module_id) else {
        return "<unknown>".to_string();
    };
    let dir = ctx.dir();
    let symbols = dir.symbols();
    let symbol = symbols.get_symbol(symbol_id.into_local());
    if let Some(name_id) = symbol.name() {
        dir.strings().get(name_id).to_string()
    } else {
        "<anonymous>".to_string()
    }
}

/// Get the symbol path for a symbol within its module.
pub fn format_symbol_path(
    symbol_id: dir::GlobalSymbolId,
    ctx: &ModuleQueryContext<'_>,
) -> Option<String> {
    // load the module symbols
    let ctx = ctx.module_context(symbol_id.module_id)?;
    let dir = ctx.dir();
    let symbols = dir.symbols();

    // seed with the symbol name
    let symbol = symbols.get_symbol(symbol_id.into_local());
    let symbol_name = static_key_segment(symbol.key, dir.strings())?;
    let mut segments = vec![symbol_name];

    // walk owner scopes for namespaces and types
    let mut scope_id = symbol.scope.id;
    let mut seen_scopes = HashSet::new();
    loop {
        // avoid cycles in scope ownership
        if !seen_scopes.insert(scope_id) {
            break;
        }

        // collect named owners into the path
        let scope = symbols.get_scope_by_id(scope_id);
        if let Some(owner_id) = scope.owner
            && owner_id != symbol_id.into_local()
        {
            let owner = symbols.get_symbol(owner_id);
            let owner_name = static_key_segment(owner.key, dir.strings())?;
            segments.push(owner_name);
        }

        // climb to the parent scope
        let Some(parent) = scope.parent else {
            break;
        };
        scope_id = parent.id;
    }

    // reverse for root to leaf order
    segments.reverse();
    Some(segments.join("."))
}

/// Get the qualified name of a symbol with module prefix.
pub fn format_symbol_qualified_name(
    symbol_id: dir::GlobalSymbolId,
    ctx: &ModuleQueryContext<'_>,
) -> Option<String> {
    // resolve the owning module and package
    let module = ctx
        .repository()
        .module(ctx.revision(), symbol_id.module_id)
        .ok()
        .flatten()?;
    let package = ctx
        .repository()
        .package(ctx.revision(), module.package_id)
        .ok()
        .flatten()?;

    // resolve package and module path
    let package_name = package.name.as_ref()?;
    if package_name.is_empty() {
        return None;
    }
    let module_path = module_path_without_extension(module.as_ref(), package.as_ref())?;

    // resolve symbol path
    let symbol_path = format_symbol_path(symbol_id, ctx)?;
    let module_prefix = if module_path.is_empty() {
        package_name.to_string()
    } else {
        format!("{package_name}/{module_path}")
    };

    Some(format!("{module_prefix}:{symbol_path}"))
}

/// Get the qualified name of a unique symbol.
pub fn format_unique_symbol_qualified_name(
    symbol_id: dir::GlobalSymbolId,
    ctx: &ModuleQueryContext<'_>,
) -> Option<String> {
    let name = format_symbol_qualified_name(symbol_id, ctx)?;
    Some(format!("{name}#unique"))
}

/// Resolve the package relative module path without extension.
fn module_path_without_extension(module: &Module, package: &Package) -> Option<String> {
    // prefer package relative paths when available
    let module_path = if let Some(path) = &module.path {
        let relative = package
            .path
            .as_ref()
            .and_then(|package_path| path.strip_prefix(package_path).ok())
            .unwrap_or(path);
        relative.to_string_lossy().to_string()
    } else {
        module.uri.to_string()
    };

    // normalize separators and drop extension
    let module_path = normalize_path_separators(&module_path);
    Some(strip_extension_from_path(&module_path))
}

/// Normalize a module path to use forward slashes.
fn normalize_path_separators(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    normalized.trim_start_matches('/').to_string()
}

/// Strip the file extension from a module path.
fn strip_extension_from_path(path: &str) -> String {
    let (prefix, leaf) = path.rsplit_once('/').unwrap_or(("", path));
    let stripped = leaf.rsplit_once('.').map(|(base, _)| base).unwrap_or(leaf);
    if prefix.is_empty() {
        stripped.to_string()
    } else {
        format!("{prefix}/{stripped}")
    }
}

/// Convert a static key into a symbol path segment.
fn static_key_segment(key: Option<dir::StaticKey>, strings: &StringPool) -> Option<String> {
    let key = key?;
    match key {
        dir::StaticKey::Name(name_id) => Some(strings.get(name_id).to_string()),
        dir::StaticKey::Index(index) => Some(index.to_string()),
        dir::StaticKey::Symbol(_) => None,
    }
}

/// Format a StaticKey.
pub fn format_static_key(key: &dir::StaticKey, strings: &StringPool) -> String {
    match key {
        dir::StaticKey::Name(name_id) => strings.get(*name_id).to_string(),
        dir::StaticKey::Index(index) => index.to_string(),
        dir::StaticKey::Symbol(symbol) => format_symbol_key(symbol, strings),
    }
}

/// Format a SymbolKey.
pub fn format_symbol_key(key: &dir::SymbolKey, strings: &StringPool) -> String {
    match key {
        dir::SymbolKey::Unique(_) => "<unique symbol>".to_string(),
        dir::SymbolKey::Registry(name_id) => {
            let name = strings.get(*name_id);
            format!("[Symbol.for(\"{name}\")]")
        }
    }
}

/// Format a StaticArgument.
pub fn format_static_argument(
    argument: &dir::StaticArgument,
    _types: &dir::TypeTable<'_>,
    ctx: &ModuleQueryContext<'_>,
) -> String {
    let strings = ctx.dir().strings();

    let value = format_static_id(argument.value, ctx);
    if let Some(name_id) = argument.name {
        let name = strings.get(name_id);
        format!("{name}: {value}")
    } else {
        value
    }
}

/// Format a static value by its id.
pub fn format_static_id(static_id: dir::LocalStaticId, ctx: &ModuleQueryContext<'_>) -> String {
    let dir = ctx.dir();
    let term = dir.statics().get_static(static_id);

    format_static_term(term, dir.types(), ctx)
}

/// Format a StaticTerm.
pub fn format_static_term(
    term: &dir::StaticTerm,
    types: &dir::TypeTable<'_>,
    ctx: &ModuleQueryContext<'_>,
) -> String {
    let strings = ctx.dir().strings();

    match term {
        dir::StaticTerm::Parameter(parameter) => format_parameter_type(parameter, ctx),
        dir::StaticTerm::ScalarLiteral { value } => format_scalar_literal(value, strings),
        dir::StaticTerm::Symbol { symbol } => format_symbol_name(*symbol, ctx),
        dir::StaticTerm::Access { access } => format!("{access:?}").to_lowercase(),
        dir::StaticTerm::Space { space } => format!("{space:?}").to_lowercase(),
        dir::StaticTerm::Place { place } => format_place(place),
        dir::StaticTerm::Lifetime { lifetime } => format_lifetime(lifetime, ctx),
        dir::StaticTerm::Union { elements } => elements
            .iter()
            .map(|element| format_static_id(*element, ctx))
            .collect::<Vec<_>>()
            .join(" | "),
        dir::StaticTerm::TypeLiteral { value } => format_source_type_literal(value, strings),
        dir::StaticTerm::Declaration { .. } => "<declaration>".to_string(),
        dir::StaticTerm::Type { ty } => {
            let ty = types.get_type(*ty);
            format_type(ty, types, ctx)
        }
        dir::StaticTerm::Array { elements } => {
            let elements: Vec<_> = elements
                .iter()
                .map(|e| format_static_term(e, types, ctx))
                .collect();
            format!("[{}]", elements.join(", "))
        }
        dir::StaticTerm::FixedArray { value, length } => {
            let value = format_static_term(value, types, ctx);
            let length = format_static_term(length, types, ctx);
            format!("[{value}; {length}]")
        }
        dir::StaticTerm::Tuple { elements } => {
            let elements: Vec<_> = elements
                .iter()
                .map(|e| format_static_term(e, types, ctx))
                .collect();
            format!("({})", elements.join(", "))
        }
        dir::StaticTerm::Object { properties } => {
            let properties = format_static_properties(properties, types, ctx);
            format!("{{{properties}}}")
        }
        dir::StaticTerm::Struct { ty, properties } => {
            let ty = format_local_type(*ty, types, ctx);
            let properties = format_static_properties(properties, types, ctx);
            format!("{ty} {{{properties}}}")
        }
    }
}

/// Format a normalized place value.
fn format_place(place: &dir::Place) -> String {
    match place {
        dir::Place::Ambient => "ambient".to_string(),
        dir::Place::Space(space) => format!("{space:?}").to_lowercase(),
    }
}

/// Format a normalized lifetime value.
fn format_lifetime(lifetime: &dir::Lifetime, ctx: &ModuleQueryContext<'_>) -> String {
    match lifetime {
        dir::Lifetime::Static => "static".to_string(),
        dir::Lifetime::Symbol(symbol) => format_symbol_name(*symbol, ctx),
    }
}

/// Format static object properties.
fn format_static_properties(
    properties: &[dir::StaticProperty],
    types: &dir::TypeTable<'_>,
    ctx: &ModuleQueryContext<'_>,
) -> String {
    properties
        .iter()
        .map(|property| format_static_property(property, types, ctx))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Format one static object property.
fn format_static_property(
    property: &dir::StaticProperty,
    types: &dir::TypeTable<'_>,
    ctx: &ModuleQueryContext<'_>,
) -> String {
    let strings = ctx.dir().strings();

    match property {
        dir::StaticProperty::Field { key, value } => {
            let key = format_static_key(key, strings);
            let value = format_static_term(value, types, ctx);
            format!("{key}: {value}")
        }
        dir::StaticProperty::Method { key, .. } => {
            let key = key
                .map(|key| format_static_key(&key, strings))
                .unwrap_or_else(|| "<call>".to_string());
            format!("{key}()")
        }
        dir::StaticProperty::Spread { value } => {
            let value = format_static_term(value, types, ctx);
            format!("...{value}")
        }
    }
}

fn format_string_mapping(function: &dir::StringMapping) -> String {
    match function {
        dir::StringMapping::Uppercase => "Uppercase".to_string(),
        dir::StringMapping::Lowercase => "Lowercase".to_string(),
        dir::StringMapping::Capitalize => "Capitalize".to_string(),
        dir::StringMapping::Uncapitalize => "Uncapitalize".to_string(),
    }
}

fn format_type_mapped_modifier_prefix(modifier: dir::MappedTypeModifier) -> &'static str {
    match modifier {
        dir::MappedTypeModifier::Present => "readonly ",
        dir::MappedTypeModifier::Add => "+readonly ",
        dir::MappedTypeModifier::Remove => "-readonly ",
        dir::MappedTypeModifier::None => "",
    }
}

fn format_type_mapped_modifier_suffix(modifier: dir::MappedTypeModifier) -> &'static str {
    match modifier {
        dir::MappedTypeModifier::Present => "?",
        dir::MappedTypeModifier::Add => "+?",
        dir::MappedTypeModifier::Remove => "-?",
        dir::MappedTypeModifier::None => "",
    }
}

/// Format a parsed type literal.
fn format_source_type_literal(lit: &dir::TypeLiteral, _strings: &StringPool) -> String {
    match lit {
        dir::TypeLiteral::Never => "never".to_string(),
        dir::TypeLiteral::Any => "any".to_string(),
        dir::TypeLiteral::Undefined => "undefined".to_string(),
        dir::TypeLiteral::Unknown => "unknown".to_string(),
        dir::TypeLiteral::Object => "object".to_string(),
        dir::TypeLiteral::Void => "void".to_string(),
        dir::TypeLiteral::Null => "null".to_string(),
        dir::TypeLiteral::Boolean => DEFAULT_BOOLEAN_DISPLAY.to_string(),
        dir::TypeLiteral::Character => DEFAULT_CHARACTER_DISPLAY.to_string(),
        dir::TypeLiteral::String => DEFAULT_STRING_DISPLAY.to_string(),
        dir::TypeLiteral::Bigint => DEFAULT_BIGINT_DISPLAY.to_string(),
        dir::TypeLiteral::Number => "number".to_string(),
        dir::TypeLiteral::Integer(integer) => integer.as_str(),
        dir::TypeLiteral::Float(float) => float.as_str().to_string(),
        dir::TypeLiteral::Symbol => "symbol".to_string(),
        dir::TypeLiteral::UniqueSymbol => "unique symbol".to_string(),
    }
}

fn format_type_tuple_element(
    element: &dir::TypeElement,
    types: &dir::TypeTable<'_>,
    ctx: &ModuleQueryContext<'_>,
) -> String {
    let strings = ctx.dir().strings();

    let mut result = String::new();
    if element.is_readonly {
        result.push_str("readonly ");
    }
    if element.is_rest {
        result.push_str("...");
    }
    if let Some(label) = element.label {
        let name = strings.get(label);
        let ty = format_local_type(element.ty, types, ctx);
        result.push_str(&format!("{name}: {ty}"));
    } else {
        result.push_str(&format_local_type(element.ty, types, ctx));
    }
    if element.is_optional {
        result.push('?');
    }
    result
}

fn format_function_parameter(
    parameter: &dir::FunctionParameterType,
    types: &dir::TypeTable<'_>,
    ctx: &ModuleQueryContext<'_>,
) -> String {
    let mut result = String::new();
    if parameter.is_rest {
        result.push_str("...");
    }
    result.push_str(&format_local_type(parameter.ty, types, ctx));
    if parameter.is_optional {
        result.push('?');
    }

    result
}

fn format_type_predicate_subject(
    subject: dir::PredicateSubject,
    ctx: &ModuleQueryContext<'_>,
) -> String {
    match subject {
        dir::PredicateSubject::This => "this".to_string(),
        dir::PredicateSubject::Symbol(symbol_id) => format_symbol_name(symbol_id, ctx),
    }
}
