use std::collections::HashMap;
use std::sync::Arc;

use crate::analyze::common::{NormalizationMode, TypeRewriteCache};
use crate::analyze::StaticMemberSymbolKind;
use crate::timing::tags;
use crate::{
    AnalyzeError, AnalyzeResult, Compiler, FlowContext, InferSession, TaskDependencyError,
    TaskResultCollector,
};
use destack_dir::{
    Declaration, Declarator, Expression, FlowGraphBuilder, InferTable, IntType, LocalNodeId,
    NodeTree, Pattern, PrimitiveType, StaticArgument, StaticExpression, SymbolTable, Type,
    TypeLiteral, TypeTable,
};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{Module, ModuleContent, ModuleSource, ModuleType, ProfileId};
use std::collections::HashSet;

use super::super::common::json_value_to_type;

impl Compiler {
    /// Ensure a module's types have been inferred.
    pub fn require_analyze_module_infer(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        use crate::AnalyzeTask;
        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleInfer { module, profile })
    }

    /// Phase 2: Infer expression types.
    pub(crate) fn analyze_module_infer(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> AnalyzeResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<AnalyzeError>(
            module_id,
            module_version,
            profile,
            profile_version,
        )?;
        let _timing = self.timing_scope(tags::ANALYZE_MODULE_INFER);

        // analyze data modules specially
        if !self.is_code_module(module_id) {
            return self.analyze_data_module_infer(module_id, profile);
        }

        // skip inference when module language is disabled
        if !self.module_language_allowed(module_id) {
            return Ok(());
        }

        self.require_analyze_module_declare(module_id, profile)?;

        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();

        if module.language_type.is_declaration() {
            let module_checks = self.module_check_options_for_module(module.id);
            let should_skip_enum_validation = module_checks.skip_lib_check
                || (matches!(module.source, ModuleSource::Builtin(_))
                    && !self.options.validate_builtin_libs);
            if should_skip_enum_validation {
                return Ok(());
            }

            // infer enum backing types when declaration validation is enabled
            for root_id in dir.roots.iter() {
                let Expression::Declaration { declaration } = tree.get(*root_id) else {
                    continue;
                };
                let Declaration::Enum {
                    descriptor, fields, ..
                } = tree.get(*declaration)
                else {
                    continue;
                };
                let enum_symbol = descriptor.symbol.into_global(module.id);
                let backing_type = self.infer_enum_field_values(
                    &module,
                    profile,
                    enum_symbol,
                    fields,
                    &tree,
                    &symbols,
                    &mut types,
                )?;
                types.set_enum_backing_type(enum_symbol, backing_type);
            }

            return Ok(());
        }

        // select runtime roots for the inference pass
        let runtime_roots = self.collect_runtime_roots(&module, &tree, &dir.roots);
        let infer_roots = runtime_roots.clone();

        // require builtins before resolving type-import operator dependencies
        self.require_resolve_builtins(profile)?;
        self.require_type_import_interface_dependency_closure_for_infer(&module, profile, &tree)?;

        drop(types);
        drop(symbols);
        drop(tree);

        // establish infer dependency preconditions
        self.require_analyze_module_interface(module_id, profile)?;
        self.require_declare_dependencies_for_infer(module_id, profile)?;
        self.require_interface_dependencies_for_infer(module_id, profile)?;
        self.require_declare_inference_for_builtin_modules(module_id, profile)?;
        self.require_interface_inference_for_ambient_libs(profile)?;

        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let mut collector = TaskResultCollector::new();
        let options = self.analyze_context_options_for_module(module.id);

        // initialize infer session state
        let mut session = InferSession::new(profile, options);

        // resolve declarator annotation types before runtime root inference
        self.prepare_declarator_annotation_types_for_infer(
            &module, profile, &tree, &symbols, &mut types,
        )?;

        // build a module level flow graph and flow table when needed
        let flow_roots = {
            let _timing = self.timing_scope(tags::ANALYZE_FLOW_REQUIREMENTS);
            runtime_roots
                .iter()
                .copied()
                .filter(|root_id| self.expression_requires_flow(&tree, *root_id))
                .collect::<Vec<_>>()
        };
        if !flow_roots.is_empty() {
            let graph = {
                let _timing = self.timing_scope(tags::ANALYZE_FLOW_GRAPH_BUILD);
                FlowGraphBuilder::new(module.id, &tree).build_roots(&flow_roots)
            };
            let flow = {
                let _timing = self.timing_scope(tags::ANALYZE_FLOW_TABLE_COMPUTE);
                let (infer_table, context) = session.parts_mut();
                self.compute_flow_table_for_graph(
                    &module,
                    &graph,
                    &tree,
                    &symbols,
                    &mut types,
                    infer_table,
                    context,
                )?
            };
            session.context_mut().flow = Some(FlowContext {
                module_id: module.id,
                graph: Arc::new(graph),
                table: Arc::new(flow),
            });
        }

        // report expression form diagnostics once before type inference
        {
            let _timing = self.timing_scope(tags::ANALYZE_EXPRESSION_INFER);
            for (expression_id, expression) in tree.iter_nodes_of_type::<Expression>() {
                if !self.is_node_active(&tree, &symbols, expression_id.into_any()) {
                    continue;
                }

                self.report_pre_infer_expression_form_diagnostics(
                    &module,
                    profile,
                    &tree,
                    &symbols,
                    &types,
                    expression_id,
                    expression,
                    options,
                );
            }
        }

        // infer each root expression
        {
            let _timing = self.timing_scope(tags::ANALYZE_EXPRESSION_INFER);
            for root_id in infer_roots.iter() {
                let (infer_table, context) = session.parts_mut();
                self.collect(
                    &mut collector,
                    self.infer_expression(
                        &module,
                        *root_id,
                        &tree,
                        &symbols,
                        &mut types,
                        infer_table,
                        context,
                    ),
                );
            }
        }

        // yield on any yields
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(AnalyzeError::Yield { dependency });
        }

        // solve constraints (and commit inferred types)
        if !session.table().vars.is_empty() || !session.table().constraints.is_empty() {
            let _timing = self.timing_scope(tags::ANALYZE_INFER_SOLVE_CONSTRAINTS);
            self.solve_infer_table(
                &module,
                profile,
                &symbols,
                session.table(),
                &mut types,
                &session.context().options,
            );
        }

        // report associated comptime projection obligations after inference convergence
        self.report_associated_comptime_projection_obligation_errors(
            &module,
            profile,
            &mut types,
            session.table_mut(),
        )?;

        // discharge instance-commit obligations after inference convergence
        self.discharge_instance_commit_obligations(session.table_mut(), &mut types)?;

        // commit instance-instantiated inferred types after inference convergence
        self.commit_instance_instantiated_inferred_types(
            &module,
            profile,
            &tree,
            &symbols,
            session.table(),
            &mut types,
        )?;
        self.refresh_direct_binding_value_types_from_inferred_initializers(
            &module, profile, &tree, &symbols, &mut types,
        );

        // report deferred type relation diagnostics after final inference commitments
        let options = session.context().options;
        self.report_type_relation_obligation_errors_after_infer_convergence(
            &module,
            profile,
            &symbols,
            &mut types,
            session.table_mut(),
            &options,
        )?;

        Ok(())
    }

    /// Commit inferred types that depend on resolved instance substitutions.
    fn commit_instance_instantiated_inferred_types(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        infer: &InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // apply substitutions to inferred types using explicit infer obligations
        let node_attachments = infer
            .iter_instance_commit_obligation_nodes()
            .collect::<Vec<_>>();
        for (node_id, obligation_id) in node_attachments {
            if node_id.module_id != module.id {
                continue;
            }
            let Some(inferred_type_id) = types.get_inferred_type_id(node_id) else {
                continue;
            };
            let Some(obligation) = infer.instance_commit_obligation(obligation_id) else {
                return Err(AnalyzeError::Internal {
                    message: "missing instance commit obligation for inferred node".to_string(),
                });
            };

            // build type substitutions from the committed obligation environment
            let mut substitutions = HashMap::new();
            for (parameter_symbol, argument) in obligation
                .static_parameter_symbols
                .iter()
                .zip(obligation.static_arguments.iter())
            {
                let StaticArgument::Evaluated {
                    value: StaticExpression::Type { ty },
                    ..
                } = argument
                else {
                    continue;
                };
                substitutions.insert(*parameter_symbol, types.unwrap_value_type_id(*ty));
            }
            if substitutions.is_empty() {
                continue;
            }

            let mut materialize_cache = TypeRewriteCache::new();
            let mut substitution_cache = HashMap::new();
            let mapped_type_id = self.instantiate_type_with_substitutions(
                module,
                profile,
                node_id.local_id,
                None,
                inferred_type_id,
                &substitutions,
                tree,
                symbols,
                types,
                &mut materialize_cache,
                &mut substitution_cache,
            );

            if mapped_type_id != inferred_type_id {
                types.set_inferred_type(node_id, mapped_type_id);
            }
        }

        Ok(())
    }

    /// Refresh direct binding value types from converged initializer inference.
    fn refresh_direct_binding_value_types_from_inferred_initializers(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) {
        // update direct unannotated declarators to the final initializer inferred type
        for declarator_id in tree.iter_node_ids_of_type::<Declarator>() {
            let declarator_node_id = declarator_id.into_global_any(module.id);
            if types.get_declared_type_id(declarator_node_id).is_some() {
                continue;
            }

            let declarator = tree.get(declarator_id);
            let Some(value_id) = declarator.value else {
                continue;
            };
            let Some(inferred_type_id) =
                types.get_inferred_type_id(value_id.into_global_any(module.id))
            else {
                continue;
            };

            let Pattern::Binding {
                symbol,
                pattern: None,
                ..
            } = tree.get(declarator.pattern)
            else {
                continue;
            };
            let binding_symbol = symbol.into_global(module.id);
            let Some(current_value_type_id) = types.get_value_type_id(binding_symbol) else {
                continue;
            };
            if current_value_type_id == inferred_type_id {
                continue;
            }

            let current_requires_refresh = match types.get_type(current_value_type_id) {
                Type::Unevaluated(_) => true,
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                } => true,
                Type::Reference { symbol, .. } => {
                    matches!(
                        self.query_static_member_symbol_kind_for_symbol(
                            module, profile, *symbol, tree, symbols
                        ),
                        Ok(Some(StaticMemberSymbolKind::AssociatedType))
                    )
                }
                _ => false,
            };
            if !current_requires_refresh {
                continue;
            }

            types.set_value_type(binding_symbol, inferred_type_id);
        }
    }

    /// Collect root expressions that can produce runtime behavior.
    fn collect_runtime_roots(
        &self,
        module: &Module,
        tree: &NodeTree,
        roots: &[LocalNodeId<Expression>],
    ) -> Vec<LocalNodeId<Expression>> {
        let mut runtime_roots = Vec::new();
        for root_id in roots {
            if self.expression_requires_infer(module, *root_id, tree) {
                runtime_roots.push(*root_id);
            }
        }

        runtime_roots
    }

    /// Resolve and normalize concrete declarator annotations before expression inference.
    fn prepare_declarator_annotation_types_for_infer(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        for (declarator_id, declarator) in tree.iter_nodes_of_type::<Declarator>() {
            if !self.is_node_active(tree, symbols, declarator_id.into_any()) {
                continue;
            }
            if declarator.ty.is_none() {
                continue;
            }
            if declarator.value.is_some() {
                continue;
            }

            let Some(annotation_ty_id) =
                types.get_declared_type_id(declarator_id.into_global(module.id).into())
            else {
                continue;
            };

            self.resolve_declared_type(module, profile, annotation_ty_id, tree, symbols, types)?;

            if self.type_contains_static_parameters(
                module,
                profile,
                annotation_ty_id,
                symbols,
                types,
                &mut HashSet::new(),
            ) {
                continue;
            }

            let _ = self.normalize_type(
                module,
                profile,
                annotation_ty_id,
                symbols,
                types,
                NormalizationMode::Assign,
            );
        }

        Ok(())
    }

    /// Infer types for data modules (JSON, TOML, YAML), text modules, and binary modules.
    ///
    /// This function converts the parsed data into a structural DIR type and associates
    /// it with the module's default export symbol.
    fn analyze_data_module_infer(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        let module_ref = self.program.modules.get(module_id);
        let module = module_ref.read();
        let dir = module.dir(profile);
        let default_symbol = dir.default_symbol;
        let source_id = dir.anchor_node;
        let module_type = module.module_type;

        match module_type {
            ModuleType::Data => {
                let value = match &module.content {
                    ModuleContent::Data { value, .. } => value.clone(),
                    _ => return Ok(()),
                };
                drop(module);

                // re-acquire the module and infer type
                let module_ref = self.program.modules.get(module_id);
                let module = module_ref.read();
                let dir = module.dir(profile);
                let mut types = dir.types.write();

                let inferred_type =
                    json_value_to_type(&value, source_id, &mut types, &self.program.strings);

                // associate the inferred type with the default symbol
                types.set_value_type(default_symbol.into_global(module_id), inferred_type);
            }
            ModuleType::Text => {
                // text modules are always string
                let mut types = dir.types.write();
                let string_type = types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::String),
                    },
                    source_id,
                );
                types.set_value_type(default_symbol.into_global(module_id), string_type);
            }
            ModuleType::Binary => {
                // binary modules are uint8[]
                let mut types = dir.types.write();
                let element = types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Uint8)),
                    },
                    source_id,
                );
                let array_type = types.insert_type_from_any(
                    Type::Array {
                        element: Some(element),
                        is_readonly: false,
                    },
                    source_id,
                );
                types.set_value_type(default_symbol.into_global(module_id), array_type);
            }
            ModuleType::Code => {
                unreachable!("code modules are handled by analyze_module_infer");
            }
        }

        Ok(())
    }
}
