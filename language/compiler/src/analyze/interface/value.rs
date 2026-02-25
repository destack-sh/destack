use crate::analyze::common::{InferTablesContext, TypeTablesContext};
use crate::{AnalyzeResult, AnalyzeWarning, Compiler, InferContext};
use destack_dir::{
    Constraint, Declaration, Declarator, Export, Expression, GlobalNodeIdAny, GlobalSymbolId,
    InferOrigin, InferScope, InferTable, LocalNodeId, LocalTypeId, Mutability, NodeTree, NodeType,
    NodeVisitor, NodeVisitorOptions, StaticKey, SymbolSpace, SymbolTable, Type, TypeLiteral,
    TypeTable, walk_expression,
};
use destack_source::ModuleId;
use destack_workspace::Module;
use indexmap::IndexMap;

/// Track an exported declarator that needs surface inference.
#[derive(Debug)]
struct InterfaceValueInference {
    /// The exported symbol to assign a value type.
    export_symbol: GlobalSymbolId,
    /// The local binding symbol for the export.
    value_symbol: GlobalSymbolId,
    /// The declarator that owns the binding.
    declarator_id: LocalNodeId<Declarator>,
    /// The initializer expression when present.
    value_id: Option<LocalNodeId<Expression>>,
    /// The binding mutability when available.
    binding_mutability: Option<Mutability>,
}

/// Track a function declaration that needs return inference.
#[derive(Debug)]
struct InterfaceDeclarationInference {
    /// The declaration id to infer.
    declaration_id: LocalNodeId<Declaration>,
}

/// Collect remote references in interface value initializers.
#[derive(Debug)]
struct InterfaceValueReferenceCollector<'a> {
    /// The module being analyzed.
    module: &'a Module,
    /// The symbol table for the module.
    symbols: &'a SymbolTable,
    /// Remote symbols referenced by the interface initializer.
    references: Vec<GlobalSymbolId>,
    /// Node visitor options.
    options: NodeVisitorOptions,
}

impl<'a> InterfaceValueReferenceCollector<'a> {
    /// Create a new interface value reference collector.
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

impl NodeVisitor for InterfaceValueReferenceCollector<'_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // collect remote references for interface value inference
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

impl Compiler {
    /// Infer interface value types using local information only.
    pub(crate) fn infer_interface_value_types(
        &self,
        type_tables: &mut TypeTablesContext<'_>,
        exported_symbols: &IndexMap<(SymbolSpace, StaticKey), Export>,
        emit_unknown_warnings: bool,
    ) -> AnalyzeResult<()> {
        // skip interface value inference for declaration modules
        if type_tables.module.language_type.is_declaration() {
            for export in exported_symbols.values() {
                let Some((export_symbol, value_symbol)) = self.interface_value_symbol_for_export(
                    type_tables.symbols,
                    type_tables.module.id,
                    export,
                ) else {
                    continue;
                };

                if let Some(value_ty_id) =
                    self.known_interface_value_type_id(type_tables.types, value_symbol)
                {
                    self.publish_interface_value_type(
                        export_symbol,
                        value_symbol,
                        value_ty_id,
                        type_tables.types,
                    );
                    continue;
                }

                let Some(declarator_id) = self.direct_binding_declarator_for_symbol(
                    type_tables.module,
                    value_symbol,
                    type_tables.tree,
                    type_tables.symbols,
                ) else {
                    continue;
                };

                if let Some(declared_type_id) = self
                    .declared_interface_value_type_id(&mut type_tables.reborrow(), declarator_id)?
                {
                    self.publish_interface_value_type(
                        export_symbol,
                        value_symbol,
                        declared_type_id,
                        type_tables.types,
                    );
                }
            }

            // ensure interface snapshots never publish value exports without one value type
            self.ensure_interface_value_types_for_exports(
                &mut type_tables.reborrow(),
                exported_symbols,
            );

            return Ok(());
        }

        // prepare surface inference for interface values
        let base_ctx =
            InferContext::new(type_tables.profile, *type_tables.options).for_surface_inference();
        let mut infer = InferTable::default();
        let mut inferred_exports = Vec::new();
        let mut export_inference = Vec::new();
        let mut export_declarations = Vec::new();

        // collect exported symbols that need value types
        for export in exported_symbols.values() {
            // resolve the local export symbol for interface value inference
            let Some((export_symbol, value_symbol)) = self.interface_value_symbol_for_export(
                type_tables.symbols,
                type_tables.module.id,
                export,
            ) else {
                continue;
            };

            // read the primary declaration for the export symbol
            let symbol_entry = type_tables.symbols.get_symbol(value_symbol.local_id);
            let Some(primary_declaration) = symbol_entry.primary_declaration else {
                continue;
            };

            // defer unannotated function returns to declaration inference
            if let Some(declaration_id) =
                self.interface_function_declaration(type_tables.tree, primary_declaration)
            {
                export_declarations.push(InterfaceDeclarationInference { declaration_id });
                continue;
            }

            // reuse known value types when already available
            if let Some(value_ty_id) =
                self.known_interface_value_type_id(type_tables.types, value_symbol)
            {
                self.publish_interface_value_type(
                    export_symbol,
                    value_symbol,
                    value_ty_id,
                    type_tables.types,
                );
                continue;
            }

            // resolve the declarator that owns this binding
            let Some(declarator_id) = self.direct_binding_declarator_for_symbol(
                type_tables.module,
                value_symbol,
                type_tables.tree,
                type_tables.symbols,
            ) else {
                continue;
            };

            // evaluate declared types when present
            if let Some(declared_type_id) =
                self.declared_interface_value_type_id(&mut type_tables.reborrow(), declarator_id)?
            {
                self.publish_interface_value_type(
                    export_symbol,
                    value_symbol,
                    declared_type_id,
                    type_tables.types,
                );
                continue;
            }

            // defer to surface inference for initializer-only exports
            let declarator = type_tables.tree.get(declarator_id);
            let binding_mutability = type_tables
                .symbols
                .get_symbol(value_symbol.local_id)
                .binding_mutability;
            export_inference.push(InterfaceValueInference {
                export_symbol,
                value_symbol,
                declarator_id,
                value_id: declarator.value,
                binding_mutability,
            });
        }

        // seed inference variables for export symbols
        for export in &export_inference {
            // create an infer var for the exported value
            let scope = InferScope {
                owner: export.export_symbol,
                function_id: None,
            };
            let origin = InferOrigin::Expression(
                export.declarator_id.into_global_any(type_tables.module.id),
            );
            let symbol_ty_id = self.infer_var_type_for_symbol(
                &mut infer,
                type_tables.types,
                export.export_symbol,
                export.declarator_id.into_any(),
                origin,
                scope,
            );

            // register the type id for this export
            self.publish_interface_value_type(
                export.export_symbol,
                export.value_symbol,
                symbol_ty_id,
                type_tables.types,
            );
            inferred_exports.push((
                symbol_ty_id,
                export.declarator_id.into_global_any(type_tables.module.id),
            ));
        }

        // infer interface declarations and initializers with one shared tables context
        let mut tables = InferTablesContext::new(
            type_tables.module,
            type_tables.profile,
            &base_ctx.options,
            type_tables.tree,
            type_tables.symbols,
            type_tables.types,
            &mut infer,
        );

        // infer unannotated exported function declarations
        for export in &export_declarations {
            // infer the declaration with an unconstrained expectation
            let mut ctx = base_ctx.fork().with_expected_type(None);
            self.infer_declaration(&mut tables.reborrow(), export.declaration_id, &mut ctx)?;
        }

        // infer initializer types and constrain export symbols
        for export in &export_inference {
            // skip exports without initializers or symbols
            let Some(value_id) = export.value_id else {
                continue;
            };
            let Some(symbol_ty_id) = tables.types.get_value_type_id(export.export_symbol) else {
                continue;
            };

            // infer the initializer with binding defaults and export expectations
            let mut ctx = base_ctx.fork().with_expected_type(Some(symbol_ty_id));
            ctx = self.binding_initializer_context(&ctx, export.binding_mutability);
            let inferred_ty_id =
                self.infer_expression(&mut tables.reborrow(), value_id, &mut ctx)?;
            let committed_ty_id = self.materialize_declarator_initializer_type(
                tables.module,
                export.declarator_id,
                value_id,
                inferred_ty_id,
                tables.tree,
                &ctx,
                tables.types,
            );
            tables.infer.push_constraint(Constraint::Subtype {
                sub_type: committed_ty_id,
                super_type: symbol_ty_id,
                variance: None,
            });
        }

        // solve interface-local constraints for this surface pass
        {
            let (mut type_tables, infer) = tables.split_type_tables_and_infer();
            self.solve_interface_value_constraints(&mut type_tables, infer);
        }

        // warn when interface exports remain unknown after surface inference
        if emit_unknown_warnings {
            for (ty_id, node_id) in inferred_exports {
                if matches!(
                    tables.types.get_type(ty_id),
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown
                    }
                ) {
                    self.warning(AnalyzeWarning::ExportTypeUnknown {
                        node: node_id.into_anchored(Some(tables.profile)),
                    });
                }
            }
        }

        // ensure interface snapshots never publish value exports without one value type
        self.ensure_interface_value_types_for_exports(
            &mut tables.type_tables_reborrow(),
            exported_symbols,
        );

        Ok(())
    }

    /// Ensure each published value export has one committed interface value type.
    fn ensure_interface_value_types_for_exports(
        &self,
        tables: &mut TypeTablesContext<'_>,
        exports: &IndexMap<(SymbolSpace, StaticKey), Export>,
    ) {
        for export in exports.values() {
            let Some((export_symbol, value_symbol)) =
                self.interface_value_symbol_for_export(tables.symbols, tables.module.id, export)
            else {
                continue;
            };

            if let Some(export_type_id) = tables.types.get_value_type_id(export_symbol) {
                self.publish_interface_value_type(
                    export_symbol,
                    value_symbol,
                    export_type_id,
                    tables.types,
                );
                continue;
            }

            if let Some(value_type_id) = tables.types.get_value_type_id(value_symbol) {
                self.publish_interface_value_type(
                    export_symbol,
                    value_symbol,
                    value_type_id,
                    tables.types,
                );
                continue;
            }

            let unknown_type_id = self.commit_unknown_interface_value_type(
                &mut tables.reborrow(),
                export,
                value_symbol,
            );
            self.publish_interface_value_type(
                export_symbol,
                value_symbol,
                unknown_type_id,
                tables.types,
            );
        }
    }

    /// Publish one interface value type for both export and local value symbols.
    fn publish_interface_value_type(
        &self,
        export_symbol: GlobalSymbolId,
        value_symbol: GlobalSymbolId,
        value_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) {
        types.set_value_type(export_symbol, value_type_id);
        if value_symbol != export_symbol {
            types.set_value_type(value_symbol, value_type_id);
        }
    }

    /// Commit semantic unknown for one published export symbol with no inferred value type.
    fn commit_unknown_interface_value_type(
        &self,
        tables: &mut TypeTablesContext<'_>,
        export: &Export,
        value_symbol: GlobalSymbolId,
    ) -> LocalTypeId {
        let source_node = tables
            .symbols
            .get_symbol(value_symbol.local_id)
            .primary_declaration
            .filter(|declaration| declaration.module_id == tables.module.id)
            .map(|declaration| declaration.local_id)
            .or_else(|| export.item.map(|item| item.into_any()))
            .unwrap_or(tables.module.dir(tables.profile).anchor_node);

        tables.types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
            source_node,
        )
    }

    /// Solve interface value inference constraints for the current surface pass.
    fn solve_interface_value_constraints(
        &self,
        type_tables: &mut TypeTablesContext<'_>,
        infer: &InferTable,
    ) {
        if infer.vars.is_empty() {
            return;
        }

        // interface inference solves an ephemeral local table, not the module solve stage table
        self.solve_infer_table(type_tables, infer);
    }

    /// Resolve the export symbol and its local target for interface value inference.
    pub(crate) fn interface_value_symbol_for_export(
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
        let export_symbol =
            self.canonical_interface_local_symbol(symbols, module_id, export_symbol);

        // follow local aliases to the concrete symbol
        let value_symbol = self.local_export_target_symbol(symbols, module_id, export_symbol);
        if value_symbol.module_id != module_id {
            return None;
        }

        Some((export_symbol, value_symbol))
    }

    /// Return an exported function declaration that needs interface return inference.
    fn interface_function_declaration(
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
    fn known_interface_value_type_id(
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
    fn declared_interface_value_type_id(
        &self,
        type_tables: &mut TypeTablesContext<'_>,
        declarator_id: LocalNodeId<Declarator>,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // read the declared type when present
        let declared_type_id = type_tables
            .types
            .get_declared_type_id(declarator_id.into_global_any(type_tables.module.id));
        let Some(declared_type_id) = declared_type_id else {
            return Ok(None);
        };

        // evaluate and return the declared type
        self.resolve_declared_type(type_tables, declared_type_id)?;
        Ok(Some(declared_type_id))
    }

    /// Collect remote references used by an interface value initializer.
    pub(crate) fn interface_value_references(
        &self,
        module: &Module,
        tree: &NodeTree,
        symbols: &SymbolTable,
        value_id: LocalNodeId<Expression>,
    ) -> Vec<GlobalSymbolId> {
        // walk the initializer and collect remote symbols
        let mut collector = InterfaceValueReferenceCollector::new(module, symbols);
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
        let mut current = self.canonical_interface_local_symbol(symbols, module_id, symbol);
        let mut visited = Vec::new();

        loop {
            if current.module_id != module_id {
                return current;
            }
            current = self.canonical_interface_local_symbol(symbols, module_id, current);
            if visited.contains(&current) {
                return current;
            }
            visited.push(current);

            let entry = symbols.get_symbol(current.local_id);
            let Some(next) = entry.target_symbol else {
                return current;
            };
            current = self.canonical_interface_local_symbol(symbols, module_id, next);
        }
    }

    /// Canonicalize one local symbol id using the current symbol table entry type.
    fn canonical_interface_local_symbol(
        &self,
        symbols: &SymbolTable,
        module_id: ModuleId,
        symbol: GlobalSymbolId,
    ) -> GlobalSymbolId {
        if symbol.module_id != module_id {
            return symbol;
        }

        let symbol_entry = symbols.get_symbol(symbol.local_id);
        GlobalSymbolId::new(module_id, symbol.local_id.with_type(symbol_entry.ty))
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

    /// Return true when one interface value type still requires solver convergence.
    pub(crate) fn interface_value_requires_solver(
        &self,
        types: &TypeTable,
        ty_id: LocalTypeId,
    ) -> bool {
        types.get_type(ty_id).is_infer()
    }

    /// Return true when one interface value type is the semantic `unknown` top type.
    pub(crate) fn interface_value_is_semantic_unknown(
        &self,
        types: &TypeTable,
        ty_id: LocalTypeId,
    ) -> bool {
        matches!(
            types.get_type(ty_id),
            Type::TypeLiteral {
                value: TypeLiteral::Unknown
            }
        )
    }
}
