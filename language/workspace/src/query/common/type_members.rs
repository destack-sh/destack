use destack_ast::StringPool;
use destack_dir::{
    GlobalSymbolId, LocalScopeId, LocalSymbolId, LocalTypeId, ScalarLiteral, StaticKey,
    SymbolSpace, SymbolTable, SymbolType, Type, TypeTable, WellKnownSymbol,
};

use crate::Session;

/// Maximum recursion depth for type member resolution.
const MAX_TYPE_DEPTH: u32 = 10;

/// Information about a member of a type.
#[derive(Debug, Clone)]
pub struct MemberInfo {
    /// The name of the member.
    pub name: MemberName,
    /// The type of the member.
    pub type_id: Option<LocalTypeId>,
    /// The kind of member (field, method, etc.).
    pub kind: MemberKind,
    /// Whether the member is optional.
    pub is_optional: bool,
    /// Whether the member is readonly.
    pub is_readonly: bool,
    /// The symbol id if this member comes from a symbol declaration.
    pub symbol_id: Option<GlobalSymbolId>,
}

/// The name of a member.
#[derive(Debug, Clone)]
pub enum MemberName {
    /// A resolved string name.
    String(String),
    /// A numeric index.
    Index(i64),
    /// A computed key (cannot be displayed directly).
    Computed,
}

/// The kind of member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberKind {
    /// A field or property.
    Field,
    /// A method (function-typed member).
    Method,
    /// A call signature.
    CallSignature,
    /// A construct signature.
    ConstructSignature,
    /// An index signature.
    IndexSignature,
    /// An enum member/variant.
    EnumMember,
}

/// Get members of a type for completion purposes.
///
/// Returns all accessible members of a type, handling:
/// - Reference types (lookup symbol's owned scope)
/// - Object types (fields, call/construct signatures)
/// - Union types (intersection of members)
/// - Intersection types (union of members)
pub fn resolve_type_members(
    types: &TypeTable,
    symbols: &SymbolTable,
    type_id: LocalTypeId,
    session: &Session,
) -> Vec<MemberInfo> {
    let ty = types.get_type(type_id);
    resolve_type_members_inner(ty, types, symbols, &session.strings, session, 0)
}

/// Internal recursive implementation with depth limit.
fn resolve_type_members_inner(
    ty: &Type,
    types: &TypeTable,
    _symbols: &SymbolTable,
    strings: &StringPool,
    session: &Session,
    depth: u32,
) -> Vec<MemberInfo> {
    // prevent infinite recursion
    if depth > MAX_TYPE_DEPTH {
        return Vec::new();
    }

    match ty {
        // reference to a declared type: look up symbol's owned scope
        Type::Reference { symbol, .. } => resolve_reference_members(*symbol, session),

        // object type: return fields directly
        Type::Object {
            fields,
            call_signatures,
            construct_signatures,
            ..
        } => {
            let mut members = Vec::new();

            // add fields
            for field in fields {
                let kind = if is_function_type(types, field.ty) {
                    MemberKind::Method
                } else {
                    MemberKind::Field
                };

                members.push(MemberInfo {
                    name: static_key_to_member_name(&field.key, strings),
                    type_id: Some(field.ty),
                    kind,
                    is_optional: field.is_optional,
                    is_readonly: field.is_readonly,
                    symbol_id: None,
                });
            }

            // add call signatures
            for sig_type_id in call_signatures {
                members.push(MemberInfo {
                    name: MemberName::Computed,
                    type_id: Some(*sig_type_id),
                    kind: MemberKind::CallSignature,
                    is_optional: false,
                    is_readonly: false,
                    symbol_id: None,
                });
            }

            // add construct signatures
            for sig_type_id in construct_signatures {
                members.push(MemberInfo {
                    name: MemberName::Computed,
                    type_id: Some(*sig_type_id),
                    kind: MemberKind::ConstructSignature,
                    is_optional: false,
                    is_readonly: false,
                    symbol_id: None,
                });
            }

            members
        }

        // union type: intersect members from all elements
        Type::Union { elements } => {
            if elements.is_empty() {
                return Vec::new();
            }

            // get members from first element
            let first_type = types.get_type(elements[0]);
            let mut common_members = resolve_type_members_inner(
                first_type,
                types,
                _symbols,
                strings,
                session,
                depth + 1,
            );

            // intersect with remaining elements
            for element_id in &elements[1..] {
                let element_type = types.get_type(*element_id);
                let element_members = resolve_type_members_inner(
                    element_type,
                    types,
                    _symbols,
                    strings,
                    session,
                    depth + 1,
                );

                // keep only members that exist in both
                common_members.retain(|member| {
                    element_members
                        .iter()
                        .any(|other| member_names_match(&member.name, &other.name))
                });
            }

            common_members
        }

        // intersection type: union members from all elements
        Type::Intersection { elements } => {
            // check if this intersection represents an enum's static type
            // by looking for a Value<Reference<Enum>> element
            let is_enum_static = elements.iter().any(|element_id| {
                let element = types.get_type(*element_id);
                if let Type::Value { value } = element {
                    let inner = types.get_type(*value);
                    if let Type::Reference { symbol, .. } = inner {
                        // load the symbol and check if it's an enum
                        let module = session.modules.get(symbol.module_id);
                        let module = module.read();
                        if let Some(ctx) = session.query_context(&module) {
                            let symbols_table = ctx.symbols();
                            let sym = symbols_table.get_symbol(symbol.local_id);
                            return sym.ty == SymbolType::Enum;
                        }
                    }
                }
                false
            });

            let mut all_members = Vec::new();
            let mut seen_names = Vec::new();

            for element_id in elements {
                let element_type = types.get_type(*element_id);
                let element_members = resolve_type_members_inner(
                    element_type,
                    types,
                    _symbols,
                    strings,
                    session,
                    depth + 1,
                );

                for mut member in element_members {
                    // avoid duplicates
                    if !seen_names
                        .iter()
                        .any(|name| member_names_match(name, &member.name))
                    {
                        // if this is an enum's static type, convert Field members to EnumMember
                        if is_enum_static && member.kind == MemberKind::Field {
                            member.kind = MemberKind::EnumMember;
                        }
                        seen_names.push(member.name.clone());
                        all_members.push(member);
                    }
                }
            }

            all_members
        }

        // tuple type: numeric indices
        Type::Tuple { elements, .. } => elements
            .iter()
            .enumerate()
            .map(|(i, element)| MemberInfo {
                name: MemberName::Index(i as i64),
                type_id: Some(element.ty),
                kind: MemberKind::Field,
                is_optional: element.is_optional,
                is_readonly: element.is_readonly,
                symbol_id: None,
            })
            .collect(),

        // array type: resolve members from well-known Array type
        Type::Array { element, .. } => array_members(*element, session),

        Type::ArraySized { element, .. } => array_members(Some(*element), session),

        // primitive types: resolve members from well-known types (String, Number, etc.)
        Type::TypeLiteral { value } => primitive_members(value, session),

        // follow value types
        Type::Value { value } => {
            let inner = types.get_type(*value);
            resolve_type_members_inner(inner, types, _symbols, strings, session, depth + 1)
        }

        // other types have no direct members
        _ => Vec::new(),
    }
}

/// Resolve members from a reference type by looking up the symbol.
fn resolve_reference_members(symbol_id: GlobalSymbolId, session: &Session) -> Vec<MemberInfo> {
    // load the symbol's module
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let Some(ctx) = session.query_context(&module) else {
        return Vec::new();
    };
    let symbols = ctx.symbols();
    let types = ctx.types();

    resolve_local_symbol_members(symbol_id.local_id, &types, &symbols, &session.strings)
}

/// Resolve members from a local symbol by finding its instance type or owned scope.
fn resolve_local_symbol_members(
    symbol_id: LocalSymbolId,
    types: &TypeTable,
    symbols: &SymbolTable,
    strings: &destack_base::StringPool,
) -> Vec<MemberInfo> {
    let mut members = Vec::new();

    let module_id = symbols.module_id;
    let global_symbol_id = GlobalSymbolId {
        module_id,
        local_id: symbol_id,
    };

    // check if this symbol is an enum (for member kind detection)
    let symbol = symbols.get_symbol(symbol_id);
    let is_enum = symbol.ty == SymbolType::Enum;

    // first: try to get members from the symbol's instance type
    // this is the primary source for struct/class/interface fields
    if let Some(instance_type_id) = types.get_instance_type_id(global_symbol_id) {
        let ty = types.get_type(instance_type_id);

        // if the instance type is an Object, get fields from there
        if let Type::Object {
            fields,
            call_signatures,
            construct_signatures,
            ..
        } = ty
        {
            for field in fields {
                // use EnumMember kind for enum variants
                let kind = if is_enum {
                    MemberKind::EnumMember
                } else if is_function_type(types, field.ty) {
                    MemberKind::Method
                } else {
                    MemberKind::Field
                };

                members.push(MemberInfo {
                    name: static_key_to_member_name(&field.key, strings),
                    type_id: Some(field.ty),
                    kind,
                    is_optional: field.is_optional,
                    is_readonly: field.is_readonly,
                    symbol_id: None,
                });
            }

            // add call signatures
            for sig_type_id in call_signatures {
                members.push(MemberInfo {
                    name: MemberName::Computed,
                    type_id: Some(*sig_type_id),
                    kind: MemberKind::CallSignature,
                    is_optional: false,
                    is_readonly: false,
                    symbol_id: None,
                });
            }

            // add construct signatures
            for sig_type_id in construct_signatures {
                members.push(MemberInfo {
                    name: MemberName::Computed,
                    type_id: Some(*sig_type_id),
                    kind: MemberKind::ConstructSignature,
                    is_optional: false,
                    is_readonly: false,
                    symbol_id: None,
                });
            }

            return members;
        }
    }

    // fallback: look for scope-based members (for classes, interfaces with methods)
    let Some(owned_scope_id) = find_symbol_owned_scope(symbols, symbol_id) else {
        return members;
    };

    let scope = symbols.get_scope_by_id(owned_scope_id);

    // collect named members from the scope
    for (key, member_id) in symbols.active_named_symbols(scope) {
        let member_symbol = symbols.get_symbol(member_id);

        // only include value-space symbols (methods, fields)
        if member_symbol.space != SymbolSpace::Value
            && member_symbol.space != SymbolSpace::TypeValue
        {
            continue;
        }

        // determine member kind based on parent type and member type
        let kind = if is_enum {
            MemberKind::EnumMember
        } else if member_symbol.ty == SymbolType::Function {
            MemberKind::Method
        } else {
            MemberKind::Field
        };

        // get type from primary declaration
        let type_id = member_symbol
            .primary_declaration
            .and_then(|decl| types.get_declared_or_inferred_type_id(decl));

        members.push(MemberInfo {
            name: static_key_to_member_name(&key, strings),
            type_id,
            kind,
            is_optional: false, // would need to check declaration
            is_readonly: false, // would need to check declaration
            symbol_id: Some(GlobalSymbolId {
                module_id,
                local_id: member_id,
            }),
        });
    }

    members
}

/// Find the scope owned by a symbol.
fn find_symbol_owned_scope(
    symbols: &SymbolTable,
    symbol_id: LocalSymbolId,
) -> Option<LocalScopeId> {
    for (idx, scope) in symbols.scopes().enumerate() {
        if scope.owner_id == Some(symbol_id) {
            return Some(LocalScopeId::new(idx as u32));
        }
    }
    None
}

/// Convert a static key to a member name, resolving the string ID to an actual string.
fn static_key_to_member_name(key: &StaticKey, strings: &destack_base::StringPool) -> MemberName {
    match key {
        StaticKey::Name(string_id) => MemberName::String(strings.get(*string_id).to_string()),
        StaticKey::Number(string_id) => MemberName::String(strings.get(*string_id).to_string()),
        StaticKey::Symbol(_) => MemberName::Computed,
    }
}

/// Check if a type is a function type.
fn is_function_type(types: &TypeTable, type_id: LocalTypeId) -> bool {
    matches!(types.get_type(type_id), Type::Function { .. })
}

/// Check if two member names match.
fn member_names_match(a: &MemberName, b: &MemberName) -> bool {
    match (a, b) {
        (MemberName::String(a), MemberName::String(b)) => a == b,
        (MemberName::Index(a), MemberName::Index(b)) => a == b,
        _ => false,
    }
}

/// Get members for array types by resolving the well-known Array symbol.
fn array_members(_element_type: Option<LocalTypeId>, session: &Session) -> Vec<MemberInfo> {
    resolve_well_known_members(session, destack_dir::WellKnownSymbol::Array)
}

/// Get members for primitive types by resolving the appropriate well-known symbol.
fn primitive_members(value: &destack_dir::TypeLiteral, session: &Session) -> Vec<MemberInfo> {
    use destack_dir::{PrimitiveType, TypeLiteral, WellKnownSymbol};

    // map primitive type literals to their backing well-known types
    let well_known = match value {
        TypeLiteral::Primitive(primitive) => match primitive {
            PrimitiveType::String => Some(WellKnownSymbol::String),
            PrimitiveType::Number | PrimitiveType::Int(_) | PrimitiveType::Float(_) => {
                Some(WellKnownSymbol::Number)
            }
            PrimitiveType::Boolean => Some(WellKnownSymbol::Boolean),
            PrimitiveType::Bigint => Some(WellKnownSymbol::BigInt),
            PrimitiveType::Symbol | PrimitiveType::UniqueSymbol => Some(WellKnownSymbol::Symbol),
            PrimitiveType::Character => None,
        },
        // scalar literals (string literals, number literals) use the same backing types
        TypeLiteral::ScalarLiteral(scalar) => match scalar {
            ScalarLiteral::String(_) => Some(WellKnownSymbol::String),
            ScalarLiteral::Integer(_) | ScalarLiteral::Float(_) => Some(WellKnownSymbol::Number),
            ScalarLiteral::Boolean(_) => Some(WellKnownSymbol::Boolean),
            ScalarLiteral::Bigint(_) => Some(WellKnownSymbol::BigInt),
            ScalarLiteral::Character(_) | ScalarLiteral::RegexString { .. } => None,
        },
        // other type literals don't have backing types with members
        _ => None,
    };

    well_known
        .map(|wk| resolve_well_known_members(session, wk))
        .unwrap_or_default()
}

/// Resolve members from a well-known symbol (Array, String, etc.).
fn resolve_well_known_members(session: &Session, well_known: WellKnownSymbol) -> Vec<MemberInfo> {
    // try to find well-known symbols from any profile
    // (LSP queries don't have a specific profile context)
    for entry in session.builtins.well_known_by_profile.iter() {
        let well_known_symbols = entry.value();
        if let Some(symbol_id) = well_known_symbols.get_type_symbol(well_known) {
            return resolve_reference_members(symbol_id, session);
        }
    }

    Vec::new()
}
