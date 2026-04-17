use std::sync::Arc;

use crate::analyze::common::{AnalyzeIndex, InferContext, NormalizationMode, TypeContext};
use crate::analyze::r#type::json_value_to_type;
use crate::timing::tags;
use crate::{
    AnalyzeError, AnalyzeResult, Compiler, CompilerContext, FlowContext, InferRepository,
    RequirementCollector,
};
use destack_artifact::Data;
use destack_dir::{
    Declaration, Declarator, Expression, FlowGraphBuilder, InferTable, IntType, LocalNodeId,
    LocalNodeIdAny, LocalSymbolId, NodeTree, PrimitiveType, SymbolTable, Type, TypeLiteral,
    TypeTable,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ModuleSource, ProfileId};
use std::collections::HashSet;

impl Compiler {
    /// Phase 3: Infer expression types.
    pub(crate) fn analyze_module_infer(
        &self,
        tree: &NodeTree,
        symbols: &SymbolTable,
        roots: &[LocalNodeId<Expression>],
        types: &mut TypeTable,
        default_symbol: LocalSymbolId,
        anchor_node: LocalNodeIdAny,
        module_id: ModuleId,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> AnalyzeResult<Option<InferTable>> {
        let _timing = self.timing_scope(tags::ANALYZE_MODULE_INFER);
        // analyze data modules specially
        if !context.is_code_module(module_id) {
            self.analyze_data_module_infer(context, types, default_symbol, anchor_node, module_id)?;
            return Ok(None);
        }

        // skip inference when module language is disabled
        if !self.module_language_allowed_in_context(context, module_id) {
            return Ok(None);
        }

        let module = context.module(module_id);

        if module.language_type.is_declaration() {
            let module_checks = context.module_check_options_for_module(module.id);
            let should_skip_enum_validation = module_checks.skip_lib_check
                || (matches!(module.source, ModuleSource::Builtin(_))
                    && !self.options.validate_builtin_libs);
            if should_skip_enum_validation {
                return Ok(None);
            }

            let options = context.analyze_context_options_for_module(module.id);
            let mut ctx = TypeContext::new(
                context,
                module.as_ref(),
                profile,
                &options,
                tree,
                symbols,
                types,
                AnalyzeIndex::default(),
            );

            // infer enum backing types when declaration validation is enabled
            for root_id in roots.iter() {
                let Expression::Declaration(declaration) = ctx.tree.get(*root_id) else {
                    continue;
                };
                let Declaration::Enum(declaration) = ctx.tree.get(*declaration) else {
                    continue;
                };
                let enum_symbol = declaration.symbol.into_global(ctx.module.id);
                let backing_type = self.infer_enum_field_values(
                    &mut ctx.reborrow(),
                    enum_symbol,
                    &declaration.fields,
                )?;
                ctx.types.set_enum_backing_type(enum_symbol, backing_type);
            }

            return Ok(None);
        }

        // select runtime roots for the inference pass
        let runtime_roots = self.collect_runtime_roots(module.as_ref(), tree, roots);
        let infer_roots = runtime_roots.clone();

        // skip infer entirely when no roots require infer work
        if infer_roots.is_empty() {
            return Ok(None);
        }

        // resolve builtins before resolving type-import operator dependencies
        self.require_language_environment(context.revision(), profile)
            .map_err(AnalyzeError::from)?;
        self.require_type_import_dependencies_for_infer(
            context.revision(),
            module.as_ref(),
            profile,
            tree,
        )?;

        // establish infer dependency preconditions
        self.require_dir_interface(context.revision(), module_id, profile)?;
        self.require_declare_dependencies_for_infer(context.revision(), module_id, profile)?;
        self.require_interface_dependencies(context.revision(), module_id, profile)?;

        let mut collector = RequirementCollector::new();
        let options = context.analyze_context_options_for_module(module.id);
        // initialize infer session state
        let mut session = InferRepository::new(profile, options);
        let (infer_table, infer_state) = session.parts_mut();
        let mut ctx = InferContext::new(
            context,
            module.as_ref(),
            profile,
            &options,
            tree,
            symbols,
            types,
            infer_table,
            AnalyzeIndex::default(),
        );

        // build a module level flow graph and flow table when needed
        let flow_roots = {
            let _timing = self.timing_scope(tags::ANALYZE_FLOW_REQUIREMENTS);
            runtime_roots
                .iter()
                .copied()
                .filter(|root_id| self.expression_requires_flow(ctx.tree, *root_id))
                .collect::<Vec<_>>()
        };
        if !flow_roots.is_empty() {
            let graph = {
                let _timing = self.timing_scope(tags::ANALYZE_FLOW_GRAPH_BUILD);
                FlowGraphBuilder::new(module.id, ctx.tree).build_roots(&flow_roots)
            };
            let flow = {
                let _timing = self.timing_scope(tags::ANALYZE_FLOW_TABLE_COMPUTE);
                self.compute_flow_table_for_graph(
                    &mut ctx.type_context_reborrow(),
                    &graph,
                    infer_state,
                )?
            };
            infer_state.flow = Some(FlowContext {
                module_id: module.id,
                graph: Arc::new(graph),
                table: Arc::new(flow),
            });
        }

        // report expression form diagnostics once before type inference
        {
            let _timing = self.timing_scope(tags::ANALYZE_EXPRESSION_INFER);
            for (expression_id, expression) in ctx.tree.iter_nodes_of_type::<Expression>() {
                if !self.is_node_active(ctx.tree, ctx.symbols, expression_id.into_any()) {
                    continue;
                }

                self.report_pre_infer_expression_form_diagnostics(
                    &mut ctx.type_context_reborrow(),
                    expression_id,
                    expression,
                );
            }
        }

        // infer each root expression
        {
            let _timing = self.timing_scope(tags::ANALYZE_EXPRESSION_INFER);
            // resolve declarator annotation types before runtime root inference
            self.prepare_declarator_annotation_types_for_infer(&mut ctx.reborrow())?;

            for root_id in infer_roots.iter() {
                self.collect(
                    &mut collector,
                    self.infer_expression(&mut ctx.reborrow(), *root_id, infer_state),
                );
            }
        }

        // yield on any yields
        if let Some(requirement) = collector.try_into_requirement() {
            return Err(AnalyzeError::Yield { requirement });
        }

        // publish infer-table state for later solve and commit work
        Ok(Some(session.into_table()))
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
        ctx: &mut InferContext<'_>,
    ) -> AnalyzeResult<()> {
        for (declarator_id, declarator) in ctx.tree.iter_nodes_of_type::<Declarator>() {
            if !self.is_node_active(ctx.tree, ctx.symbols, declarator_id.into_any()) {
                continue;
            }
            let Some(annotation_id) = declarator.ty else {
                continue;
            };
            let declarator_node_id = declarator_id.into_global_any(ctx.module.id);

            // register one declared type id for all annotated declarators
            let annotation_ty_id = if let Some(annotation_ty_id) =
                ctx.types.get_declared_type_id(declarator_node_id)
            {
                annotation_ty_id
            } else {
                let annotation_ty_id = self.resolve_declared_type_expression(
                    &mut ctx.type_context_reborrow(),
                    annotation_id,
                    true,
                    true,
                )?;
                ctx.types
                    .set_declared_type(declarator_node_id, annotation_ty_id);
                annotation_ty_id
            };

            self.resolve_declared_type(&mut ctx.type_context_reborrow(), annotation_ty_id)?;

            if declarator.value.is_some() {
                continue;
            }

            if self.type_contains_static_parameters(
                ctx.type_view(),
                annotation_ty_id,
                &mut HashSet::new(),
            ) {
                continue;
            }

            let _ = self.normalize_type(
                &mut ctx.type_context_reborrow(),
                annotation_ty_id,
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
        context: &CompilerContext<'_>,
        types: &mut TypeTable,
        default_symbol: LocalSymbolId,
        source_id: LocalNodeIdAny,
        module_id: ModuleId,
    ) -> AnalyzeResult<()> {
        let module = context.module(module_id);
        let module = module.as_ref();
        if module.loader.is_data() {
            let data = match context.data(module_id) {
                Some(data) => data,
                None => return Ok(()),
            };
            let value = match data.as_ref() {
                Data::Json(value) => value,
                Data::Html(_) | Data::Css(_) => return Ok(()),
            };

            let inferred_type =
                json_value_to_type(value, source_id, types, &self.repository.strings);

            // associate the inferred type with the default symbol
            types.set_value_type(default_symbol.into_global(module_id), inferred_type);
        }
        // otherwise text and file modules are always string
        else if module.loader.is_text() || module.loader.is_file() {
            let string_type = types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::String),
                },
                source_id,
            );
            types.set_value_type(default_symbol.into_global(module_id), string_type);
        }
        // otherwise binary modules are uint8[]
        else if module.loader.is_binary() {
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
        // otherwise code modules are handled elsewhere
        else {
            debug_assert!(module.is_code());
        }

        Ok(())
    }
}
