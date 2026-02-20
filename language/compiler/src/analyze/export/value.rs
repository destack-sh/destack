use crate::analyze::common::ConstContext;
use crate::{AnalyzeError, AnalyzeResult, AnalyzeWarning, Compiler, InferContext};
use destack_dir::{
    Constraint, Declaration, Declarator, Export, Expression, GlobalNodeIdAny, GlobalSymbolId,
    InferOrigin, InferScope, InferTable, LocalNodeId, LocalTypeId, NodeTree, NodeType, NodeVisitor,
    NodeVisitorOptions, StaticKey, SymbolSpace, SymbolTable, Type, TypeLiteral, TypeTable,
    walk_expression,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

/// Track an exported declarator that needs surface inference.
#[derive(Debug)]
struct ExportInference {
    /// The exported symbol to assign a value type.
    export_symbol: GlobalSymbolId,
    /// The declarator that owns the binding.
    declarator_id: LocalNodeId<Declarator>,
    /// The initializer expression when present.
    value_id: Option<LocalNodeId<Expression>>,
    /// The binding mutability when available.
    binding_mutability: Option<destack_dir::Mutability>,
    /// Whether the initializer is a const assertion.
    is_const_asserted: bool,
}

/// Track a function declaration that needs return inference.
#[derive(Debug)]
struct ExportDeclarationInference {
    /// The declaration id to infer.
    declaration_id: LocalNodeId<Declaration>,
}

/// Collect remote references in exported initializers.
#[derive(Debug)]
struct ExportInferenceReferenceCollector<'a> {
    /// The module being analyzed.
    module: &'a Module,
    /// The symbol table for the module.
    symbols: &'a SymbolTable,
    /// Remote symbols referenced by the export initializer.
    references: Vec<GlobalSymbolId>,
    /// Node visitor options.
    options: NodeVisitorOptions,
}

impl<'a> ExportInferenceReferenceCollector<'a> {
    /// Create a new export inference collector.
    fn new(module: &'a Module, symbols: &'a SymbolTable) -> Self {
        // initialize the collector state
        Self {
            module,
            symbols,
            references: Vec::new(),
            options: NodeVisitorOptions::default(),
        }
    }
}

impl NodeVisitor for ExportInferenceReferenceCollector<'_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // collect remote references for export inference
        if let Some(target_symbol) = expression.target_symbol() {
            if target_symbol.module_id != self.module.id {
                self.references.push(target_symbol);
            } else if let Some(imported_symbol) = self
                .symbols
                .get_symbol(target_symbol.local_id)
                .target_symbol
            {
                self.references.push(imported_symbol);
            }
        }

        destack_base::ensure_sufficient_stack(|| {
            walk_expression(self, tree, id, expression);
        });
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Infer exported value types using local information only.
    pub(crate) fn infer_exported_value_types(
        &self,
        module: &Module,
        profile: ProfileId,
        exported_symbols: &indexmap::IndexMap<(SymbolSpace, StaticKey), Export>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // skip export inference for declaration modules
        if module.language_type.is_declaration() {
            for export in exported_symbols.values() {
                let Some((export_symbol, value_symbol)) =
                    self.export_inference_value_symbol(symbols, module.id, export)
                else {
                    continue;
                };

                if let Some(value_ty_id) = self.export_known_value_type_id(types, value_symbol) {
                    types.set_value_type(export_symbol, value_ty_id);
                    continue;
                }

                let Some(declarator_id) =
                    self.direct_binding_declarator_for_symbol(module, value_symbol, tree, symbols)
                else {
                    continue;
                };

                if let Some(declared_type_id) =
                    types.get_declared_type_id(declarator_id.into_global_any(module.id))
                {
                    types.set_value_type(export_symbol, declared_type_id);
                }
            }

            return Ok(());
        }

        // prepare surface inference for exported values
        let options = self.analyze_context_options_for_module(module.id);
        let mut infer = InferTable::default();
        let base_ctx = InferContext::new(profile, options).for_surface_inference();
        let mut inferred_exports = Vec::new();
        let mut export_inference = Vec::new();
        let mut export_declarations = Vec::new();

        // collect exported symbols that need value types
        for export in exported_symbols.values() {
            // resolve the local export symbol for value inference
            let Some((export_symbol, value_symbol)) =
                self.export_inference_value_symbol(symbols, module.id, export)
            else {
                continue;
            };

            // read the primary declaration for the export symbol
            let symbol_entry = symbols.get_symbol(value_symbol.local_id);
            let Some(primary_declaration) = symbol_entry.primary_declaration else {
                continue;
            };

            // defer unannotated function returns to declaration inference
            if let Some(declaration_id) =
                self.export_inference_function_declaration(tree, primary_declaration)
            {
                export_declarations.push(ExportDeclarationInference { declaration_id });
                continue;
            }

            // reuse known value types when already available
            if let Some(value_ty_id) = self.export_known_value_type_id(types, value_symbol) {
                types.set_value_type(export_symbol, value_ty_id);
                continue;
            }

            // resolve the declarator that owns this binding
            let Some(declarator_id) =
                self.direct_binding_declarator_for_symbol(module, value_symbol, tree, symbols)
            else {
                continue;
            };

            // evaluate declared types when present
            if let Some(declared_type_id) = self.export_declared_value_type_id(
                module,
                profile,
                declarator_id,
                tree,
                symbols,
                types,
            )? {
                types.set_value_type(export_symbol, declared_type_id);
                continue;
            }

            // defer to surface inference for initializer-only exports
            let declarator = tree.get(declarator_id);
            let binding_mutability = symbols.get_symbol(value_symbol.local_id).binding_mutability;
            let is_const_asserted = self.declarator_is_const_assertion(declarator_id, tree);
            export_inference.push(ExportInference {
                export_symbol,
                declarator_id,
                value_id: declarator.value,
                binding_mutability,
                is_const_asserted,
            });
        }

        // seed inference variables for export symbols
        for export in &export_inference {
            // create an infer var for the exported value
            let scope = InferScope {
                owner: export.export_symbol,
                function_id: None,
            };
            let origin = InferOrigin::Expression(export.declarator_id.into_global_any(module.id));
            let symbol_ty_id = self.infer_var_type_for_symbol(
                &mut infer,
                types,
                export.export_symbol,
                export.declarator_id.into_any(),
                origin,
                scope,
            );

            // register the type id for this export
            types.set_value_type(export.export_symbol, symbol_ty_id);
            inferred_exports.push((
                symbol_ty_id,
                export.declarator_id.into_global_any(module.id),
            ));
        }

        // infer unannotated exported function declarations
        for export in &export_declarations {
            // infer the declaration with an unconstrained expectation
            let mut ctx = base_ctx.fork().with_expected_type(None);
            self.infer_declaration(
                module,
                export.declaration_id,
                tree,
                symbols,
                types,
                &mut infer,
                &mut ctx,
            )?;
        }

        // infer initializer types and constrain export symbols
        for export in export_inference {
            // skip exports without initializers or symbols
            let Some(value_id) = export.value_id else {
                continue;
            };
            let Some(symbol_ty_id) = types.get_value_type_id(export.export_symbol) else {
                continue;
            };

            // reject export inference cycles that lack explicit annotations
            if self
                .export_inference_requires_annotation(module, profile, tree, symbols, value_id)?
            {
                self.error(AnalyzeError::ExportInferenceRequiresAnnotation {
                    node: value_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                });
                let error_ty_id = types.insert_type_from_any(Type::Error, value_id.into_any());
                types.set_value_type(export.export_symbol, error_ty_id);
                continue;
            }

            // infer the initializer with binding defaults and export expectations
            let mut ctx = base_ctx.fork().with_expected_type(Some(symbol_ty_id));
            if let Some(mutability) = export.binding_mutability {
                ctx = ctx.with_binding_mutability(mutability);
            } else {
                ctx = ctx
                    .with_const_context(ConstContext::None)
                    .with_widening()
                    .with_fresh_literals();
            }
            let inferred_ty_id = self
                .infer_expression(module, value_id, tree, symbols, types, &mut infer, &mut ctx)?;
            let committed_ty_id = self.commit_binding_type(
                module,
                &ctx,
                inferred_ty_id,
                types,
                export.is_const_asserted,
            );
            infer.push_constraint(Constraint::Subtype {
                sub_type: committed_ty_id,
                super_type: symbol_ty_id,
                variance: None,
            });
        }

        // solve surface inference constraints before warning
        if !infer.vars.is_empty() {
            self.solve_infer_table(module, profile, symbols, &infer, types, &base_ctx.options);
        }

        // warn when exports remain unknown after surface inference
        for (ty_id, node_id) in inferred_exports {
            if matches!(
                types.get_type(ty_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown
                }
            ) {
                self.warning(AnalyzeWarning::ExportTypeUnknown {
                    node: node_id.into_anchored(Some(profile)),
                });
            }
        }

        Ok(())
    }

    /// Resolve the export symbol and its local target for value inference.
    fn export_inference_value_symbol(
        &self,
        symbols: &SymbolTable,
        module_id: ModuleId,
        export: &Export,
    ) -> Option<(GlobalSymbolId, GlobalSymbolId)> {
        // skip exports that cannot produce local values
        if export.space != SymbolSpace::Value {
            return None;
        }

        // require resolved local exports
        let export_symbol = export.target.resolved()?;
        if export_symbol.module_id != module_id {
            return None;
        }

        // follow local aliases to the concrete symbol
        let value_symbol = self.local_export_target_symbol(symbols, module_id, export_symbol);
        if value_symbol.module_id != module_id {
            return None;
        }

        Some((export_symbol, value_symbol))
    }

    /// Return an exported function declaration that needs return inference.
    fn export_inference_function_declaration(
        &self,
        tree: &NodeTree,
        primary_declaration: GlobalNodeIdAny,
    ) -> Option<LocalNodeId<Declaration>> {
        // resolve the primary declaration node
        let declaration_id = self.primary_declaration_id(tree, primary_declaration)?;

        // require an unannotated function declaration with a body
        let Declaration::Function {
            signature, body, ..
        } = tree.get(declaration_id)
        else {
            return None;
        };
        if signature.return_type.is_some() || body.is_none() {
            return None;
        }

        Some(declaration_id)
    }

    /// Return a known value type id for an exported symbol.
    fn export_known_value_type_id(
        &self,
        types: &TypeTable,
        value_symbol: GlobalSymbolId,
    ) -> Option<LocalTypeId> {
        // reuse known value types when already available
        let value_ty_id = types.get_value_type_id(value_symbol)?;
        let value_ty = types.get_type(value_ty_id);
        let is_unknown = matches!(
            value_ty,
            Type::TypeLiteral {
                value: TypeLiteral::Unknown
            } | Type::InferVar { .. }
        );
        if is_unknown {
            return None;
        }

        Some(value_ty_id)
    }

    /// Resolve and evaluate the declared value type for an export.
    fn export_declared_value_type_id(
        &self,
        module: &Module,
        profile: ProfileId,
        declarator_id: LocalNodeId<Declarator>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // read the declared type when present
        let declared_type_id = types.get_declared_type_id(declarator_id.into_global_any(module.id));
        let Some(declared_type_id) = declared_type_id else {
            return Ok(None);
        };

        // evaluate and return the declared type
        self.resolve_declared_type(module, profile, declared_type_id, tree, symbols, types)?;
        Ok(Some(declared_type_id))
    }

    /// Return true when an export initializer needs an explicit annotation.
    fn export_inference_requires_annotation(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        value_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<bool> {
        // collect remote references used by the initializer
        let references = self.export_inference_references(module, tree, symbols, value_id);

        // check for cycles without declared annotations
        for referenced in references {
            let has_cycle =
                self.export_inference_has_cycle(module.id, profile, referenced.module_id)?;
            if has_cycle && !self.remote_symbol_has_declared_value_type(profile, referenced)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Collect remote references used by an export inference initializer.
    fn export_inference_references(
        &self,
        module: &Module,
        tree: &NodeTree,
        symbols: &SymbolTable,
        value_id: LocalNodeId<Expression>,
    ) -> Vec<GlobalSymbolId> {
        // walk the initializer and collect remote symbols
        let mut collector = ExportInferenceReferenceCollector::new(module, symbols);
        collector.visit_expression(tree, value_id, tree.get(value_id));
        collector.references
    }

    /// Follow local export aliases to reach the concrete symbol.
    fn local_export_target_symbol(
        &self,
        symbols: &SymbolTable,
        module_id: ModuleId,
        symbol: GlobalSymbolId,
    ) -> GlobalSymbolId {
        let mut current = symbol;
        let mut visited = Vec::new();

        loop {
            if current.module_id != module_id {
                return current;
            }
            if visited.contains(&current) {
                return current;
            }
            visited.push(current);

            let entry = symbols.get_symbol(current.local_id);
            let Some(next) = entry.target_symbol else {
                return current;
            };
            current = next;
        }
    }

    /// Resolve a declaration id from a primary declaration node.
    fn primary_declaration_id(
        &self,
        tree: &NodeTree,
        primary_declaration: GlobalNodeIdAny,
    ) -> Option<LocalNodeId<Declaration>> {
        match primary_declaration.local_id.ty {
            NodeType::Declaration => Some(primary_declaration.local_id.into_typed()),
            NodeType::Expression => {
                let expression_id = primary_declaration.local_id.into_typed::<Expression>();
                match tree.get(expression_id) {
                    Expression::Declaration { declaration } => Some(*declaration),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
