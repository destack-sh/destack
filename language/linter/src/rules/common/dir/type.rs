use destack_dir as dir;

/// Return true when the type is strictly boolean.
pub fn is_strict_boolean_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    // prefer flow-normalized types when available
    let normalized_type_id = types
        .normalized_type(dir::NormalizationMode::Flow, type_id)
        .unwrap_or(type_id);

    // track visited nodes to avoid recursion cycles
    let mut visited = Vec::new();

    // resolve the type tree
    is_strict_boolean_type_inner(types, normalized_type_id, &mut visited)
}

/// Return true when the type is an array type.
pub fn is_array_type(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    array_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    // prefer flow-normalized types when available
    let normalized_type_id = types
        .normalized_type(dir::NormalizationMode::Flow, type_id)
        .unwrap_or(type_id);

    // track visited nodes to avoid recursion cycles
    let mut visited = Vec::new();

    // resolve the type tree
    is_array_type_inner(types, normalized_type_id, array_symbol, &mut visited)
}

/// Return true when the type is an async function.
pub fn is_async_function_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    // prefer flow-normalized types when available
    let normalized_type_id = types
        .normalized_type(dir::NormalizationMode::Flow, type_id)
        .unwrap_or(type_id);

    // track visited nodes to avoid recursion cycles
    let mut visited = Vec::new();

    // resolve the type tree
    is_async_function_type_inner(types, normalized_type_id, &mut visited)
}

/// Return true when the type is strictly boolean with cycle protection.
fn is_strict_boolean_type_inner(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    visited: &mut Vec<dir::LocalTypeId>,
) -> bool {
    // guard against cycles
    if visited.contains(&type_id) {
        return false;
    }
    visited.push(type_id);

    // inspect the type node
    let ty = types.get_type(type_id);
    match ty {
        dir::Type::TypeLiteral { value } => matches!(
            value,
            dir::TypeLiteral::Primitive(dir::PrimitiveType::Boolean)
                | dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::Boolean(_))
        ),
        dir::Type::Value { value } => is_strict_boolean_type_inner(types, *value, visited),
        dir::Type::Mutable { right, .. }
        | dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => {
            is_strict_boolean_type_inner(types, *right, visited)
        }
        dir::Type::Union { elements } => elements
            .iter()
            .all(|element| is_strict_boolean_type_inner(types, *element, visited)),
        dir::Type::Intersection { elements } => elements
            .iter()
            .all(|element| is_strict_boolean_type_inner(types, *element, visited)),
        dir::Type::Reference { symbol, .. } => {
            // follow instance types when available
            if let Some(instance_type_id) = types.get_instance_type_id(*symbol) {
                return is_strict_boolean_type_inner(types, instance_type_id, visited);
            }

            // fall back to value types for aliases
            if let Some(value_type_id) = types.get_value_type_id(*symbol) {
                return is_strict_boolean_type_inner(types, value_type_id, visited);
            }

            false
        }
        _ => false,
    }
}

/// Return true when the type is an array with cycle protection.
fn is_array_type_inner(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    array_symbol: Option<dir::GlobalSymbolId>,
    visited: &mut Vec<dir::LocalTypeId>,
) -> bool {
    // guard against cycles
    if visited.contains(&type_id) {
        return false;
    }
    visited.push(type_id);

    // inspect the type node
    let ty = types.get_type(type_id);
    match ty {
        dir::Type::Array { .. } | dir::Type::ArraySized { .. } | dir::Type::Tuple { .. } => true,
        dir::Type::Value { value } => is_array_type_inner(types, *value, array_symbol, visited),
        dir::Type::Mutable { right, .. }
        | dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => {
            is_array_type_inner(types, *right, array_symbol, visited)
        }
        dir::Type::Union { elements } => elements
            .iter()
            .all(|element| is_array_type_inner(types, *element, array_symbol, visited)),
        dir::Type::Intersection { elements } => elements
            .iter()
            .all(|element| is_array_type_inner(types, *element, array_symbol, visited)),
        dir::Type::Reference { symbol, .. } => {
            if array_symbol.is_some_and(|array_symbol| array_symbol == *symbol) {
                return true;
            }

            // follow instance types when available
            if let Some(instance_type_id) = types.get_instance_type_id(*symbol) {
                return is_array_type_inner(types, instance_type_id, array_symbol, visited);
            }

            // fall back to value types for aliases
            if let Some(value_type_id) = types.get_value_type_id(*symbol) {
                return is_array_type_inner(types, value_type_id, array_symbol, visited);
            }

            false
        }
        _ => false,
    }
}

/// Return true when the type is an async function with cycle protection.
fn is_async_function_type_inner(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    visited: &mut Vec<dir::LocalTypeId>,
) -> bool {
    // guard against cycles
    if visited.contains(&type_id) {
        return false;
    }
    visited.push(type_id);

    // inspect the type node
    let ty = types.get_type(type_id);
    match ty {
        dir::Type::Function { asynchrony, .. } => *asynchrony == dir::Asynchrony::Async,
        dir::Type::Object {
            call_signatures, ..
        } => call_signatures
            .iter()
            .any(|signature| is_async_function_type_inner(types, *signature, visited)),
        dir::Type::Value { value } => is_async_function_type_inner(types, *value, visited),
        dir::Type::Mutable { right, .. }
        | dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => {
            is_async_function_type_inner(types, *right, visited)
        }
        dir::Type::Union { elements } => elements
            .iter()
            .all(|element| is_async_function_type_inner(types, *element, visited)),
        dir::Type::Intersection { elements } => elements
            .iter()
            .all(|element| is_async_function_type_inner(types, *element, visited)),
        dir::Type::Reference { symbol, .. } => {
            // follow instance types when available
            if let Some(instance_type_id) = types.get_instance_type_id(*symbol) {
                return is_async_function_type_inner(types, instance_type_id, visited);
            }

            // fall back to value types for aliases
            if let Some(value_type_id) = types.get_value_type_id(*symbol) {
                return is_async_function_type_inner(types, value_type_id, visited);
            }

            false
        }
        _ => false,
    }
}
