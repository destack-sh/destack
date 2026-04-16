use std::collections::HashSet;

use destack_core::StringPool;
use destack_dir as dir;
use destack_source::ProfileId;
use destack_workspace::{Module, Package, Repository, Revision};

const DEFAULT_INT_DISPLAY: &str = "int32";
const DEFAULT_FLOAT_DISPLAY: &str = "float64";
const DEFAULT_BOOLEAN_DISPLAY: &str = "boolean";
const DEFAULT_STRING_DISPLAY: &str = "string";
const DEFAULT_BIGINT_DISPLAY: &str = "bigint";
const DEFAULT_CHARACTER_DISPLAY: &str = "character";

/// Format a global type id as a human-readable string.
pub fn format_global_type(
    ty_id: dir::GlobalTypeId,
    repository: &Repository,
    revision: Revision,
    strings: &StringPool,
    profile: ProfileId,
) -> String {
    let Some(dir) = repository.dir_analyzed(revision, ty_id.module_id, profile) else {
        return "<missing>".to_string();
    };
    let Some(ty) = dir.types.get_type_maybe(ty_id.local_id) else {
        return "<missing>".to_string();
    };

    format_type(ty, &dir.types, repository, revision, strings)
}

/// Format a type by its id.
pub fn format_local_type(
    ty_id: dir::LocalTypeId,
    types: &dir::TypeTable,
    repository: &Repository,
    revision: Revision,
    strings: &StringPool,
) -> String {
    let ty = types.get_type(ty_id);
    format_type(ty, types, repository, revision, strings)
}

/// Format a type as a human-readable string.
pub fn format_type(
    ty: &dir::Type,
    types: &dir::TypeTable,
    repository: &Repository,
    revision: Revision,
    strings: &StringPool,
) -> String {
    match ty {
        dir::Type::TypeLiteral { value } => format_type_literal(value, strings),
        dir::Type::Value { value } => {
            format!(
                "type {}",
                format_local_type(*value, types, repository, revision, strings)
            )
        }
        dir::Type::This => "this".to_string(),
        dir::Type::Reference {
            symbol,
            static_arguments,
        } => format_type_reference(
            *symbol,
            static_arguments.as_deref(),
            types,
            repository,
            revision,
            strings,
        ),
        dir::Type::Unevaluated(_) => "<unevaluated>".to_string(),
        dir::Type::InferVar { id } => format!("<infer {}>", id.0),
        dir::Type::Conditional {
            distributive_symbol: _,
            left,
            right,
            then_type,
            else_type,
        } => {
            let left = format_local_type(*left, types, repository, revision, strings);
            let right = format_local_type(*right, types, repository, revision, strings);
            let then_type = format_local_type(*then_type, types, repository, revision, strings);
            let else_type = format_local_type(*else_type, types, repository, revision, strings);
            format!("{left} extends {right} ? {then_type} : {else_type}")
        }
        dir::Type::Mapped {
            parameter,
            modifiers,
            value,
        } => {
            let name = strings.get(parameter.name).to_string();
            let constraint =
                format_local_type(parameter.constraint, types, repository, revision, strings);
            let key_remap = parameter
                .key_remap
                .map(|key_remap| {
                    format!(
                        " as {}",
                        format_local_type(key_remap, types, repository, revision, strings)
                    )
                })
                .unwrap_or_default();
            let readonly = format_type_mapped_modifier_prefix(modifiers.readonly);
            let optional = format_type_mapped_modifier_suffix(modifiers.optional);
            let value = format_local_type(*value, types, repository, revision, strings);
            format!("{{ {readonly}[{name} in {constraint}{key_remap}]{optional}: {value} }}")
        }
        dir::Type::Index { left, index } => {
            let left = format_local_type(*left, types, repository, revision, strings);
            let index = format_local_type(*index, types, repository, revision, strings);
            format!("{left}[{index}]")
        }
        dir::Type::TemplateLiteral {
            strings: template_strings,
            spans,
        } => {
            let mut result = String::from("`");
            for (index, string_id) in template_strings.iter().enumerate() {
                result.push_str(&strings.get(*string_id));
                if let Some(span_id) = spans.get(index) {
                    let span = format_local_type(*span_id, types, repository, revision, strings);
                    result.push_str("${");
                    result.push_str(&span);
                    result.push('}');
                }
            }
            result.push('`');
            result
        }
        dir::Type::Import {
            target,
            qualifier,
            static_arguments,
        } => {
            let target = strings.get(*target);
            let mut result = format!("import(\"{}\")", target.as_ref());
            if let Some(qualifier) = qualifier {
                let path = format_path(qualifier, strings);
                result.push('.');
                result.push_str(&path);
            }
            if let Some(static_arguments) = static_arguments {
                let formatted_arguments = static_arguments
                    .iter()
                    .map(|argument| {
                        format_static_argument(argument, types, repository, revision, strings)
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                result.push('<');
                result.push_str(&formatted_arguments);
                result.push('>');
            }
            result
        }
        dir::Type::Infer { name, constraint } => {
            let name = strings.get(*name);
            let constraint = constraint.map(|constraint| {
                format!(
                    " extends {}",
                    format_local_type(constraint, types, repository, revision, strings)
                )
            });
            format!("infer {}{}", name.as_ref(), constraint.unwrap_or_default())
        }
        dir::Type::Predicate {
            asserts,
            subject,
            target,
        } => {
            let subject = format_type_predicate_subject(*subject, repository, revision, strings);
            let target = target
                .map(|target| format_local_type(target, types, repository, revision, strings));
            match (asserts, target) {
                (true, Some(target)) => format!("asserts {subject} is {target}"),
                (true, None) => format!("asserts {subject}"),
                (false, Some(target)) => format!("{subject} is {target}"),
                (false, None) => subject,
            }
        }
        dir::Type::Readonly { target_type } => {
            let target_type = format_local_type(*target_type, types, repository, revision, strings);
            format!("readonly {target_type}")
        }
        dir::Type::KeyOf { target_type } => {
            let target_type = format_local_type(*target_type, types, repository, revision, strings);
            format!("keyof {target_type}")
        }
        dir::Type::Must { target_type } => {
            let target_type = format_local_type(*target_type, types, repository, revision, strings);
            format!("{target_type}!")
        }
        dir::Type::AsComptime { target_type } => {
            let target_type = format_local_type(*target_type, types, repository, revision, strings);
            format!("{target_type} as comptime")
        }
        dir::Type::Not { target_type } => {
            let target_type = format_local_type(*target_type, types, repository, revision, strings);
            format!("!{target_type}")
        }
        dir::Type::ValueOf {
            mutability,
            variance,
            right,
        } => {
            let right_str = format_local_type(*right, types, repository, revision, strings);
            let mut result = String::from("^");
            if let Some(m) = mutability {
                result.push_str(match m {
                    dir::Mutability::Mutable => "mut ",
                    dir::Mutability::Immutable => "const ",
                });
            }
            if let Some(v) = variance {
                result.push_str(&format!("{v:?} ").to_lowercase());
            }
            result.push_str(&right_str);
            result
        }
        dir::Type::ReferenceOf {
            mutability,
            variance,
            right,
        } => {
            let right_str = format_local_type(*right, types, repository, revision, strings);
            let mut result = String::from("&");
            if let Some(m) = mutability {
                result.push_str(match m {
                    dir::Mutability::Mutable => "mut ",
                    dir::Mutability::Immutable => "const ",
                });
            }
            if let Some(v) = variance {
                result.push_str(&format!("{v:?} ").to_lowercase());
            }
            result.push_str(&right_str);
            result
        }
        dir::Type::PointerOf { mutability, right } => {
            let right_str = format_local_type(*right, types, repository, revision, strings);
            let mut result = String::from("*");
            if let Some(m) = mutability {
                result.push_str(match m {
                    dir::Mutability::Mutable => "mut ",
                    dir::Mutability::Immutable => "const ",
                });
            }
            result.push_str(&right_str);
            result
        }
        dir::Type::In { left, right } => {
            let left = format_local_type(*left, types, repository, revision, strings);
            let right = format_local_type(*right, types, repository, revision, strings);
            format!("{left} in {right}")
        }
        dir::Type::Extends { left, right } => {
            let left = format_local_type(*left, types, repository, revision, strings);
            let right = format_local_type(*right, types, repository, revision, strings);
            format!("{left} extends {right}")
        }
        dir::Type::Implements { left, right } => {
            let left = format_local_type(*left, types, repository, revision, strings);
            let right = format_local_type(*right, types, repository, revision, strings);
            format!("{left} implements {right}")
        }
        dir::Type::ArraySized {
            element,
            count: _,
            is_readonly,
        } => {
            let elem_str = format_local_type(*element, types, repository, revision, strings);
            let needs_parens = matches!(types.get_type(*element), dir::Type::Union { .. });
            let readonly_prefix = if *is_readonly { "readonly " } else { "" };
            if needs_parens {
                format!("{readonly_prefix}({elem_str})[]")
            } else {
                format!("{readonly_prefix}{elem_str}[]")
            }
        }
        dir::Type::Array {
            element,
            is_readonly,
        } => {
            let readonly_prefix = if *is_readonly { "readonly " } else { "" };
            if let Some(elem) = element {
                let elem_str = format_local_type(*elem, types, repository, revision, strings);
                let needs_parens = matches!(types.get_type(*elem), dir::Type::Union { .. });
                if needs_parens {
                    format!("{readonly_prefix}({elem_str})[]")
                } else {
                    format!("{readonly_prefix}{elem_str}[]")
                }
            } else {
                format!("{readonly_prefix}[]")
            }
        }
        dir::Type::Tuple {
            elements,
            is_readonly,
        } => {
            let elements: Vec<_> = elements
                .iter()
                .map(|element| {
                    format_type_tuple_element(element, types, repository, revision, strings)
                })
                .collect();
            let readonly_prefix = if *is_readonly { "readonly " } else { "" };
            format!("{readonly_prefix}({})", elements.join(", "))
        }
        dir::Type::Object {
            fields,
            call_signatures,
            construct_signatures,
            index_signatures,
        } => {
            let mut items: Vec<String> = Vec::new();

            for field in fields {
                let key = format_static_key(&field.key, strings);
                let ty = format_local_type(field.ty, types, repository, revision, strings);
                let opt = if field.is_optional { "?" } else { "" };
                let readonly = if field.is_readonly { "readonly " } else { "" };
                items.push(format!("{readonly}{key}{opt}: {ty}"));
            }

            for signature in call_signatures {
                let signature = format_local_type(*signature, types, repository, revision, strings);
                items.push(signature);
            }

            for signature in construct_signatures {
                let signature = format_local_type(*signature, types, repository, revision, strings);
                items.push(format!("new {signature}"));
            }

            for signature in index_signatures {
                let name = strings.get(signature.name).to_string();
                let key_type =
                    format_local_type(signature.key_type, types, repository, revision, strings);
                let value_type =
                    format_local_type(signature.value_type, types, repository, revision, strings);
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
        dir::Type::Function {
            asynchrony,
            cardinality: _,
            static_parameters,
            this_parameter,
            dynamic_parameters,
            return_type,
        } => {
            let async_str = if *asynchrony == dir::Asynchrony::Async {
                "async "
            } else {
                ""
            };
            let static_params_str = if static_parameters.is_empty() {
                String::new()
            } else {
                let params: Vec<_> = static_parameters
                    .iter()
                    .map(|p| format_local_type(*p, types, repository, revision, strings))
                    .collect();
                format!("<{}>", params.join(", "))
            };
            let mut dynamic_params: Vec<String> = Vec::new();
            if let Some(this_parameter) = this_parameter {
                let this_type =
                    format_local_type(*this_parameter, types, repository, revision, strings);
                dynamic_params.push(format!("this: {this_type}"));
            }
            dynamic_params.extend(
                dynamic_parameters
                    .iter()
                    .map(|p| format_local_type(*p, types, repository, revision, strings)),
            );
            let ret = if let Some(ret_ty) = return_type {
                format!(
                    ": {}",
                    format_local_type(*ret_ty, types, repository, revision, strings)
                )
            } else {
                String::new()
            };
            format!(
                "{async_str}{static_params_str}({}){ret}",
                dynamic_params.join(", ")
            )
        }
        dir::Type::Union { elements } => {
            let mut seen = HashSet::new();
            let mut formatted = Vec::new();
            for element_id in elements {
                if !seen.insert(*element_id) {
                    continue;
                }
                formatted.push(format_local_type(
                    *element_id,
                    types,
                    repository,
                    revision,
                    strings,
                ));
            }
            formatted.join(" | ")
        }
        dir::Type::Intersection { elements } => {
            let mut seen = HashSet::new();
            let mut formatted = Vec::new();
            for element_id in elements {
                if !seen.insert(*element_id) {
                    continue;
                }
                formatted.push(format_local_type(
                    *element_id,
                    types,
                    repository,
                    revision,
                    strings,
                ));
            }
            formatted.join(" & ")
        }
        dir::Type::Error => "<error>".to_string(),
    }
}

/// Format a TypeLiteral.
pub fn format_type_literal(lit: &dir::TypeLiteral, strings: &StringPool) -> String {
    match lit {
        dir::TypeLiteral::Never => "never".to_string(),
        dir::TypeLiteral::Any => "any".to_string(),
        dir::TypeLiteral::Infer => "_".to_string(),
        dir::TypeLiteral::Undefined => "undefined".to_string(),
        dir::TypeLiteral::Unknown => "unknown".to_string(),
        dir::TypeLiteral::Object => "object".to_string(),
        dir::TypeLiteral::Void => "void".to_string(),
        dir::TypeLiteral::Null => "null".to_string(),
        dir::TypeLiteral::Primitive(p) => format_primitive_type(p),
        dir::TypeLiteral::Intrinsic(intrinsic) => format_type_intrinsic(intrinsic),
        dir::TypeLiteral::ScalarLiteral(s) => format_scalar_literal(s, strings),
    }
}

/// Format a PrimitiveType.
pub fn format_primitive_type(prim: &dir::PrimitiveType) -> String {
    match prim {
        dir::PrimitiveType::Boolean => "boolean".to_string(),
        dir::PrimitiveType::Character => "char".to_string(),
        dir::PrimitiveType::String => "string".to_string(),
        dir::PrimitiveType::Bigint => "bigint".to_string(),
        dir::PrimitiveType::Number => "number".to_string(),
        dir::PrimitiveType::Int(int_type) => int_type.as_str(),
        dir::PrimitiveType::Float(float_type) => float_type.as_str(),
        dir::PrimitiveType::Symbol => "symbol".to_string(),
        dir::PrimitiveType::UniqueSymbol => "unique symbol".to_string(),
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
            let s = &*strings.get(*string_id);
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
pub fn format_type_for_inlay_hint(
    ty: &dir::Type,
    types: &dir::TypeTable,
    repository: &Repository,
    revision: Revision,
    strings: &StringPool,
) -> String {
    // widen scalar literal types to their default primitive display types
    if let dir::Type::TypeLiteral {
        value: dir::TypeLiteral::ScalarLiteral(value),
    } = ty
    {
        let widened = widened_scalar_literal_name(value);
        return widened.to_string();
    }

    // otherwise, format the type as usual
    format_type(ty, types, repository, revision, strings)
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
    static_arguments: Option<&[dir::StaticArgument]>,
    types: &dir::TypeTable,
    repository: &Repository,
    revision: Revision,
    strings: &StringPool,
) -> String {
    let name = format_symbol_name(symbol, repository, revision, strings);
    if let Some(arguments) = static_arguments
        && !arguments.is_empty()
    {
        let argument_strs: Vec<_> = arguments
            .iter()
            .map(|argument| format_static_argument(argument, types, repository, revision, strings))
            .collect();
        format!("{name}<{}>", argument_strs.join(", "))
    } else {
        name
    }
}

/// Get the name of a symbol from any module.
pub fn format_symbol_name(
    symbol_id: dir::GlobalSymbolId,
    repository: &Repository,
    revision: Revision,
    strings: &StringPool,
) -> String {
    let Some(dir) = repository.dir_base(revision, symbol_id.module_id) else {
        return "<unknown>".to_string();
    };
    let symbols = &dir.symbols;
    let symbol = symbols.get_symbol(symbol_id.into_local());
    if let Some(name_id) = symbol.name() {
        strings.get(name_id).to_string()
    } else {
        "<anonymous>".to_string()
    }
}

/// Get the symbol path for a symbol within its module.
pub fn format_symbol_path(
    symbol_id: dir::GlobalSymbolId,
    repository: &Repository,
    revision: Revision,
    strings: &StringPool,
) -> Option<String> {
    // load the module symbols
    let dir = repository.dir_base(revision, symbol_id.module_id)?;
    let symbols = &dir.symbols;

    // seed with the symbol name
    let symbol = symbols.get_symbol(symbol_id.into_local());
    let symbol_name = static_key_segment(symbol.key, strings)?;
    let mut segments = vec![symbol_name];

    // walk owner scopes for namespaces and types
    let mut scope_id = symbol.scope.0;
    let mut seen_scopes = HashSet::new();
    loop {
        // avoid cycles in scope ownership
        if !seen_scopes.insert(scope_id) {
            break;
        }

        // collect named owners into the path
        let scope = symbols.get_scope_by_id(scope_id);
        if let Some(owner_id) = scope.owner_id
            && owner_id != symbol_id.into_local()
        {
            let owner = symbols.get_symbol(owner_id);
            let owner_name = static_key_segment(owner.key, strings)?;
            segments.push(owner_name);
        }

        // climb to the parent scope
        let Some((parent_id, _)) = scope.parent else {
            break;
        };
        scope_id = parent_id;
    }

    // reverse for root to leaf order
    segments.reverse();
    Some(segments.join("."))
}

/// Get the qualified name of a symbol with module prefix.
pub fn format_symbol_qualified_name(
    symbol_id: dir::GlobalSymbolId,
    repository: &Repository,
    revision: Revision,
    strings: &StringPool,
) -> Option<String> {
    // resolve the owning module and package
    let module = repository
        .module(revision, symbol_id.module_id)
        .ok()
        .flatten()?;
    let package = repository
        .package(revision, module.package_id)
        .ok()
        .flatten()?;

    // resolve package and module path
    let package_name = package.name.as_ref()?;
    if package_name.is_empty() {
        return None;
    }
    let module_path = module_path_without_extension(module.as_ref(), package.as_ref())?;

    // resolve symbol path
    let symbol_path = format_symbol_path(symbol_id, repository, revision, strings)?;
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
    repository: &Repository,
    revision: Revision,
    strings: &StringPool,
) -> Option<String> {
    let name = format_symbol_qualified_name(symbol_id, repository, revision, strings)?;
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
        dir::StaticKey::Name(name_id) | dir::StaticKey::Number(name_id) => {
            Some(strings.get(name_id).to_string())
        }
        dir::StaticKey::Symbol(_) => None,
    }
}

/// Format a StaticKey.
pub fn format_static_key(key: &dir::StaticKey, strings: &StringPool) -> String {
    match key {
        dir::StaticKey::Name(name_id) => strings.get(*name_id).to_string(),
        dir::StaticKey::Number(name_id) => strings.get(*name_id).to_string(),
        dir::StaticKey::Symbol(symbol) => format_symbol_key(symbol, strings),
    }
}

/// Format a SymbolKey.
pub fn format_symbol_key(key: &dir::SymbolKey, strings: &StringPool) -> String {
    match key {
        dir::SymbolKey::Unique(_) => "<unique symbol>".to_string(),
        dir::SymbolKey::WellKnown(symbol) => format!("[{}]", symbol.global_symbol_name()),
        dir::SymbolKey::Registry(name_id) => {
            let name = &*strings.get(*name_id);
            format!("[Symbol.for(\"{name}\")]")
        }
    }
}

/// Format a StaticArgument.
pub fn format_static_argument(
    argument: &dir::StaticArgument,
    types: &dir::TypeTable,
    repository: &Repository,
    revision: Revision,
    strings: &StringPool,
) -> String {
    match argument {
        dir::StaticArgument::Unevaluated { node } => types
            .get_declared_or_inferred_type_id(*node)
            .map(|type_id| format_local_type(type_id, types, repository, revision, strings))
            .unwrap_or_else(|| "<unevaluated>".to_string()),
        dir::StaticArgument::Evaluated { name, value } => {
            let value_str = format_static_expression(value, types, repository, revision, strings);
            if let Some(name_id) = name {
                let name_str = &*strings.get(*name_id);
                format!("{name_str}: {value_str}")
            } else {
                value_str
            }
        }
    }
}

/// Format a StaticExpression.
pub fn format_static_expression(
    expression: &dir::StaticExpression,
    types: &dir::TypeTable,
    repository: &Repository,
    revision: Revision,
    strings: &StringPool,
) -> String {
    match expression {
        dir::StaticExpression::Unevaluated { .. } => "<unevaluated>".to_string(),
        dir::StaticExpression::ScalarLiteral { value } => format_scalar_literal(value, strings),
        dir::StaticExpression::TypeLiteral { value } => format_type_literal(value, strings),
        dir::StaticExpression::Declaration { .. } => "<declaration>".to_string(),
        dir::StaticExpression::Type { ty } => {
            let ty = types.get_type(*ty);
            format_type(ty, types, repository, revision, strings)
        }
        dir::StaticExpression::ArrayExpression { elements } => {
            let elements: Vec<_> = elements
                .iter()
                .map(|e| format_static_expression(e, types, repository, revision, strings))
                .collect();
            format!("[{}]", elements.join(", "))
        }
        dir::StaticExpression::TupleExpression { elements } => {
            let elements: Vec<_> = elements
                .iter()
                .map(|e| format_static_expression(e, types, repository, revision, strings))
                .collect();
            format!("({})", elements.join(", "))
        }
        dir::StaticExpression::ObjectExpression { .. } => "{...}".to_string(),
    }
}

fn format_type_intrinsic(intrinsic: &dir::IntrinsicType) -> String {
    match intrinsic {
        dir::IntrinsicType::Uppercase => "Uppercase".to_string(),
        dir::IntrinsicType::Lowercase => "Lowercase".to_string(),
        dir::IntrinsicType::Capitalize => "Capitalize".to_string(),
        dir::IntrinsicType::Uncapitalize => "Uncapitalize".to_string(),
        dir::IntrinsicType::NoInfer => "NoInfer".to_string(),
        dir::IntrinsicType::BuiltinIteratorReturn => "BuiltinIteratorReturn".to_string(),
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

fn format_type_tuple_element(
    element: &dir::TypeElement,
    types: &dir::TypeTable,
    repository: &Repository,
    revision: Revision,
    strings: &StringPool,
) -> String {
    let mut result = String::new();
    if element.is_readonly {
        result.push_str("readonly ");
    }
    if element.is_rest {
        result.push_str("...");
    }
    if let Some(label) = element.label {
        let name = strings.get(label);
        let ty = format_local_type(element.ty, types, repository, revision, strings);
        result.push_str(&format!("{}: {ty}", name.as_ref()));
    } else {
        result.push_str(&format_local_type(
            element.ty, types, repository, revision, strings,
        ));
    }
    if element.is_optional {
        result.push('?');
    }
    result
}

fn format_path(path: &dir::Path, strings: &StringPool) -> String {
    let segments: Vec<_> = path
        .segments
        .iter()
        .map(|segment| strings.get(*segment).to_string())
        .collect();
    segments.join(".")
}

fn format_type_predicate_subject(
    subject: dir::PredicateSubject,
    repository: &Repository,
    revision: Revision,
    strings: &StringPool,
) -> String {
    match subject {
        dir::PredicateSubject::This => "this".to_string(),
        dir::PredicateSubject::Unresolved(name) => strings.get(name).to_string(),
        dir::PredicateSubject::Symbol(symbol_id) => {
            format_symbol_name(symbol_id, repository, revision, strings)
        }
    }
}
