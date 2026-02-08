use destack_dir as dir;

const DEFAULT_RELATION_CACHE_KEY: u64 = 0;

/// Return true when the type is strictly boolean.
pub fn is_strict_boolean_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    // prefer flow-normalized types when available
    let normalized_type_id = types
        .normalized_type(
            dir::NormalizationMode::Flow,
            DEFAULT_RELATION_CACHE_KEY,
            type_id,
        )
        .map(|entry| entry.normalized_type)
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
        .normalized_type(
            dir::NormalizationMode::Flow,
            DEFAULT_RELATION_CACHE_KEY,
            type_id,
        )
        .map(|entry| entry.normalized_type)
        .unwrap_or(type_id);

    // track visited nodes to avoid recursion cycles
    let mut visited = Vec::new();

    // resolve the type tree
    is_array_type_inner(types, normalized_type_id, array_symbol, &mut visited)
}

/// Return true when the type is a string type.
pub fn is_string_type(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    string_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    // prefer flow-normalized types when available
    let normalized_type_id = types
        .normalized_type(
            dir::NormalizationMode::Flow,
            DEFAULT_RELATION_CACHE_KEY,
            type_id,
        )
        .map(|entry| entry.normalized_type)
        .unwrap_or(type_id);

    // track visited nodes to avoid recursion cycles
    let mut visited = Vec::new();

    // resolve the type tree
    is_string_type_inner(types, normalized_type_id, string_symbol, &mut visited)
}

/// Return true when the type is a function type.
pub fn is_function_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    // prefer flow-normalized types when available
    let normalized_type_id = types
        .normalized_type(
            dir::NormalizationMode::Flow,
            DEFAULT_RELATION_CACHE_KEY,
            type_id,
        )
        .map(|entry| entry.normalized_type)
        .unwrap_or(type_id);

    // track visited nodes to avoid recursion cycles
    let mut visited = Vec::new();

    // resolve the type tree
    is_function_type_inner(types, normalized_type_id, &mut visited)
}

/// Return true when the type is an async function.
pub fn is_async_function_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    // prefer flow-normalized types when available
    let normalized_type_id = types
        .normalized_type(
            dir::NormalizationMode::Flow,
            DEFAULT_RELATION_CACHE_KEY,
            type_id,
        )
        .map(|entry| entry.normalized_type)
        .unwrap_or(type_id);

    // track visited nodes to avoid recursion cycles
    let mut visited = Vec::new();

    // resolve the type tree
    is_async_function_type_inner(types, normalized_type_id, &mut visited)
}

/// Return true when the type is `any`.
pub fn is_any_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    // prefer flow-normalized types when available
    let normalized_type_id = types
        .normalized_type(
            dir::NormalizationMode::Flow,
            DEFAULT_RELATION_CACHE_KEY,
            type_id,
        )
        .map(|entry| entry.normalized_type)
        .unwrap_or(type_id);

    // track visited nodes to avoid recursion cycles
    let mut visited = Vec::new();

    // resolve the type tree
    is_any_type_inner(types, normalized_type_id, &mut visited)
}

/// Return true when the type is a Promise.
pub fn is_promise_type(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    promise_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    let Some(promise_symbol) = promise_symbol else {
        return false;
    };

    // prefer flow-normalized types when available
    let normalized_type_id = types
        .normalized_type(
            dir::NormalizationMode::Flow,
            DEFAULT_RELATION_CACHE_KEY,
            type_id,
        )
        .map(|entry| entry.normalized_type)
        .unwrap_or(type_id);

    // track visited nodes to avoid recursion cycles
    let mut visited = Vec::new();

    // resolve the type tree
    is_promise_type_inner(types, normalized_type_id, promise_symbol, &mut visited)
}

/// Resolve the parameter type at an index for a function like type.
pub fn function_parameter_type_at(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    index: usize,
) -> Option<dir::LocalTypeId> {
    // prefer flow-normalized types when available
    let normalized_type_id = types
        .normalized_type(
            dir::NormalizationMode::Flow,
            DEFAULT_RELATION_CACHE_KEY,
            type_id,
        )
        .map(|entry| entry.normalized_type)
        .unwrap_or(type_id);

    // track visited nodes to avoid recursion cycles
    let mut visited = Vec::new();

    // resolve the type tree
    function_parameter_type_at_inner(types, normalized_type_id, index, &mut visited)
}

/// Resolve all parameter types at an index for a function like type.
pub fn function_parameter_types_at(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    index: usize,
) -> Vec<dir::LocalTypeId> {
    // prefer flow-normalized types when available
    let normalized_type_id = types
        .normalized_type(
            dir::NormalizationMode::Flow,
            DEFAULT_RELATION_CACHE_KEY,
            type_id,
        )
        .map(|entry| entry.normalized_type)
        .unwrap_or(type_id);

    // track visited nodes to avoid recursion cycles
    let mut visited = Vec::new();
    let mut results = Vec::new();

    // resolve all parameter types for the index
    function_parameter_types_at_inner(types, normalized_type_id, index, &mut visited, &mut results);

    results
}

/// Resolve the return type for a function like type.
pub fn function_return_type(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
) -> Option<dir::LocalTypeId> {
    // prefer flow-normalized types when available
    let normalized_type_id = types
        .normalized_type(
            dir::NormalizationMode::Flow,
            DEFAULT_RELATION_CACHE_KEY,
            type_id,
        )
        .map(|entry| entry.normalized_type)
        .unwrap_or(type_id);

    // track visited nodes to avoid recursion cycles
    let mut visited = Vec::new();

    // resolve the type tree
    function_return_type_inner(types, normalized_type_id, &mut visited)
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
        dir::Type::ValueOf { right, .. }
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
        dir::Type::ValueOf { right, .. }
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

/// Return true when the type is a string with cycle protection.
fn is_string_type_inner(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    string_symbol: Option<dir::GlobalSymbolId>,
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
            dir::TypeLiteral::Primitive(dir::PrimitiveType::String)
                | dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::String(_))
                | dir::TypeLiteral::Intrinsic(
                    dir::IntrinsicType::Uppercase
                        | dir::IntrinsicType::Lowercase
                        | dir::IntrinsicType::Capitalize
                        | dir::IntrinsicType::Uncapitalize
                )
        ),
        dir::Type::TemplateLiteral { .. } => true,
        dir::Type::Value { value } => is_string_type_inner(types, *value, string_symbol, visited),
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => {
            is_string_type_inner(types, *right, string_symbol, visited)
        }
        dir::Type::Union { elements } => elements
            .iter()
            .all(|element| is_string_type_inner(types, *element, string_symbol, visited)),
        dir::Type::Intersection { elements } => elements
            .iter()
            .all(|element| is_string_type_inner(types, *element, string_symbol, visited)),
        dir::Type::Reference { symbol, .. } => {
            if string_symbol.is_some_and(|string_symbol| string_symbol == *symbol) {
                return true;
            }

            // follow instance types when available
            if let Some(instance_type_id) = types.get_instance_type_id(*symbol) {
                return is_string_type_inner(types, instance_type_id, string_symbol, visited);
            }

            // fall back to value types for aliases
            if let Some(value_type_id) = types.get_value_type_id(*symbol) {
                return is_string_type_inner(types, value_type_id, string_symbol, visited);
            }

            false
        }
        _ => false,
    }
}

/// Return true when the type is a floating-point type.
pub fn is_float_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    // prefer flow-normalized types when available
    let normalized_type_id = types
        .normalized_type(
            dir::NormalizationMode::Flow,
            DEFAULT_RELATION_CACHE_KEY,
            type_id,
        )
        .map(|entry| entry.normalized_type)
        .unwrap_or(type_id);

    // track visited nodes to avoid recursion cycles
    let mut visited = Vec::new();

    // resolve the type tree
    is_float_type_inner(types, normalized_type_id, &mut visited)
}

/// Return true when the type is a floating-point type with cycle protection.
fn is_float_type_inner(
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
            dir::TypeLiteral::Primitive(dir::PrimitiveType::Float(_))
                | dir::TypeLiteral::Primitive(dir::PrimitiveType::Number)
        ),
        dir::Type::Value { value } => is_float_type_inner(types, *value, visited),
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => is_float_type_inner(types, *right, visited),
        dir::Type::Union { elements } => elements
            .iter()
            .all(|element| is_float_type_inner(types, *element, visited)),
        dir::Type::Intersection { elements } => elements
            .iter()
            .all(|element| is_float_type_inner(types, *element, visited)),
        dir::Type::Reference { symbol, .. } => {
            // follow instance types when available
            if let Some(instance_type_id) = types.get_instance_type_id(*symbol) {
                return is_float_type_inner(types, instance_type_id, visited);
            }

            // fall back to value types for aliases
            if let Some(value_type_id) = types.get_value_type_id(*symbol) {
                return is_float_type_inner(types, value_type_id, visited);
            }

            false
        }
        _ => false,
    }
}

/// Return true when the type is a function with cycle protection.
fn is_function_type_inner(
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
        dir::Type::Function { .. } => true,
        dir::Type::Object {
            call_signatures, ..
        } => !call_signatures.is_empty(),
        dir::Type::Value { value } => is_function_type_inner(types, *value, visited),
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => is_function_type_inner(types, *right, visited),
        dir::Type::Union { elements } => elements
            .iter()
            .all(|element| is_function_type_inner(types, *element, visited)),
        dir::Type::Intersection { elements } => elements
            .iter()
            .all(|element| is_function_type_inner(types, *element, visited)),
        dir::Type::Reference { symbol, .. } => {
            // follow instance types when available
            if let Some(instance_type_id) = types.get_instance_type_id(*symbol) {
                return is_function_type_inner(types, instance_type_id, visited);
            }

            // fall back to value types for aliases
            if let Some(value_type_id) = types.get_value_type_id(*symbol) {
                return is_function_type_inner(types, value_type_id, visited);
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
        dir::Type::ValueOf { right, .. }
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

/// Return true when the type is `any` with cycle protection.
fn is_any_type_inner(
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
        dir::Type::TypeLiteral {
            value: dir::TypeLiteral::Any | dir::TypeLiteral::Unknown,
        } => true,
        dir::Type::Value { value } => is_any_type_inner(types, *value, visited),
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => is_any_type_inner(types, *right, visited),
        dir::Type::Union { elements } | dir::Type::Intersection { elements } => elements
            .iter()
            .any(|element| is_any_type_inner(types, *element, visited)),
        dir::Type::Reference { symbol, .. } => {
            // follow instance types when available
            if let Some(instance_type_id) = types.get_instance_type_id(*symbol) {
                return is_any_type_inner(types, instance_type_id, visited);
            }

            // fall back to value types for aliases
            if let Some(value_type_id) = types.get_value_type_id(*symbol) {
                return is_any_type_inner(types, value_type_id, visited);
            }

            false
        }
        _ => false,
    }
}

/// Return true when the type is a Promise with cycle protection.
fn is_promise_type_inner(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    promise_symbol: dir::GlobalSymbolId,
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
        dir::Type::Reference { symbol, .. } => {
            if *symbol == promise_symbol {
                return true;
            }

            // follow instance types when available
            if let Some(instance_type_id) = types.get_instance_type_id(*symbol) {
                return is_promise_type_inner(types, instance_type_id, promise_symbol, visited);
            }

            // fall back to value types for aliases
            if let Some(value_type_id) = types.get_value_type_id(*symbol) {
                return is_promise_type_inner(types, value_type_id, promise_symbol, visited);
            }

            false
        }
        dir::Type::Value { value } => is_promise_type_inner(types, *value, promise_symbol, visited),
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => {
            is_promise_type_inner(types, *right, promise_symbol, visited)
        }
        dir::Type::Union { elements } => elements
            .iter()
            .all(|element| is_promise_type_inner(types, *element, promise_symbol, visited)),
        dir::Type::Intersection { elements } => elements
            .iter()
            .any(|element| is_promise_type_inner(types, *element, promise_symbol, visited)),
        _ => false,
    }
}

/// Resolve a parameter type for a function type with cycle protection.
fn function_parameter_type_at_inner(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    index: usize,
    visited: &mut Vec<dir::LocalTypeId>,
) -> Option<dir::LocalTypeId> {
    // guard against cycles
    if visited.contains(&type_id) {
        return None;
    }
    visited.push(type_id);

    // inspect the type node
    let ty = types.get_type(type_id);
    match ty {
        dir::Type::Function {
            dynamic_parameters, ..
        } => dynamic_parameters.get(index).copied(),
        dir::Type::Object {
            call_signatures, ..
        } => {
            let first_signature = call_signatures.first()?;
            function_parameter_type_at_inner(types, *first_signature, index, visited)
        }
        dir::Type::Value { value } => {
            function_parameter_type_at_inner(types, *value, index, visited)
        }
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => {
            function_parameter_type_at_inner(types, *right, index, visited)
        }
        dir::Type::Reference { symbol, .. } => {
            if let Some(instance_type_id) = types.get_instance_type_id(*symbol) {
                return function_parameter_type_at_inner(types, instance_type_id, index, visited);
            }

            if let Some(value_type_id) = types.get_value_type_id(*symbol) {
                return function_parameter_type_at_inner(types, value_type_id, index, visited);
            }

            None
        }
        _ => None,
    }
}

/// Resolve all parameter types for a function type with cycle protection.
fn function_parameter_types_at_inner(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    index: usize,
    visited: &mut Vec<dir::LocalTypeId>,
    results: &mut Vec<dir::LocalTypeId>,
) {
    // guard against cycles
    if visited.contains(&type_id) {
        return;
    }
    visited.push(type_id);

    // inspect the type node
    let ty = types.get_type(type_id);
    match ty {
        dir::Type::Function {
            dynamic_parameters, ..
        } => {
            if let Some(parameter_type_id) = dynamic_parameters.get(index).copied()
                && !results.contains(&parameter_type_id)
            {
                results.push(parameter_type_id);
            }
        }
        dir::Type::Object {
            call_signatures, ..
        } => {
            for signature_id in call_signatures {
                function_parameter_types_at_inner(types, *signature_id, index, visited, results);
            }
        }
        dir::Type::Union { elements } | dir::Type::Intersection { elements } => {
            for element_type_id in elements {
                function_parameter_types_at_inner(types, *element_type_id, index, visited, results);
            }
        }
        dir::Type::Value { value } => {
            function_parameter_types_at_inner(types, *value, index, visited, results);
        }
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => {
            function_parameter_types_at_inner(types, *right, index, visited, results);
        }
        dir::Type::Reference { symbol, .. } => {
            if let Some(instance_type_id) = types.get_instance_type_id(*symbol) {
                function_parameter_types_at_inner(types, instance_type_id, index, visited, results);
            }

            if let Some(value_type_id) = types.get_value_type_id(*symbol) {
                function_parameter_types_at_inner(types, value_type_id, index, visited, results);
            }
        }
        _ => {}
    }
}

/// Resolve a return type for a function type with cycle protection.
fn function_return_type_inner(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    visited: &mut Vec<dir::LocalTypeId>,
) -> Option<dir::LocalTypeId> {
    // guard against cycles
    if visited.contains(&type_id) {
        return None;
    }
    visited.push(type_id);

    // inspect the type node
    let ty = types.get_type(type_id);
    match ty {
        dir::Type::Function { return_type, .. } => *return_type,
        dir::Type::Object {
            call_signatures, ..
        } => {
            let first_signature = call_signatures.first()?;
            function_return_type_inner(types, *first_signature, visited)
        }
        dir::Type::Value { value } => function_return_type_inner(types, *value, visited),
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => function_return_type_inner(types, *right, visited),
        dir::Type::Reference { symbol, .. } => {
            if let Some(instance_type_id) = types.get_instance_type_id(*symbol) {
                return function_return_type_inner(types, instance_type_id, visited);
            }

            if let Some(value_type_id) = types.get_value_type_id(*symbol) {
                return function_return_type_inner(types, value_type_id, visited);
            }

            None
        }
        _ => None,
    }
}
