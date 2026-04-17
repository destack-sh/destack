use crate::analyze::common::{InferContext, TypeContext};
use crate::{AnalyzeResult, AnalyzeWarning, Compiler, InferState};
use destack_dir::{
    Constraint, Declaration, Declarator, Export, Expression, GlobalNodeIdAny, GlobalSymbolId,
    InferOrigin, InferScope, InferTable, LocalNodeId, LocalTypeId, NodeTree, NodeType, StaticKey,
    SymbolSpace, SymbolTable, Type, TypeLiteral, TypeTable,
};
use destack_source::ModuleId;
use indexmap::IndexMap;

impl Compiler {
    /// Infer interface value types using local information only.
    pub(crate) fn infer_interface_value_types(
        &self,
        ctx: &mut TypeContext<'_>,
        exported_symbols: &IndexMap<(SymbolSpace, StaticKey), Export>,
        emit_unknown_warnings: bool,
    ) -> AnalyzeResult<()> {
        // skip interface value inference for declaration modules
        if ctx.module.language_type.is_declaration() {
            for export in exported_symbols.values() {
                let Some((export_symbol, value_symbol)) =
                    self.interface_value_symbol_for_export(ctx.symbols, ctx.module.id, export)
                else {
                    continue;
                };

                let Some(declarator_id) =
                    self.direct_binding_declarator_for_symbol(ctx.tree_symbol_view(), value_symbol)
                else {
                    if let Some(value_type_id) =
                        self.concrete_interface_value_type_id(&mut ctx.reborrow(), value_symbol)?
                    {
                        self.publish_interface_value_type(
                            export_symbol,
                            value_symbol,
                            value_type_id,
                            ctx.types,
                        );
                    }
                    continue;
                };

                let declared_type_id =
                    self.declared_interface_value_type_id(&mut ctx.reborrow(), declarator_id)?;
                if let Some(declared_type_id) = declared_type_id {
                    self.publish_interface_value_type(
                        export_symbol,
                        value_symbol,
                        declared_type_id,
                        ctx.types,
                    );
                    continue;
                }

                if let Some(value_type_id) =
                    self.concrete_interface_value_type_id(&mut ctx.reborrow(), value_symbol)?
                {
                    self.publish_interface_value_type(
                        export_symbol,
                        value_symbol,
                        value_type_id,
                        ctx.types,
                    );
                }
            }

            // ensure interface snapshots never publish value exports without one value type
            self.ensure_interface_value_types_for_exports(&mut ctx.reborrow(), exported_symbols)?;

            return Ok(());
        }

        // prepare surface inference for interface values
        let base_ctx = InferState::new(ctx.profile, *ctx.options).for_surface_inference();
        let mut infer = InferTable::default();
        let mut inferred_exports = Vec::new();
        let mut pending_initializers = Vec::new();
        let mut export_declarations = Vec::new();

        // collect exported symbols that need value types
        for export in exported_symbols.values() {
            // resolve the local export symbol for interface value inference
            let Some((export_symbol, value_symbol)) =
                self.interface_value_symbol_for_export(ctx.symbols, ctx.module.id, export)
            else {
                continue;
            };

            // read the primary declaration for the export symbol
            let symbol_entry = ctx.symbols.get_symbol(value_symbol.local_id);
            let Some(primary_declaration) = symbol_entry.primary_declaration else {
                continue;
            };

            // defer unannotated function returns to declaration inference
            if let Some(declaration_id) =
                self.interface_function_declaration(ctx.tree, primary_declaration)
            {
                export_declarations.push(declaration_id);
                continue;
            }

            // resolve the declarator that owns this binding
            let Some(declarator_id) =
                self.direct_binding_declarator_for_symbol(ctx.tree_symbol_view(), value_symbol)
            else {
                let value_type_id =
                    self.concrete_interface_value_type_id(&mut ctx.reborrow(), value_symbol)?;
                if let Some(value_type_id) = value_type_id {
                    self.publish_interface_value_type(
                        export_symbol,
                        value_symbol,
                        value_type_id,
                        ctx.types,
                    );
                    continue;
                }

                continue;
            };

            // evaluate declared types when present
            let declared_type_id =
                self.declared_interface_value_type_id(&mut ctx.reborrow(), declarator_id)?;
            if let Some(declared_type_id) = declared_type_id {
                self.publish_interface_value_type(
                    export_symbol,
                    value_symbol,
                    declared_type_id,
                    ctx.types,
                );
                continue;
            }

            // use one already concrete value type when available
            let value_type_id =
                self.concrete_interface_value_type_id(&mut ctx.reborrow(), value_symbol)?;
            if let Some(value_type_id) = value_type_id {
                self.publish_interface_value_type(
                    export_symbol,
                    value_symbol,
                    value_type_id,
                    ctx.types,
                );
                continue;
            }

            // seed one surface infer var for initializer-only exports
            let declarator = ctx.tree.get(declarator_id);
            let binding_mutability = ctx
                .symbols
                .get_symbol(value_symbol.local_id)
                .binding_mutability;
            let scope = InferScope {
                owner: export_symbol,
                function_id: None,
            };
            let origin = InferOrigin::Expression(declarator_id.into_global_any(ctx.module.id));
            let symbol_ty_id = self.infer_var_type_for_symbol(
                &mut infer,
                ctx.types,
                export_symbol,
                declarator_id.into_any(),
                origin,
                scope,
            );

            // publish the infer var as the current export surface
            self.publish_interface_value_type(export_symbol, value_symbol, symbol_ty_id, ctx.types);
            inferred_exports.push((symbol_ty_id, declarator_id.into_global_any(ctx.module.id)));
            pending_initializers.push((
                symbol_ty_id,
                declarator_id,
                declarator.value,
                binding_mutability,
            ));
        }

        // infer interface declarations and initializers with one shared ctx context
        let mut ctx = InferContext {
            compiler_context: ctx.compiler_context,
            module: ctx.module,
            profile: ctx.profile,
            options: &base_ctx.options,
            tree: ctx.tree,
            symbols: ctx.symbols,
            index: ctx.index.clone(),
            types: ctx.types,
            infer: &mut infer,
        };

        // infer unannotated exported function declarations
        for declaration_id in &export_declarations {
            // infer the declaration with an unconstrained expectation
            let mut state = base_ctx.fork().with_expected_type(None);
            self.infer_declaration(&mut ctx.reborrow(), *declaration_id, &mut state)?;
        }

        // infer initializer types and constrain export symbols
        for (symbol_ty_id, declarator_id, value_id, binding_mutability) in pending_initializers {
            // skip exports without initializers or symbols
            let Some(value_id) = value_id else {
                continue;
            };

            // infer the initializer with binding defaults and export expectations
            let mut state = base_ctx.fork().with_expected_type(Some(symbol_ty_id));
            state = self.binding_initializer_context(&state, binding_mutability);
            let inferred_ty_id =
                self.infer_expression(&mut ctx.reborrow(), value_id, &mut state)?;
            let committed_ty_id = self.materialize_declarator_initializer_type(
                &mut ctx.type_context_reborrow(),
                declarator_id,
                inferred_ty_id,
                &state,
            );
            ctx.infer.push_constraint(Constraint::Subtype {
                sub_type: committed_ty_id,
                super_type: symbol_ty_id,
                variance: None,
            });
        }

        // solve interface-local constraints for this surface pass
        {
            let (mut ctx, infer) = ctx.split_type_context_and_infer();
            self.solve_interface_value_constraints(&mut ctx, infer);
        }

        // warn when interface exports remain unknown after surface inference
        if emit_unknown_warnings {
            for (ty_id, node_id) in inferred_exports {
                if matches!(
                    ctx.types.get_type(ty_id),
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown
                    }
                ) {
                    self.warning(AnalyzeWarning::ExportTypeUnknown {
                        node: node_id.into_anchored(Some(ctx.profile)),
                    });
                }
            }
        }

        // ensure interface snapshots never publish value exports without one value type
        self.ensure_interface_value_types_for_exports(
            &mut ctx.type_context_reborrow(),
            exported_symbols,
        )?;

        Ok(())
    }

    /// Ensure each published value export has one committed interface value type.
    fn ensure_interface_value_types_for_exports(
        &self,
        ctx: &mut TypeContext<'_>,
        exports: &IndexMap<(SymbolSpace, StaticKey), Export>,
    ) -> AnalyzeResult<()> {
        for export in exports.values() {
            let Some((export_symbol, value_symbol)) =
                self.interface_value_symbol_for_export(ctx.symbols, ctx.module.id, export)
            else {
                continue;
            };

            if let Some(export_type_id) = ctx.types.get_value_type_id(export_symbol) {
                self.publish_interface_value_type(
                    export_symbol,
                    value_symbol,
                    export_type_id,
                    ctx.types,
                );
                continue;
            }

            if let Some(value_type_id) = ctx.types.get_value_type_id(value_symbol) {
                self.publish_interface_value_type(
                    export_symbol,
                    value_symbol,
                    value_type_id,
                    ctx.types,
                );
                continue;
            }

            let unknown_type_id =
                self.commit_unknown_interface_value_type(&mut ctx.reborrow(), export, value_symbol);
            self.publish_interface_value_type(
                export_symbol,
                value_symbol,
                unknown_type_id,
                ctx.types,
            );
        }

        Ok(())
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
        ctx: &mut TypeContext<'_>,
        export: &Export,
        value_symbol: GlobalSymbolId,
    ) -> LocalTypeId {
        let source_node = ctx
            .symbols
            .get_symbol(value_symbol.local_id)
            .primary_declaration
            .filter(|declaration| declaration.module_id == ctx.module.id)
            .map(|declaration| declaration.local_id)
            .or_else(|| export.item.map(|item| item.into_any()))
            .unwrap_or(ctx.anchor_node());

        ctx.types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
            source_node,
        )
    }

    /// Solve interface value inference constraints for the current surface pass.
    fn solve_interface_value_constraints(&self, ctx: &mut TypeContext<'_>, infer: &InferTable) {
        if infer.vars.is_empty() {
            return;
        }

        // interface inference solves an ephemeral local table, not the module solve stage table
        self.solve_infer_table(ctx, infer);
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
        let Declaration::Function(declaration) = tree.get(declaration_id) else {
            return None;
        };
        if declaration.signature.return_type.is_some() || declaration.body.is_none() {
            return None;
        }

        Some(declaration_id)
    }

    /// Return one already concrete value type id for an exported symbol.
    fn concrete_interface_value_type_id(
        &self,
        ctx: &mut TypeContext<'_>,
        value_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // read one already-computed value type
        let value_ty_id = ctx.types.get_value_type_id(value_symbol);
        let Some(value_ty_id) = value_ty_id else {
            return Ok(None);
        };

        let value_ty_id =
            self.materialize_interface_value_type(&mut ctx.reborrow(), value_ty_id)?;
        let value_ty = ctx.types.get_type(value_ty_id);
        let is_unknown = matches!(
            value_ty,
            Type::TypeLiteral {
                value: TypeLiteral::Unknown
            } | Type::InferVar { .. }
        );
        if is_unknown {
            return Ok(None);
        }

        Ok(Some(value_ty_id))
    }

    /// Materialize reachable unevaluated type ids before publishing one interface value type.
    fn materialize_interface_value_type(
        &self,
        ctx: &mut TypeContext<'_>,
        value_type_id: LocalTypeId,
    ) -> AnalyzeResult<LocalTypeId> {
        loop {
            let Some(unevaluated_id) = self
                .collect_unevaluated_type_ids(value_type_id, ctx.types)
                .into_iter()
                .next()
            else {
                return Ok(value_type_id);
            };

            self.resolve_declared_type(&mut ctx.reborrow(), unevaluated_id)?;

            if matches!(ctx.types.get_type(unevaluated_id), Type::Unevaluated(_)) {
                return Ok(value_type_id);
            }
        }
    }

    /// Resolve and evaluate the declared value type for an export.
    fn declared_interface_value_type_id(
        &self,
        ctx: &mut TypeContext<'_>,
        declarator_id: LocalNodeId<Declarator>,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let declarator = ctx.tree.get(declarator_id);
        let Some(annotation_id) = declarator.ty else {
            return Ok(None);
        };

        let declared_type_id =
            self.resolve_declared_type_expression(&mut ctx.reborrow(), annotation_id, true, true)?;
        ctx.types.set_declared_type(
            declarator_id.into_global_any(ctx.module.id),
            declared_type_id,
        );

        Ok(Some(declared_type_id))
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
                    Expression::Declaration(declaration) => Some(*declaration),
                    _ => None,
                }
            }
            _ => None,
        }
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
