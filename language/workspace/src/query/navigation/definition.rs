use destack_source::{FileId, Span};

use crate::query::common::{find_symbol_at_offset, get_dir_node_span, get_symbol_definition_span};
use crate::{ModuleAst, ModuleDir, Session};
use destack_dir::{self as dir, Declarator, Expression, NodeType};

/// Result of a goto definition query.
#[derive(Debug, Clone, Default)]
pub struct DefinitionResult {
    /// The definition location(s).
    /// Multiple locations for overloaded symbols or partial definitions.
    pub locations: Vec<Span>,
}

impl DefinitionResult {
    /// Create an empty result.
    pub fn empty() -> Self {
        Self {
            locations: Vec::new(),
        }
    }

    /// Create a result with a single span.
    pub fn single(span: Span) -> Self {
        Self {
            locations: vec![span],
        }
    }

    /// Whether any definitions were found.
    pub fn is_empty(&self) -> bool {
        self.locations.is_empty()
    }
}

/// Find the definition of the symbol at the given position.
///
/// Returns the location(s) where the symbol is defined.
/// For imports, follows to the original definition.
pub fn goto_definition(session: &Session, file: FileId, offset: u32) -> Option<DefinitionResult> {
    // find the symbol at the offset
    let symbol_at = find_symbol_at_offset(session, file, offset)?;

    // get the definition span
    let span = get_symbol_definition_span(session, symbol_at.symbol_id)?;

    Some(DefinitionResult::single(span))
}

/// Find the declaration of the symbol at the given position.
///
/// For imports, returns the import statement location.
/// For locals, same as goto_definition.
pub fn goto_declaration(session: &Session, file: FileId, offset: u32) -> Option<DefinitionResult> {
    // find the symbol at offset
    let symbol_at = find_symbol_at_offset(session, file, offset)?;

    // get the declaration span
    let span = get_declaration_span(session, symbol_at.symbol_id)?;

    Some(DefinitionResult::single(span))
}

/// Get the declaration span without following canonical_symbol.
fn get_declaration_span(session: &Session, symbol_id: dir::GlobalSymbolId) -> Option<Span> {
    // get module and dir
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let (Some(ast), Some(dir)) = (&module.ast, &module.dir) else {
        return None;
    };
    let symbols = dir.symbols.read();
    let symbol = symbols.get_symbol(symbol_id.local_id);

    // get primary_declaration directly
    let declaration = symbol.primary_declaration?;

    drop(symbols);

    get_dir_node_span(ast, dir, declaration.local_id)
}

/// Find the type definition of the symbol at the given position.
///
/// For a variable, returns the location of its type's definition.
/// For a type, returns the type itself.
pub fn goto_type_definition(
    session: &Session,
    file: FileId,
    offset: u32,
) -> Option<DefinitionResult> {
    // find the symbol at the offset
    let symbol_at = find_symbol_at_offset(session, file, offset)?;
    let symbol_id = symbol_at.symbol_id;

    // get the module to access type table
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let (Some(ast), Some(dir)) = (&module.ast, &module.dir) else {
        return None;
    };

    // check if the symbol itself is a type symbol (class, struct, enum, etc.)
    let symbols = dir.symbols.read();
    let symbol = symbols.get_symbol(symbol_id.local_id);

    // go directly to the definition of the type symbol (if it's a type or type-value)
    if symbol.space == dir::SymbolSpace::Type || symbol.space == dir::SymbolSpace::TypeValue {
        drop(symbols);
        drop(module);
        let span = get_symbol_definition_span(session, symbol_id)?;
        return Some(DefinitionResult::single(span));
    }

    drop(symbols);

    // for non-type symbols (variables, parameters, etc.), look up their value type
    let types = dir.types.read();
    if let Some(type_id) = types.get_value_type_id(symbol_id) {
        let ty = types.get_type(type_id);
        if let Some(type_symbol) = ty.symbol() {
            drop(types);
            drop(module);
            let span = get_symbol_definition_span(session, type_symbol)?;
            return Some(DefinitionResult::single(span));
        }
    }
    drop(types);

    // NOTE #Incomplete: get_value_type_id doesn't populate types for all variables/parameters yet
    // (we can remove this once type inference populates value types for all symbols)
    if let Some(type_symbol) =
        get_type_from_declaration_context(session, ast, dir, symbol_at.node_id)
    {
        drop(module);
        let span = get_symbol_definition_span(session, type_symbol)?;
        return Some(DefinitionResult::single(span));
    }

    None
}

/// Get the type symbol from the declaration context of a node.
/// (we can remove this once type inference populates value types for all symbols, see goto_type_definition)
fn get_type_from_declaration_context(
    session: &Session,
    _ast: &ModuleAst,
    dir: &ModuleDir,
    node_id: dir::LocalNodeIdAny,
) -> Option<dir::GlobalSymbolId> {
    let dir_tree = dir.tree.read();
    let types = dir.types.read();

    match node_id.ty {
        // for patterns, find parent declarator and get its type annotation
        NodeType::Pattern => {
            // look for a parent Declarator
            if let Some(parent_node_id) = dir_tree.get_parent(node_id.id)
                && parent_node_id.ty == NodeType::Declarator
            {
                let declarator_id = parent_node_id.try_into_typed().ok()?;
                let declarator = dir_tree.get::<Declarator>(declarator_id);

                // get the type expression
                if let Some(ty_expr_id) = declarator.ty {
                    let ty_expr = dir_tree.get::<Expression>(ty_expr_id);
                    // get target_symbol from the type expression
                    return ty_expr.target_symbol();
                }
            }
        }
        // for parameters, get the declared type
        NodeType::Parameter => {
            // parameters have their declared type stored in TypeTable
            let global_node_id = dir::GlobalNodeIdAny {
                module_id: dir.id,
                local_id: node_id,
            };
            if let Some(type_id) = types.get_declared_type_id(global_node_id) {
                let ty = types.get_type(type_id);
                return ty.symbol();
            }
        }
        // for expressions that are type references, get the target_symbol
        NodeType::Expression => {
            let expr_id = node_id.try_into_typed().ok()?;
            let expr = dir_tree.get::<Expression>(expr_id);

            // check if this is a type reference (GlobalReference, etc.)
            if let Some(target) = expr.target_symbol() {
                drop(types);
                drop(dir_tree);
                // verify target is a type symbol
                let target_module = session.modules.get(target.module_id);
                let target_module = target_module.read();
                let Some(target_dir) = &target_module.dir else {
                    return None;
                };
                let symbols = target_dir.symbols.read();
                let symbol = symbols.get_symbol(target.local_id);

                if symbol.space == dir::SymbolSpace::Type
                    || symbol.space == dir::SymbolSpace::TypeValue
                {
                    return Some(target);
                }
            }
        }
        _ => {}
    }

    None
}
