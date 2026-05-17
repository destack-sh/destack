use destack_dir as dir;
use destack_source::{FileId, NodeSpanRegion, NodeSpanType, Span, Uri};
use destack_workspace::{Repository, Revision};
use serde::{Deserialize, Serialize};

use crate::core::{QueryContext, query_context};
use crate::dir::{
    SymbolAtOffset, binding_symbol_at_offset, dependency_symbol_target, find_symbol_at_offset,
    get_canonical_symbol, get_symbol_definition_span, get_symbol_local_definition_span,
    semantic_target_symbol_at_offset, type_definition_span_for_symbol,
};
use crate::source::{get_module_by_file_id, get_node_tree_main_span};
use destack_dir::{DependencyItem, Expression, GlobalNodeIdAny, NodeType, Resolution};
/// Result of a goto definition query.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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

/// Request goto definition at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoDefinitionRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
}

/// Response payload for goto definition queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoDefinitionResponse {
    /// Definition locations, if any.
    pub result: Option<DefinitionResult>,
}

/// Request goto declaration at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoDeclarationRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
}

/// Response payload for goto declaration queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoDeclarationResponse {
    /// Declaration locations, if any.
    pub result: Option<DefinitionResult>,
}

/// Request goto type definition at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoTypeDefinitionRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
}

/// Response payload for goto type definition queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoTypeDefinitionResponse {
    /// Type definition locations, if any.
    pub result: Option<DefinitionResult>,
}

/// Find the definition of the symbol at the given position.
///
/// Returns the location(s) where the symbol is defined.
/// For imports, follows to the original definition.
pub fn goto_definition(
    repository: &Repository,
    revision: Revision,
    file: FileId,
    offset: u32,
) -> Option<DefinitionResult> {
    // resolve import-specifier definitions before generic symbol lookup
    if let Some(span) = resolve_import_definition_at_offset(repository, revision, file, offset) {
        return Some(DefinitionResult::single(span));
    }

    // find the symbol at the offset
    let symbol_at = find_symbol_at_offset(repository, revision, file, offset)?;

    // prefer overload declaration spans when call resolution selected a concrete signature
    if let Some(span) =
        overload_definition_span_for_call_site(repository, revision, file, &symbol_at)
    {
        return Some(DefinitionResult::single(span));
    }

    // get the definition span
    let symbol_id =
        semantic_target_symbol_at_offset(repository, revision, file, offset, &symbol_at)
            .unwrap_or(symbol_at.symbol_id);
    let span = get_symbol_definition_span(repository, revision, symbol_id)?;

    Some(DefinitionResult::single(span))
}

/// Resolve a definition span when the cursor is on an import dependency item.
fn resolve_import_definition_at_offset(
    repository: &Repository,
    revision: Revision,
    file: FileId,
    offset: u32,
) -> Option<Span> {
    // resolve the module and query context for this file
    let module = get_module_by_file_id(repository, revision, file)?;
    let ctx = query_context(repository, revision, module.id)?;
    let parsed = ctx.source();
    let dir_tree = ctx.dir().view();

    // scan dependency items and select the one at the cursor
    for item_id in dir_tree.iter_node_ids_of_type::<DependencyItem>() {
        // resolve the main declaration span for coarse overlap checks
        let fallback_span = get_node_tree_main_span(ctx.source(), ctx.dir().view(), item_id.into());

        // skip items that do not cover the cursor
        if !fallback_span.contains(offset) {
            continue;
        }

        // resolve side spans for imported-name and alias positions
        let source_id = dir_tree.get_source(item_id);
        let imported_name_span = parsed
            .tree()
            .get_side_span_by_id(source_id, NodeSpanType::Region(NodeSpanRegion::Type))
            .map(|span| Span::new(ctx.file_id(), span.start, span.end));
        let local_alias_span = parsed
            .tree()
            .get_side_span_by_id(source_id, NodeSpanType::Main)
            .map(|span| Span::new(ctx.file_id(), span.start, span.end));

        // only resolve definition targets from the imported name or local alias
        let is_symbol_span = imported_name_span.is_some_and(|span| span.contains(offset))
            || local_alias_span.is_some_and(|span| span.contains(offset));
        if !is_symbol_span {
            continue;
        }

        let target_symbol = dependency_symbol_target(ctx.dir(), item_id)?;

        return get_symbol_definition_span(repository, revision, target_symbol);
    }

    None
}

/// Find the declaration of the symbol at the given position.
///
/// For imports, returns the import statement location.
/// For locals, same as goto_definition.
pub fn goto_declaration(
    repository: &Repository,
    revision: Revision,
    file: FileId,
    offset: u32,
) -> Option<DefinitionResult> {
    // find the symbol at offset
    let symbol_at = find_symbol_at_offset(repository, revision, file, offset)?;
    let symbol_id = binding_symbol_at_offset(repository, revision, file, offset, &symbol_at)
        .unwrap_or(symbol_at.symbol_id);

    // get the declaration span
    let span = get_symbol_local_definition_span(repository, revision, symbol_id)?;

    Some(DefinitionResult::single(span))
}

/// Find the type definition of the symbol at the given position.
///
/// For a variable, returns the location of its type's definition.
/// For a type, returns the type itself.
pub fn goto_type_definition(
    repository: &Repository,
    revision: Revision,
    file: FileId,
    offset: u32,
) -> Option<DefinitionResult> {
    // find the symbol at the offset
    let symbol_at = find_symbol_at_offset(repository, revision, file, offset)?;
    let symbol_id = binding_symbol_at_offset(repository, revision, file, offset, &symbol_at)
        .unwrap_or(symbol_at.symbol_id);

    // use the canonical symbol when it resolves to a type
    let canonical_id = get_canonical_symbol(repository, revision, symbol_id);
    if let Some(span) = type_definition_span_for_symbol(repository, revision, canonical_id) {
        return Some(DefinitionResult::single(span));
    }

    // get the module to access type table
    let ctx = query_context(repository, revision, symbol_id.module_id)?;

    if let Some(span) = type_definition_span_for_symbol(repository, revision, symbol_id) {
        return Some(DefinitionResult::single(span));
    }

    // for non-type symbols (variables, parameters, etc.), look up their value type
    let resolved_type_symbol = {
        let types = ctx.dir().types();

        // try get_value_type_id first
        if let Some(type_id) = types.get_value_type_id(symbol_id) {
            resolve_nominal_type_symbol(types, type_id)
        }
        // otherwise fall back to declared or inferred type
        else {
            let node_id = symbol_at.node_id.into_global(symbol_id.module_id);
            let type_id = types.get_declared_or_inferred_type_id(node_id)?;
            resolve_nominal_type_symbol(types, type_id)
        }
    };
    if let Some(type_symbol) = resolved_type_symbol {
        let span = get_symbol_definition_span(repository, revision, type_symbol)?;
        return Some(DefinitionResult::single(span));
    }
    None
}

/// Resolve the nominal symbol for a possibly wrapped type.
fn resolve_nominal_type_symbol(
    types: &dir::TypeTable<'_>,
    type_id: dir::LocalTypeId,
) -> Option<dir::GlobalSymbolId> {
    match types.get_type(type_id) {
        dir::Type::Reference(reference) => Some(reference.symbol),
        dir::Type::Value(value) => resolve_nominal_type_symbol(types, value.value),
        dir::Type::KeyOf(unary)
        | dir::Type::Must(unary)
        | dir::Type::AsComptime(unary)
        | dir::Type::Not(unary) => resolve_nominal_type_symbol(types, unary.target_type),
        dir::Type::Form(form) => resolve_nominal_type_symbol(types, form.base),
        dir::Type::In(binary) | dir::Type::Extends(binary) | dir::Type::Implements(binary) => {
            resolve_nominal_type_symbol(types, binary.left)
                .or_else(|| resolve_nominal_type_symbol(types, binary.right))
        }
        dir::Type::Conditional(conditional) => resolve_nominal_type_symbol(types, conditional.left)
            .or_else(|| resolve_nominal_type_symbol(types, conditional.right))
            .or_else(|| resolve_nominal_type_symbol(types, conditional.then_type))
            .or_else(|| resolve_nominal_type_symbol(types, conditional.else_type)),
        dir::Type::Union(union) => {
            for element in &union.elements {
                if let Some(symbol_id) = resolve_nominal_type_symbol(types, *element) {
                    return Some(symbol_id);
                }
            }

            None
        }
        dir::Type::Intersection(intersection) => {
            for element in &intersection.elements {
                if let Some(symbol_id) = resolve_nominal_type_symbol(types, *element) {
                    return Some(symbol_id);
                }
            }

            None
        }
        ty => ty.symbol(),
    }
}

/// Resolve an overload definition span for the selected call site candidate.
fn overload_definition_span_for_call_site(
    repository: &Repository,
    revision: Revision,
    file: FileId,
    symbol_at: &SymbolAtOffset,
) -> Option<Span> {
    // require a local expression node at the cursor
    if symbol_at.node_id.ty != NodeType::Expression {
        return None;
    }

    let expression_id: dir::LocalNodeId<Expression> = symbol_at.node_id.try_into().ok()?;

    // resolve the query context for this file
    let module = get_module_by_file_id(repository, revision, file)?;
    let ctx = query_context(repository, revision, module.id)?;
    let dir_tree = ctx.dir().view();

    // require a call/new parent where this expression is the callee
    let parent = dir_tree.get_parent_for(expression_id)?;
    if parent.ty != NodeType::Expression {
        return None;
    }
    let parent_expression_id: dir::LocalNodeId<Expression> = parent.try_into().ok()?;
    let parent_expression = dir_tree.get::<Expression>(parent_expression_id);
    let is_callee = matches!(
        parent_expression,
        Expression::Call { left, .. } | Expression::New { left, .. } if *left == expression_id
    );
    if !is_callee {
        return None;
    }

    // resolve the selected call candidate signature
    let (target_symbol, parameters) = {
        let types = ctx.dir().types();
        let node_id = GlobalNodeIdAny {
            module_id: ctx.module_id(),
            local_id: parent_expression_id.into(),
        };
        let resolution = types.resolution(node_id)?;
        match resolution {
            Resolution::Dispatch(dir::DispatchResolution::Static { target, .. }) => Some((
                get_canonical_symbol(repository, revision, target.symbol),
                target.signature.as_ref()?.parameters.clone(),
            )),
            _ => None,
        }
    }?;

    // only match declaration signatures within the target symbol module
    if target_symbol.module_id != ctx.module_id() {
        return None;
    }

    overload_declaration_span_for_signature(repository, revision, target_symbol, &parameters)
}

/// Resolve an overload declaration span by matching parameter type ids.
fn overload_declaration_span_for_signature(
    repository: &Repository,
    revision: Revision,
    symbol_id: dir::GlobalSymbolId,
    parameter_types: &[dir::LocalTypeId],
) -> Option<Span> {
    // read the symbol context and declaration
    let ctx = query_context(repository, revision, symbol_id.module_id)?;
    let declaration = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.declaration
    };

    let mut declarations = Vec::new();
    if let Some(declaration) = declaration {
        declarations.push(declaration);
    }

    // find the declaration whose parameter type ids match the resolved signature
    for declaration in declarations {
        let Some(declaration_parameter_types) =
            declaration_parameter_type_ids(&ctx, declaration.local_id)
        else {
            continue;
        };

        if declaration_parameter_types.len() != parameter_types.len() {
            continue;
        }

        if declaration_parameter_types
            .iter()
            .zip(parameter_types.iter())
            .all(|(left, right)| left == right)
        {
            return Some(get_node_tree_main_span(
                ctx.source(),
                ctx.dir().view(),
                declaration.local_id,
            ));
        }
    }

    None
}

/// Resolve declared parameter type ids for a declaration or method member.
fn declaration_parameter_type_ids(
    ctx: &QueryContext<'_>,
    declaration_id: dir::LocalNodeIdAny,
) -> Option<Vec<dir::LocalTypeId>> {
    let dir_tree = ctx.dir().view();
    let types = ctx.dir().types();

    let parameters = match declaration_id.ty {
        NodeType::Declaration => {
            let declaration_id = declaration_id.try_into().ok()?;
            let declaration = dir_tree.get::<dir::Declaration>(declaration_id);
            let dir::Declaration::Function(declaration) = declaration else {
                return None;
            };
            declaration.signature.parameters.clone()
        }
        NodeType::Member => {
            let member_id = declaration_id.try_into().ok()?;
            let member = dir_tree.get::<dir::Member>(member_id);
            let signature = member.signature()?;
            signature.parameters.clone()
        }
        _ => return None,
    };

    let mut parameter_types = Vec::with_capacity(parameters.len());
    for parameter_id in parameters {
        let global_parameter_id = parameter_id.into_global_any(ctx.module_id());
        let type_id = types.get_declared_type_id(global_parameter_id)?;
        parameter_types.push(type_id);
    }

    Some(parameter_types)
}
