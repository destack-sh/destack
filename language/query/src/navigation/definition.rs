use destack_dir as dir;
use destack_source::{NodeSpanRegion, NodeSpanType, Span};
use serde::{Deserialize, Serialize};

use crate::core::{ModuleQueryContext, QueryPosition, QueryTarget};
use crate::dir::SymbolAtOffset;

/// Relationship between a navigation origin and target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NavigationRelation {
    /// Declaration target.
    Declaration,
    /// Definition target.
    Definition,
    /// Type definition target.
    TypeDefinition,
    /// Implementation target.
    Implementation,
}

/// One navigation target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavigationTarget {
    /// The target location and resolved identity.
    pub target: QueryTarget,
    /// The relationship to the query origin.
    pub relation: NavigationRelation,
}

impl NavigationTarget {
    /// Create a navigation target from a source span.
    pub fn span(ctx: &ModuleQueryContext<'_>, span: Span, relation: NavigationRelation) -> Self {
        let module = ctx.query_module();
        let target = QueryTarget::span(module, span);
        Self { target, relation }
    }

    /// Return this navigation target with a symbol id.
    pub fn with_symbol(mut self, symbol_id: dir::GlobalSymbolId) -> Self {
        self.target = self.target.with_symbol(symbol_id);

        self
    }
}

/// Request goto definition at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoDefinitionRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// Response payload for goto definition queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoDefinitionResponse {
    /// Definition targets.
    pub targets: Vec<NavigationTarget>,
}

/// Request goto declaration at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoDeclarationRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// Response payload for goto declaration queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoDeclarationResponse {
    /// Declaration targets.
    pub targets: Vec<NavigationTarget>,
}

/// Request goto type definition at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoTypeDefinitionRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// Response payload for goto type definition queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoTypeDefinitionResponse {
    /// Type definition targets.
    pub targets: Vec<NavigationTarget>,
}

impl ModuleQueryContext<'_> {
    /// Find the definition of the symbol at the given position.
    ///
    /// Returns the location(s) where the symbol is defined.
    /// For imports, follows to the original definition.
    pub fn goto_definition(&self, offset: u32) -> Vec<NavigationTarget> {
        let _ctx = self;
        // resolve import-specifier definitions before generic symbol lookup
        if let Some(span) = self.resolve_import_definition_at_offset(offset) {
            return vec![NavigationTarget::span(
                self,
                span,
                NavigationRelation::Definition,
            )];
        }

        // find the symbol at the offset
        let Some(symbol_at) = self.find_symbol_at_offset(offset) else {
            return Vec::new();
        };

        // prefer overload declaration spans when call resolution selected a concrete signature
        if let Some(span) = self.overload_definition_span_for_call_site(&symbol_at) {
            return vec![NavigationTarget::span(
                self,
                span,
                NavigationRelation::Definition,
            )];
        }

        // get the definition span
        let symbol_id = self
            .semantic_target_symbol_at_offset(offset, &symbol_at)
            .unwrap_or(symbol_at.symbol_id);
        let Some(ctx) = self.module_context(symbol_id.module_id) else {
            return Vec::new();
        };
        let Some(span) = ctx.symbol_definition_span(symbol_id) else {
            return Vec::new();
        };

        vec![
            NavigationTarget::span(&ctx, span, NavigationRelation::Definition)
                .with_symbol(symbol_id),
        ]
    }

    /// Resolve a definition span when the cursor is on an import dependency item.
    fn resolve_import_definition_at_offset(&self, offset: u32) -> Option<Span> {
        let dir = self.dir();
        let dir_tree = dir.view();

        // scan dependency items and select the one at the cursor
        for item_id in dir_tree.iter_node_ids_of_type::<dir::DependencyItem>() {
            // resolve the main declaration span for coarse overlap checks
            let item_span = dir.get_node_tree_main_span(dir.view(), item_id.into());

            // skip items that do not cover the cursor
            if !item_span.contains(offset) {
                continue;
            }

            // resolve side spans for imported-name and alias positions
            let source_id = dir_tree.get_source(item_id);
            let imported_name_span = dir
                .tree()
                .get_side_span_by_id(source_id, NodeSpanType::Region(NodeSpanRegion::Type))
                .map(|span| Span::new(dir.file_id(), span.start, span.end));
            let local_alias_span = dir
                .tree()
                .get_side_span_by_id(source_id, NodeSpanType::Main)
                .map(|span| Span::new(dir.file_id(), span.start, span.end));

            // only resolve definition targets from the imported name or local alias
            let is_symbol_span = imported_name_span.is_some_and(|span| span.contains(offset))
                || local_alias_span.is_some_and(|span| span.contains(offset));
            if !is_symbol_span {
                continue;
            }

            let target_symbol = dir.dependency_symbol_target(item_id)?;

            return self.symbol_definition_span(target_symbol);
        }

        None
    }

    /// Find the declaration of the symbol at the given position.
    ///
    /// For imports, returns the import statement location.
    /// For locals, same as goto_definition.
    pub fn goto_declaration(&self, offset: u32) -> Vec<NavigationTarget> {
        let _ctx = self;
        // find the symbol at offset
        let Some(symbol_at) = self.find_symbol_at_offset(offset) else {
            return Vec::new();
        };
        let symbol_id = self
            .binding_symbol_at_offset(offset, &symbol_at)
            .unwrap_or(symbol_at.symbol_id);

        // get the declaration span
        let Some(ctx) = self.module_context(symbol_id.module_id) else {
            return Vec::new();
        };
        let Some(span) = ctx.symbol_local_definition_span(symbol_id) else {
            return Vec::new();
        };

        vec![
            NavigationTarget::span(&ctx, span, NavigationRelation::Declaration)
                .with_symbol(symbol_id),
        ]
    }

    /// Find the type definition of the symbol at the given position.
    ///
    /// For a variable, returns the location of its type's definition.
    /// For a type, returns the type itself.
    pub fn goto_type_definition(&self, offset: u32) -> Vec<NavigationTarget> {
        let _ctx = self;
        // find the symbol at the offset
        let Some(symbol_at) = self.find_symbol_at_offset(offset) else {
            return Vec::new();
        };
        let symbol_id = self
            .binding_symbol_at_offset(offset, &symbol_at)
            .unwrap_or(symbol_at.symbol_id);

        // use the canonical symbol when it resolves to a type
        let Some(ctx) = self.module_context(symbol_id.module_id) else {
            return Vec::new();
        };
        let canonical_id = ctx.canonical_symbol(symbol_id);
        if let Some(span) = ctx.type_definition_span(canonical_id) {
            return vec![
                NavigationTarget::span(&ctx, span, NavigationRelation::TypeDefinition)
                    .with_symbol(canonical_id),
            ];
        }

        if let Some(span) = ctx.type_definition_span(symbol_id) {
            return vec![
                NavigationTarget::span(&ctx, span, NavigationRelation::TypeDefinition)
                    .with_symbol(symbol_id),
            ];
        }

        // for non-type symbols, look up their checked type
        let types = ctx.dir().types();
        let resolved_type_symbol = if let Some(type_id) = types.get_symbol_type_id(symbol_id) {
            ctx.resolve_nominal_type_symbol(type_id)
        } else {
            let node_id = symbol_at.node_id.into_global(symbol_id.module_id);
            let Some(type_id) = types.get_node_type_id(node_id) else {
                return Vec::new();
            };
            ctx.resolve_nominal_type_symbol(type_id)
        };

        if let Some(type_symbol) = resolved_type_symbol {
            let Some(span) = ctx.symbol_definition_span(type_symbol) else {
                return Vec::new();
            };
            return vec![
                NavigationTarget::span(&ctx, span, NavigationRelation::TypeDefinition)
                    .with_symbol(type_symbol),
            ];
        }
        Vec::new()
    }

    /// Return whether a declaration has matching parameter type ids.
    fn declaration_parameter_type_ids_match(
        &self,
        declaration_id: dir::LocalNodeIdAny,
        parameter_types: &[dir::GlobalTypeId],
    ) -> Option<bool> {
        let ctx = self;
        let dir_tree = ctx.dir().view();
        let types = ctx.dir().types();

        let parameters = match declaration_id.ty {
            dir::NodeType::Declaration => {
                let declaration_id = declaration_id.try_into().ok()?;
                let declaration = dir_tree.get::<dir::Declaration>(declaration_id);
                let dir::Declaration::Function(declaration) = declaration else {
                    return None;
                };
                declaration.signature.parameters.as_slice()
            }
            dir::NodeType::Member => {
                let member_id = declaration_id.try_into().ok()?;
                let member = dir_tree.get::<dir::Member>(member_id);
                let signature = member.signature()?;
                signature.parameters.as_slice()
            }
            _ => return None,
        };

        if parameters.len() != parameter_types.len() {
            return Some(false);
        }

        for (parameter_id, expected_type_id) in parameters.iter().zip(parameter_types.iter()) {
            let global_parameter_id = parameter_id.into_global_any(ctx.module_id());
            let type_id = types.get_node_type_id(global_parameter_id)?;
            if type_id != *expected_type_id {
                return Some(false);
            }
        }

        Some(true)
    }

    /// Resolve the nominal symbol for a possibly wrapped type.
    fn resolve_nominal_type_symbol(
        &self,
        type_id: dir::GlobalTypeId,
    ) -> Option<dir::GlobalSymbolId> {
        let ctx = self;
        ctx.with_global_type(type_id, |ty, _| match ty {
            dir::Type::Reference(reference) => Some(reference.symbol),
            dir::Type::Form(value) => ctx.resolve_nominal_type_symbol(value.value),
            dir::Type::Dynamic(dynamic) => ctx.resolve_nominal_type_symbol(dynamic.constraint),
            dir::Type::Operation(operation) => {
                ctx.resolve_nominal_type_symbol_from_operation(operation)
            }
            dir::Type::Union(union) => {
                for element in &union.elements {
                    if let Some(symbol_id) = ctx.resolve_nominal_type_symbol(*element) {
                        return Some(symbol_id);
                    }
                }

                None
            }
            dir::Type::Intersection(intersection) => {
                for element in &intersection.elements {
                    if let Some(symbol_id) = ctx.resolve_nominal_type_symbol(*element) {
                        return Some(symbol_id);
                    }
                }

                None
            }
            ty => ty.symbol(),
        })
        .flatten()
    }

    /// Resolve the nominal symbol for a type operation.
    fn resolve_nominal_type_symbol_from_operation(
        &self,
        operation: &dir::TypeOperation,
    ) -> Option<dir::GlobalSymbolId> {
        let ctx = self;
        match operation {
            dir::TypeOperation::KeyOf(unary) => ctx.resolve_nominal_type_symbol(unary.target),
            dir::TypeOperation::Conditional(conditional) => ctx
                .resolve_nominal_type_symbol(conditional.left)
                .or_else(|| ctx.resolve_nominal_type_symbol(conditional.right))
                .or_else(|| ctx.resolve_nominal_type_symbol(conditional.then_type))
                .or_else(|| ctx.resolve_nominal_type_symbol(conditional.else_type)),
            _ => None,
        }
    }

    /// Resolve an overload definition span for the selected call site candidate.
    fn overload_definition_span_for_call_site(&self, symbol_at: &SymbolAtOffset) -> Option<Span> {
        let ctx = self;

        // require a local expression node at the cursor
        if symbol_at.node_id.ty != dir::NodeType::Expression {
            return None;
        }

        let expression_id: dir::LocalNodeId<dir::Expression> = symbol_at.node_id.try_into().ok()?;

        let dir_tree = ctx.dir().view();

        // require a call parent where this expression is the callee
        let parent = dir_tree.get_parent_for(expression_id)?;
        if parent.ty != dir::NodeType::Expression {
            return None;
        }
        let parent_expression_id: dir::LocalNodeId<dir::Expression> = parent.try_into().ok()?;
        let parent_expression = dir_tree.get::<dir::Expression>(parent_expression_id);
        let is_callee = matches!(
            parent_expression,
            dir::Expression::Call { left, .. } if *left == expression_id
        );
        if !is_callee {
            return None;
        }

        // resolve the selected call candidate signature
        {
            let node_id = dir::GlobalNodeIdAny {
                module_id: ctx.module_id(),
                local_id: parent_expression_id.into(),
            };
            let resolution = ctx.dir().resolutions().call_resolution(node_id)?;
            match &resolution.target {
                dir::CallTarget::Symbol(candidate) => {
                    let target_symbol = ctx.canonical_symbol(candidate.symbol);
                    let parameters = resolution.parameters.as_slice();

                    // only match declaration signatures within the target symbol module
                    if target_symbol.module_id != ctx.module_id() {
                        return None;
                    }

                    ctx.overload_declaration_span_for_signature(target_symbol, parameters)
                }
                _ => None,
            }
        }
    }

    /// Resolve an overload declaration span by matching parameter type ids.
    fn overload_declaration_span_for_signature(
        &self,
        symbol_id: dir::GlobalSymbolId,
        parameter_types: &[dir::GlobalTypeId],
    ) -> Option<Span> {
        let ctx = self;
        // read the symbol declaration
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
            let Some(is_match) =
                ctx.declaration_parameter_type_ids_match(declaration.local_id, parameter_types)
            else {
                continue;
            };
            if is_match {
                let dir = ctx.dir();
                return Some(dir.get_node_tree_main_span(dir.view(), declaration.local_id));
            }
        }

        None
    }
}
