use std::sync::Arc;

use destack_dir as dir;
use destack_formatter::format_file_tree;
use destack_repository::{FormatterOptions, Module};
use destack_source::ModuleId;

use crate::check::reify::r#type::TypeReifier;
use crate::check::{CheckModuleState, CheckState, Decision};
use crate::{CompilerError, CompilerResult};

/// One rendered module source with solved annotations filled in.
pub(in crate::check) struct AnnotatedSource {
    /// The rendered module.
    pub(in crate::check) module: Arc<Module>,
    /// The formatted source text.
    pub(in crate::check) content: String,
}

impl CheckState<'_> {
    /// Render every member module's source with solved annotations.
    /// Each render clones the parsed tree, reifies solved declaration
    /// boundary types into annotation nodes, and prints the amended
    /// tree through the canonical formatter.
    pub(in crate::check) fn render_annotated_sources(
        &mut self,
    ) -> CompilerResult<Vec<AnnotatedSource>> {
        let mut sources = Vec::with_capacity(self.modules.len());
        for module_id in self.modules.keys().copied().collect::<Vec<_>>() {
            if let Some(source) = self.render_annotated_source(module_id)? {
                sources.push(source);
            }
        }

        Ok(sources)
    }

    /// Render one member module's source with solved annotations.
    fn render_annotated_source(
        &mut self,
        module_id: ModuleId,
    ) -> CompilerResult<Option<AnnotatedSource>> {
        let coercions = self.module_coercions(module_id);
        let state = self.module(module_id);
        let file_id = state.module.file_id;
        let Some(roots) = state.parsed.roots_for_file(file_id) else {
            return Ok(None);
        };

        // amend a clone of the parsed tree with reified annotations
        let tree = SourceReifier::new(self, state).run(&coercions)?;

        // print the amended tree through the canonical formatter
        let file = self.compiler.file(self.context, file_id)?;
        let tokens = state
            .parsed
            .iter_token_spans_for_file(file_id)
            .map(|tokens| tokens.collect::<Vec<_>>())
            .unwrap_or_default();
        let side_tokens = state
            .parsed
            .iter_side_token_spans_for_file(file_id)
            .map(|tokens| tokens.collect::<Vec<_>>())
            .unwrap_or_default();
        let content = format_file_tree(
            file.as_ref(),
            &tree,
            &tokens,
            &side_tokens,
            roots,
            state.strings.as_ref(),
            FormatterOptions::default(),
        )
        .map_err(|error| CompilerError::Internal {
            message: format!("annotated render failed for {}: {error}", state.module.uri),
        })?;

        Ok(Some(AnnotatedSource {
            module: Arc::clone(&state.module),
            content,
        }))
    }
}

/// Source reification pass for one checked module.
struct SourceReifier<'a, 'b> {
    /// The checked component state.
    check: &'a CheckState<'b>,
    /// The module being rendered.
    state: &'a CheckModuleState,
    /// The type-expression reifier writing synthesized nodes.
    types: TypeReifier<'a, 'b>,
}

impl<'a, 'b> SourceReifier<'a, 'b> {
    /// Create a source reifier for one module.
    fn new(check: &'a CheckState<'b>, state: &'a CheckModuleState) -> Self {
        let tree = state.source_tree().clone();
        let types = TypeReifier::new(check, tree, state.strings.as_ref());

        Self {
            check,
            state,
            types,
        }
    }

    /// Run source reification and return the amended tree.
    fn run(
        mut self,
        coercions: &[(dir::GlobalNodeIdAny, dir::Coercion)],
    ) -> CompilerResult<dir::Tree> {
        self.reify_type_expressions()?;
        self.reify_declarations()?;
        self.reify_declarators()?;
        self.reify_parameters()?;
        self.reify_members()?;
        self.reify_expressions()?;
        self.reify_coercions(coercions)?;

        Ok(self.types.tree)
    }

    /// Reify authored type-expression sites that carry solved check facts.
    ///
    /// Example:
    /// ```ds
    /// const values: [_; 3] = [1, 2, 3];
    /// // renders as `const values: [float64; 3] = [1, 2, 3];`
    /// ```
    fn reify_type_expressions(&mut self) -> CompilerResult<()> {
        let module_id = self.state.module.id;
        let view = dir::View::new(self.state.source_tree());
        for (hole_id, expression) in view.iter_nodes_of_type::<dir::TypeExpression>() {
            if !matches!(
                expression,
                dir::TypeExpression::Infer {
                    form: dir::InferForm::Hole,
                    ..
                }
            ) {
                continue;
            }

            // the hole's node type carries its solved variable
            let Some(ty) = self
                .check
                .node_type_maybe(hole_id.into_global_any(module_id))
            else {
                continue;
            };
            self.anchor(hole_id.into_any());
            let Some(filled) = self.types.reify(ty)? else {
                continue;
            };

            // replace the hole in place with the reified spelling
            *self.types.tree.get_mut(hole_id) = self.types.tree.get(filled).clone();
        }

        Ok(())
    }

    /// Reify declaration-level annotations and induced generic parameters.
    fn reify_declarations(&mut self) -> CompilerResult<()> {
        let module_id = self.state.module.id;
        let view = dir::View::new(self.state.source_tree());
        for (declaration_id, declaration) in view.iter_nodes_of_type::<dir::Declaration>() {
            self.reify_declaration_generic_parameters(module_id, declaration_id)?;
            self.reify_declaration_return(declaration_id, declaration)?;
        }

        Ok(())
    }

    /// Reify induced generic parameters for one declaration.
    fn reify_declaration_generic_parameters(
        &mut self,
        module_id: ModuleId,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
    ) -> CompilerResult<()> {
        let source = declaration_id.into_global_any(module_id);
        let Some(template) = self.check.generics.template_by_source(source) else {
            return Ok(());
        };
        let Some(template) = self.check.generic_template(template) else {
            return Ok(());
        };

        let mut parameters = Vec::new();
        for parameter in &template.parameters {
            let parameter = dir::GlobalGenericParameterId::new(module_id, *parameter);
            let Some(binding) = self.check.generic_parameter(parameter) else {
                continue;
            };
            if !binding.is_induced_header_parameter() {
                continue;
            }

            self.anchor(declaration_id.into_any());
            let Some(parameter) = self.types.reify_generic_parameter(binding)? else {
                continue;
            };
            parameters.push(parameter);
        }
        if parameters.is_empty() {
            return Ok(());
        }

        let Some(generic_parameters) = self
            .types
            .tree
            .get_mut(declaration_id)
            .generic_parameters_mut()
        else {
            return Ok(());
        };
        generic_parameters.extend(parameters);

        Ok(())
    }

    /// Reify one function declaration return annotation.
    fn reify_declaration_return(
        &mut self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) -> CompilerResult<()> {
        let dir::Declaration::Function(function) = declaration else {
            return Ok(());
        };
        if !TypeReifier::returns_fillable(&function.signature) || function.body.is_none() {
            return Ok(());
        }

        let Some(annotation) = self.reify_return(declaration_id.into_any())? else {
            return Ok(());
        };
        let dir::Declaration::Function(function) = self.types.tree.get_mut(declaration_id) else {
            unreachable!("function declarations keep their kind");
        };
        function.signature.return_type = Some(annotation);

        Ok(())
    }

    /// Reify binding declarator annotations.
    fn reify_declarators(&mut self) -> CompilerResult<()> {
        let view = dir::View::new(self.state.source_tree());
        for (declarator_id, declarator) in view.iter_nodes_of_type::<dir::Declarator>() {
            if !matches!(view.get(declarator.pattern), dir::Pattern::Binding { .. }) {
                continue;
            }

            let Some(ty) = self.declaration_site_type(declarator.pattern.into_any()) else {
                continue;
            };
            self.anchor(declarator.pattern.into_any());
            let Some(annotation) = self.types.reify(ty)? else {
                continue;
            };

            self.types.tree.get_mut(declarator_id).ty = Some(annotation);
        }

        Ok(())
    }

    /// Reify named parameter annotations.
    fn reify_parameters(&mut self) -> CompilerResult<()> {
        let view = dir::View::new(self.state.source_tree());
        for (parameter_id, parameter) in view.iter_nodes_of_type::<dir::Parameter>() {
            if parameter.is_comptime() {
                continue;
            }
            if !matches!(
                parameter,
                dir::Parameter::Named { .. } | dir::Parameter::VariadicNamed { .. }
            ) {
                continue;
            }

            let Some(ty) = self.declaration_site_type(parameter_id.into_any()) else {
                continue;
            };
            self.anchor(parameter_id.into_any());
            let Some(annotation) = self
                .types
                .reify_parameter_type(ty, parameter.is_optional())?
            else {
                continue;
            };

            match self.types.tree.get_mut(parameter_id) {
                dir::Parameter::Named { declared_type, .. }
                | dir::Parameter::VariadicNamed { declared_type, .. } => {
                    *declared_type = Some(annotation);
                }
                _ => unreachable!("filled parameter forms are filtered above"),
            }
        }

        Ok(())
    }

    /// Reify member annotations.
    fn reify_members(&mut self) -> CompilerResult<()> {
        let view = dir::View::new(self.state.source_tree());
        for (member_id, member) in view.iter_nodes_of_type::<dir::Member>() {
            match member {
                // reify field types
                dir::Member::Field { .. } => {
                    let Some(ty) = self.declaration_site_type(member_id.into_any()) else {
                        continue;
                    };
                    self.anchor(member_id.into_any());
                    let Some(annotation) = self.types.reify(ty)? else {
                        continue;
                    };
                    let dir::Member::Field { declared_type, .. } =
                        self.types.tree.get_mut(member_id)
                    else {
                        unreachable!("member fields keep their kind");
                    };

                    *declared_type = Some(annotation);
                }
                // reify method returns
                dir::Member::Method {
                    signature, body, ..
                } if TypeReifier::returns_fillable(signature) && body.is_some() => {
                    let Some(annotation) = self.reify_return(member_id.into_any())? else {
                        continue;
                    };
                    let dir::Member::Method { signature, .. } = self.types.tree.get_mut(member_id)
                    else {
                        unreachable!("member methods keep their kind");
                    };

                    signature.return_type = Some(annotation);
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Reify expression-level selected arguments.
    fn reify_expressions(&mut self) -> CompilerResult<()> {
        let module_id = self.state.module.id;
        let view = dir::View::new(self.state.source_tree());
        for (expression_id, expression) in view.iter_nodes_of_type::<dir::Expression>() {
            match expression {
                dir::Expression::Call { .. } => {
                    self.reify_call_arguments(module_id, expression_id, expression)?;
                }
                dir::Expression::New { .. } | dir::Expression::NewMaybe { .. } => {
                    self.reify_construct_arguments(module_id, expression_id, expression)?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Reify one call expression's selected generic arguments.
    fn reify_call_arguments(
        &mut self,
        module_id: ModuleId,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) -> CompilerResult<()> {
        let dir::Expression::Call {
            left,
            generic_arguments,
            ..
        } = expression
        else {
            return Ok(());
        };
        if !generic_arguments.is_empty() {
            return Ok(());
        }
        if matches!(
            self.types.tree.get(*left),
            dir::Expression::Instantiation { .. }
        ) {
            return Ok(());
        }
        let Some(decision) = self
            .check
            .decisions
            .get(expression_id.into_global_any(module_id))
        else {
            return Ok(());
        };
        let Some(arguments) = decision.generic_arguments() else {
            return Ok(());
        };
        if arguments.is_empty() {
            return Ok(());
        }

        self.anchor(expression_id.into_any());
        let Some(generic_arguments) = self.types.reify_generic_argument_bindings(arguments)? else {
            return Ok(());
        };
        let instantiation = self.types.insert(dir::Expression::Instantiation {
            left: *left,
            generic_arguments,
        });
        let dir::Expression::Call { left, .. } = self.types.tree.get_mut(expression_id) else {
            unreachable!("call expressions keep their kind");
        };
        *left = instantiation;

        Ok(())
    }

    /// Reify one construct expression's selected generic arguments.
    fn reify_construct_arguments(
        &mut self,
        module_id: ModuleId,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) -> CompilerResult<()> {
        let ty = match expression {
            dir::Expression::New { ty, .. } | dir::Expression::NewMaybe { ty, .. } => *ty,
            _ => return Ok(()),
        };
        let node = expression_id.into_global_any(module_id);
        let Some(Decision::Construct(resolution)) = self.check.decisions.get(node) else {
            return Ok(());
        };
        let arguments = resolution.target.generic_arguments();
        if arguments.is_empty() {
            return Ok(());
        }

        self.anchor(ty.into_any());
        self.types.fill_type_arguments(ty, arguments)?;

        Ok(())
    }

    /// Reify the solved return type of one function declaration site.
    fn reify_return(
        &mut self,
        site: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::TypeExpression>>> {
        let Some(ty) = self.declaration_site_type(site) else {
            return Ok(None);
        };

        // look through function values to their signature
        let mut signature = self.check.resolve_shallow(ty)?;
        loop {
            match self.check.ty(signature)? {
                dir::Type::Function(function) => {
                    signature = self.check.resolve_shallow(function.signature)?;
                }
                dir::Type::FunctionSignature(_) => break,
                _ => return Ok(None),
            }
        }
        let dir::Type::FunctionSignature(function) = self.check.ty(signature)?.clone() else {
            unreachable!("the signature loop stops on function signatures");
        };

        self.anchor(site);
        match function.return_type {
            Some(return_type) => self.types.reify(return_type),
            // an omitted semantic return spells void
            None => Ok(Some(self.types.insert_keyword(dir::TypeLiteral::Void))),
        }
    }

    /// Return the solved type bound at one declaration site.
    fn declaration_site_type(&self, site: dir::LocalNodeIdAny) -> Option<dir::GlobalTypeId> {
        let module_id = self.state.module.id;
        let symbol = self
            .state
            .bindings
            .declaration_symbol(site.into_global(module_id))?;

        self.check
            .component_symbol_type_maybe(symbol.into_global(module_id))
    }

    /// Anchor synthesized nodes at one source site.
    fn anchor(&mut self, site: dir::LocalNodeIdAny) {
        self.types.anchor(self.state.authored_span(site));
    }

    /// Reify recorded implicit coercions as explicit cast expressions.
    fn reify_coercions(
        &mut self,
        coercions: &[(dir::GlobalNodeIdAny, dir::Coercion)],
    ) -> CompilerResult<()> {
        for (node, coercion) in coercions.iter().copied() {
            // only authored expression nodes spell casts
            if node.local_id.ty != dir::NodeType::Expression
                || !self.state.is_authored(node.local_id)
            {
                continue;
            }

            self.anchor(node.local_id);
            let Some(target_type) = self.types.reify(coercion.target)? else {
                continue;
            };

            // wrap the coerced value in an explicit cast
            let id = dir::LocalNodeId::<dir::Expression>::new(node.local_id.id);
            let original = self.types.tree.get(id).clone();
            let span = self.state.authored_span(node.local_id);
            let expression = self.types.tree.insert(original, span);
            *self.types.tree.get_mut(id) = dir::Expression::As {
                expression,
                target_type,
            };
        }

        Ok(())
    }
}
