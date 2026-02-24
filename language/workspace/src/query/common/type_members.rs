use destack_ast::StringPool;
use destack_dir as dir;
use destack_dir::{
    Declaration, DynamicKey, GlobalSymbolId, LocalSymbolId, LocalTypeId, Member, ScalarLiteral,
    StaticKey, SymbolSpace, SymbolTable, SymbolType, Type, TypeTable, WellKnownSymbol,
};
use destack_source::ModuleId;

use super::{for_each_visible_extension, owned_scope_for_symbol};
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
    /// A method (function typed member).
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
/// Returns all accessible members of a type.
/// Handles reference types, object types, union types, intersection types, and extension members
/// visible from the current module.
pub fn resolve_type_members(
    types: &TypeTable,
    symbols: &SymbolTable,
    type_id: LocalTypeId,
    session: &Session,
    current_module_id: ModuleId,
) -> Vec<MemberInfo> {
    // resolve the root type and collect members
    let ty = types.get_type(type_id);
    resolve_type_members_inner(
        ty,
        types,
        symbols,
        &session.strings,
        session,
        current_module_id,
        0,
    )
}

/// Internal recursive implementation with depth limit.
fn resolve_type_members_inner(
    ty: &Type,
    types: &TypeTable,
    _symbols: &SymbolTable,
    strings: &StringPool,
    session: &Session,
    current_module_id: ModuleId,
    depth: u32,
) -> Vec<MemberInfo> {
    // prevent infinite recursion
    if depth > MAX_TYPE_DEPTH {
        return Vec::new();
    }

    match ty {
        // reference to a declared type: look up symbol's owned scope
        Type::Reference { symbol, .. } => {
            resolve_reference_members(*symbol, session, current_module_id)
        }

        // object type: return fields directly
        Type::Object {
            fields,
            call_signatures,
            construct_signatures,
            ..
        } => {
            // initialize the member buffer
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
            // return no members for empty unions
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
                current_module_id,
                depth + 1,
            );

            // intersect with remaining elements
            for element_id in &elements[1..] {
                // resolve members for the current union element
                let element_type = types.get_type(*element_id);
                let element_members = resolve_type_members_inner(
                    element_type,
                    types,
                    _symbols,
                    strings,
                    session,
                    current_module_id,
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

            // initialize member collection and name tracking
            let mut all_members = Vec::new();
            let mut seen_names = Vec::new();

            // collect members from each element
            for element_id in elements {
                // resolve the current element type
                let element_type = types.get_type(*element_id);
                let element_members = resolve_type_members_inner(
                    element_type,
                    types,
                    _symbols,
                    strings,
                    session,
                    current_module_id,
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
            // map tuple elements to index members
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

        // array type: resolve members from well known Array type
        Type::Array { element, .. } => array_members(*element, session, current_module_id),

        Type::ArraySized { element, .. } => {
            array_members(Some(*element), session, current_module_id)
        }

        // primitive types: resolve members from well known types (String, Number, etc.)
        Type::TypeLiteral { value } => primitive_members(value, session, current_module_id),

        // follow value types
        Type::Value { value } => {
            let inner = types.get_type(*value);
            resolve_type_members_inner(
                inner,
                types,
                _symbols,
                strings,
                session,
                current_module_id,
                depth + 1,
            )
        }

        // other types have no direct members
        _ => Vec::new(),
    }
}

/// Resolve members from a reference type by looking up the symbol.
pub(crate) fn resolve_reference_members(
    symbol_id: GlobalSymbolId,
    session: &Session,
    current_module_id: ModuleId,
) -> Vec<MemberInfo> {
    // load the symbol's module
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let Some(ctx) = session.query_context(&module) else {
        return Vec::new();
    };
    let symbols = ctx.symbols();
    let types = ctx.types();

    // resolve direct members from the symbol definition
    let mut members =
        resolve_local_symbol_members(symbol_id.local_id, &types, &symbols, &session.strings);

    // merge extension members for this symbol
    let extension_members =
        resolve_extension_members_for_symbol(session, symbol_id, current_module_id);

    // avoid duplicate member names across direct and extension members
    for member in extension_members {
        let is_duplicate = members
            .iter()
            .any(|existing| member_names_match(&existing.name, &member.name));
        if is_duplicate {
            continue;
        }

        members.push(member);
    }

    members
}

/// Resolve members from a local symbol by finding its instance type or owned scope.
fn resolve_local_symbol_members(
    symbol_id: LocalSymbolId,
    types: &TypeTable,
    symbols: &SymbolTable,
    strings: &destack_base::StringPool,
) -> Vec<MemberInfo> {
    // prepare the member buffer
    let mut members = Vec::new();

    // resolve the global symbol id for type table lookups
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

    // fallback: look for scope based members (for classes, interfaces with methods)
    let Some(owned_scope_id) = owned_scope_for_symbol(symbols, symbol_id) else {
        return members;
    };

    let scope = symbols.get_scope_by_id(owned_scope_id);

    // collect named members from the scope
    for (key, member_id) in symbols.active_named_symbols(scope) {
        let member_symbol = symbols.get_symbol(member_id);

        // only include value space symbols (methods, fields)
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

    // return the collected members
    members
}

/// Resolve extension members for a target symbol across all modules.
pub(crate) fn resolve_extension_members_for_symbol(
    session: &Session,
    target_symbol: GlobalSymbolId,
    current_module_id: ModuleId,
) -> Vec<MemberInfo> {
    // prepare member collection
    let mut members = Vec::new();
    for_each_visible_extension(
        session,
        target_symbol,
        current_module_id,
        |ctx, extension| {
            // load symbol and node trees for extension lookup
            let symbols = ctx.symbols();
            let tree = ctx.tree();

            // resolve the extension declaration
            let ext_symbol = symbols.get_symbol(extension.symbol.local_id);
            let Some(ext_decl_id) = ext_symbol.primary_declaration else {
                return false;
            };
            let Ok(local_decl_id): Result<dir::LocalNodeId<Declaration>, _> =
                ext_decl_id.try_into()
            else {
                return false;
            };
            let ext_decl: &Declaration = tree.get(local_decl_id);

            // collect members declared on the extension
            let Some(member_ids) = ext_decl.member_ids() else {
                return false;
            };

            for member_node_id in member_ids {
                let member_node: &Member = tree.get(*member_node_id);

                // skip members without a simple name
                let Some(key) = member_node.key() else {
                    continue;
                };
                let name = match key {
                    DynamicKey::Name(name_id) | DynamicKey::Number(name_id) => {
                        session.strings.get(*name_id).to_string()
                    }
                    _ => continue,
                };

                // map to member info kinds we support in completions
                let kind = match member_node {
                    Member::Method { .. } => MemberKind::Method,
                    Member::Field { .. } => MemberKind::Field,
                    _ => continue,
                };

                // record the extension member
                let member_symbol_id = GlobalSymbolId {
                    module_id: ctx.module_id,
                    local_id: member_node.symbol(),
                };

                members.push(MemberInfo {
                    name: MemberName::String(name),
                    type_id: None,
                    kind,
                    is_optional: false,
                    is_readonly: false,
                    symbol_id: Some(member_symbol_id),
                });
            }

            false
        },
    );

    members
}

/// Convert a static key to a member name, resolving the string ID to an actual string.
fn static_key_to_member_name(key: &StaticKey, strings: &destack_base::StringPool) -> MemberName {
    // convert the key into a displayable member name
    match key {
        StaticKey::Name(string_id) => MemberName::String(strings.get(*string_id).to_string()),
        StaticKey::Number(string_id) => MemberName::String(strings.get(*string_id).to_string()),
        StaticKey::Symbol(_) => MemberName::Computed,
    }
}

/// Check if a type is a function type.
fn is_function_type(types: &TypeTable, type_id: LocalTypeId) -> bool {
    // return true for function types
    matches!(types.get_type(type_id), Type::Function { .. })
}

/// Check if two member names match.
fn member_names_match(a: &MemberName, b: &MemberName) -> bool {
    // compare member names by variant
    match (a, b) {
        (MemberName::String(a), MemberName::String(b)) => a == b,
        (MemberName::Index(a), MemberName::Index(b)) => a == b,
        _ => false,
    }
}

/// Get members for array types by resolving the well known Array symbol.
fn array_members(
    _element_type: Option<LocalTypeId>,
    session: &Session,
    current_module_id: ModuleId,
) -> Vec<MemberInfo> {
    // resolve members from the Array well known symbol
    resolve_well_known_members(session, dir::WellKnownSymbol::Array, current_module_id)
}

/// Get members for primitive types by resolving the appropriate well known symbol.
fn primitive_members(
    value: &dir::TypeLiteral,
    session: &Session,
    current_module_id: ModuleId,
) -> Vec<MemberInfo> {
    // import primitive type helpers
    use destack_dir::{PrimitiveType, TypeLiteral, WellKnownSymbol};

    // map primitive type literals to their backing well known types
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

    // return members for the resolved well known symbol
    well_known
        .map(|wk| resolve_well_known_members(session, wk, current_module_id))
        .unwrap_or_default()
}

/// Resolve members from a well known symbol (Array, String, etc.).
fn resolve_well_known_members(
    session: &Session,
    well_known: WellKnownSymbol,
    current_module_id: ModuleId,
) -> Vec<MemberInfo> {
    // try to find well known symbols from any profile
    // (NOTE #Broken?: LSP queries don't have a specific profile context)
    if let Some(symbol_id) = session.builtins.first_well_known_type_symbol(well_known) {
        return resolve_reference_members(symbol_id, session, current_module_id);
    }

    // fall back to empty members when no well known symbol is available
    Vec::new()
}
