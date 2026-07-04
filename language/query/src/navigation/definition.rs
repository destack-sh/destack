use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{NodeSpanRegion, NodeSpanType, Span};
use serde::{Deserialize, Serialize};

use crate::{ModuleQueryContext, Position, SymbolHit, Target};

/// Relationship between a navigation origin and target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct NavigationTarget {
    /// The target location and resolved identity.
    pub target: Target,
    /// The relationship to the query origin.
    pub relation: NavigationRelation,
}

impl NavigationTarget {
    /// Create a navigation target from a source span.
    pub fn span(module: &ModuleQueryContext<'_>, span: Span, relation: NavigationRelation) -> Self {
        let target_module = module.module();
        let target = Target::new(target_module, span);
        Self { target, relation }
    }

    /// Return this navigation target with a symbol id.
    pub fn with_symbol(mut self, symbol_id: dir::GlobalSymbolId) -> Self {
        self.target = self.target.with_symbol_id(symbol_id);

        self
    }
}

/// Request goto definition at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoDefinitionRequest {
    /// The queried position.
    pub position: Position,
}

/// Response payload for goto definition queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoDefinitionResponse {
    /// Definition targets.
    pub targets: Vec<NavigationTarget>,
}

/// Request goto declaration at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoDeclarationRequest {
    /// The queried position.
    pub position: Position,
}

/// Response payload for goto declaration queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoDeclarationResponse {
    /// Declaration targets.
    pub targets: Vec<NavigationTarget>,
}

/// Request goto type definition at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoTypeDefinitionRequest {
    /// The queried position.
    pub position: Position,
}

/// Response payload for goto type definition queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
        if let Some(span) = self.call_site_overload_definition_span(&symbol_at) {
            return vec![NavigationTarget::span(
                self,
                span,
                NavigationRelation::Definition,
            )];
        }

        // get the definition span
        let symbol_id = self.semantic_target_symbol_at_offset(offset, &symbol_at);
        let module = self.module_context(symbol_id.module_id);
        let Some(span) = module.symbol_definition_span(symbol_id) else {
            return Vec::new();
        };

        vec![
            NavigationTarget::span(&module, span, NavigationRelation::Definition)
                .with_symbol(symbol_id),
        ]
    }

    /// Resolve a definition span when the cursor is on an import dependency item.
    fn resolve_import_definition_at_offset(&self, offset: u32) -> Option<Span> {
        let view = self.view();

        // scan dependency items and select the one at the cursor
        for item_id in view.iter_node_ids_of_type::<dir::DependencyItem>() {
            // resolve the main declaration span for coarse overlap checks
            let item_span = self.get_main_span(view, item_id.into());

            // skip items that do not cover the cursor
            if !item_span.contains(offset) {
                continue;
            }

            // resolve side spans for imported-name and alias positions
            let source_id = view.get_source(item_id);
            let imported_name_span = self
                .tree()
                .get_side_span_by_id(source_id, NodeSpanType::Region(NodeSpanRegion::Type))
                .map(|span| Span::new(self.file_id(), span.start, span.end));
            let local_alias_span = self
                .tree()
                .get_side_span_by_id(source_id, NodeSpanType::Main)
                .map(|span| Span::new(self.file_id(), span.start, span.end));

            // only resolve definition targets from the imported name or local alias
            let is_symbol_span = imported_name_span.is_some_and(|span| span.contains(offset))
                || local_alias_span.is_some_and(|span| span.contains(offset));
            if !is_symbol_span {
                continue;
            }

            let target_symbol = self.dependency_symbol_target(item_id)?;

            return self.symbol_definition_span(target_symbol);
        }

        None
    }

    /// Find the declaration of the symbol at the given position.
    ///
    /// For imports, returns the import statement location.
    /// For locals, same as goto_definition.
    pub fn goto_declaration(&self, offset: u32) -> Vec<NavigationTarget> {
        // find the symbol at offset
        let Some(symbol_at) = self.find_symbol_at_offset(offset) else {
            return Vec::new();
        };
        let symbol_id = self.binding_symbol_at_offset(offset, &symbol_at);

        // get the declaration span
        let module = self.module_context(symbol_id.module_id);
        let Some(span) = module.symbol_local_definition_span(symbol_id) else {
            return Vec::new();
        };

        vec![
            NavigationTarget::span(&module, span, NavigationRelation::Declaration)
                .with_symbol(symbol_id),
        ]
    }

    /// Find the type definition of the symbol at the given position.
    ///
    /// For a variable, returns the location of its type's definition.
    /// For a type, returns the type itself.
    pub fn goto_type_definition(&self, offset: u32) -> Vec<NavigationTarget> {
        // find the symbol at the offset
        let Some(symbol_at) = self.find_symbol_at_offset(offset) else {
            return Vec::new();
        };
        let symbol_id = self.binding_symbol_at_offset(offset, &symbol_at);

        // use the canonical symbol when it resolves to a type
        let module = self.module_context(symbol_id.module_id);
        let canonical_id = module.canonical_symbol(symbol_id);
        if let Some(span) = module.type_definition_span(canonical_id) {
            return vec![
                NavigationTarget::span(&module, span, NavigationRelation::TypeDefinition)
                    .with_symbol(canonical_id),
            ];
        }

        if let Some(span) = module.type_definition_span(symbol_id) {
            return vec![
                NavigationTarget::span(&module, span, NavigationRelation::TypeDefinition)
                    .with_symbol(symbol_id),
            ];
        }

        // for non-type symbols, look up their checked type
        let types = module.types();
        let resolved_type_symbol = if let Some(type_id) = types.get_symbol_type_id(symbol_id) {
            module.resolve_nominal_type_symbol(type_id)
        } else {
            let node_id = symbol_at.node_id.into_global(symbol_id.module_id);
            let Some(type_id) = types.get_node_type_id(node_id) else {
                return Vec::new();
            };
            module.resolve_nominal_type_symbol(type_id)
        };

        if let Some(type_symbol) = resolved_type_symbol {
            let Some(span) = module.symbol_definition_span(type_symbol) else {
                return Vec::new();
            };
            return vec![
                NavigationTarget::span(&module, span, NavigationRelation::TypeDefinition)
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
        let view = self.view();
        let types = self.types();

        let parameters = match declaration_id.ty {
            dir::NodeType::Declaration => {
                let declaration_id = declaration_id.try_into().unwrap_or_else(|_| {
                    panic!("call target declaration id has incompatible type: {declaration_id:?}")
                });
                let declaration = view.get::<dir::Declaration>(declaration_id);
                let dir::Declaration::Function(declaration) = declaration else {
                    return None;
                };
                declaration.signature.parameters.as_slice()
            }
            dir::NodeType::Member => {
                let member_id = declaration_id.try_into().unwrap_or_else(|_| {
                    panic!("call target member id has incompatible type: {declaration_id:?}")
                });
                let member = view.get::<dir::Member>(member_id);
                let signature = member.signature()?;
                signature.parameters.as_slice()
            }
            _ => return None,
        };

        if parameters.len() != parameter_types.len() {
            return Some(false);
        }

        for (parameter_id, expected_type_id) in parameters.iter().zip(parameter_types.iter()) {
            let global_parameter_id = parameter_id.into_global_any(self.module_id());
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
        self.read_global_type(type_id, |ty, type_module| match ty {
            dir::Type::Instance(reference) => Some(reference.symbol),
            dir::Type::Form(value) => self.resolve_nominal_type_symbol(value.value),
            dir::Type::Dynamic(dynamic) => self.resolve_nominal_type_symbol(dynamic.constraint),
            dir::Type::Operation(operation) => {
                self.resolve_nominal_type_symbol_from_operation(operation)
            }
            dir::Type::Union(union) => {
                for element in type_module.types().type_ids(union.elements) {
                    if let Some(symbol_id) = self.resolve_nominal_type_symbol(*element) {
                        return Some(symbol_id);
                    }
                }

                None
            }
            dir::Type::Intersection(intersection) => {
                for element in type_module.types().type_ids(intersection.elements) {
                    if let Some(symbol_id) = self.resolve_nominal_type_symbol(*element) {
                        return Some(symbol_id);
                    }
                }

                None
            }
            ty => ty.symbol(),
        })
    }

    /// Resolve the nominal symbol for a type operation.
    fn resolve_nominal_type_symbol_from_operation(
        &self,
        operation: &dir::TypeOperation,
    ) -> Option<dir::GlobalSymbolId> {
        match operation {
            dir::TypeOperation::KeyOf(unary) => self.resolve_nominal_type_symbol(unary.target),
            dir::TypeOperation::NoInfer(unary) => self.resolve_nominal_type_symbol(unary.target),
            dir::TypeOperation::Conditional(conditional) => self
                .resolve_nominal_type_symbol(conditional.left)
                .or_else(|| self.resolve_nominal_type_symbol(conditional.right))
                .or_else(|| self.resolve_nominal_type_symbol(conditional.then_type))
                .or_else(|| self.resolve_nominal_type_symbol(conditional.else_type)),
            _ => None,
        }
    }

    /// Resolve an overload definition span for the selected call site candidate.
    fn call_site_overload_definition_span(&self, symbol_at: &SymbolHit) -> Option<Span> {
        // require a local expression node at the cursor
        if symbol_at.node_id.ty != dir::NodeType::Expression {
            return None;
        }

        let expression_id: dir::LocalNodeId<dir::Expression> =
            symbol_at.node_id.try_into().unwrap_or_else(|_| {
                panic!(
                    "symbol-at node is not an expression: {:?}",
                    symbol_at.node_id
                )
            });

        let view = self.view();

        // require a call parent where this expression is the callee
        let parent = view.get_parent_for(expression_id)?;
        if parent.ty != dir::NodeType::Expression {
            return None;
        }
        let parent_expression_id: dir::LocalNodeId<dir::Expression> = parent
            .try_into()
            .unwrap_or_else(|_| panic!("call parent node is not an expression: {parent:?}"));
        let parent_expression = view.get::<dir::Expression>(parent_expression_id);
        let is_callee = matches!(
            parent_expression,
            dir::Expression::Call { left, .. } if *left == expression_id
        );
        if !is_callee {
            return None;
        }

        // resolve the selected call candidate signature
        {
            let node_id = parent_expression_id.into_global_any(self.module_id());
            let resolution = self.resolutions().call_resolution(node_id)?;
            match &resolution.target {
                dir::CallTarget::Symbol(candidate) => {
                    let target_symbol = self.canonical_symbol(candidate.symbol);
                    let parameters = resolution.parameters.as_slice();

                    // only match declaration signatures within the target symbol module
                    if target_symbol.module_id != self.module_id() {
                        return None;
                    }

                    self.signature_overload_declaration_span(target_symbol, parameters)
                }
                _ => None,
            }
        }
    }

    /// Resolve an overload declaration span by matching parameter type ids.
    fn signature_overload_declaration_span(
        &self,
        symbol_id: dir::GlobalSymbolId,
        parameter_types: &[dir::GlobalTypeId],
    ) -> Option<Span> {
        // read the symbol declaration
        let declaration = {
            let symbols = self.symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration
        };

        let declaration = declaration?;

        // find the declaration whose parameter type ids match the resolved signature
        let is_match =
            self.declaration_parameter_type_ids_match(declaration.local_id, parameter_types)?;
        if is_match {
            return Some(self.get_main_span(self.view(), declaration.local_id));
        }

        None
    }
}
