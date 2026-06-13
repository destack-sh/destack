use std::sync::Arc;

use destack_dir as dir;
use destack_formatter::format_file_tree;
use destack_repository::{FormatterOptions, Module};
use destack_source::{ModuleId, Span};

use crate::check::reify::r#type::Reifier;
use crate::check::{CheckModuleState, CheckState};
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
    /// Each render clones the parsed tree, reifies the solved type of
    /// every unannotated site into synthesized annotation nodes, and
    /// prints the amended tree through the canonical formatter.
    pub(in crate::check) fn render_annotated_sources(
        &self,
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
        &self,
        module_id: ModuleId,
    ) -> CompilerResult<Option<AnnotatedSource>> {
        let state = self.module(module_id);
        let file_id = state.module.file_id;
        let Some(roots) = state.parsed.roots_for_file(file_id) else {
            return Ok(None);
        };

        // amend a clone of the parsed tree with reified annotations
        let mut reifier = Reifier::new(self, state.parsed_tree().clone(), state.strings.as_ref());
        self.fill_holes(state, &mut reifier)?;
        self.fill_declarators(state, &mut reifier)?;
        self.fill_parameters(state, &mut reifier)?;
        self.fill_returns(state, &mut reifier)?;
        self.fill_members(state, &mut reifier)?;
        self.fill_coercions(state, &mut reifier)?;

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
            &reifier.tree,
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

    /// Fill inference holes inside written annotations with their
    /// solved types.
    ///
    /// Example:
    /// ```ds
    /// const values: [_; 3] = [1, 2, 3];
    /// // renders as `const values: [float64; 3] = [1, 2, 3];`
    /// ```
    fn fill_holes(
        &self,
        state: &CheckModuleState,
        reifier: &mut Reifier<'_, '_>,
    ) -> CompilerResult<()> {
        let module_id = state.module.id;
        let view = dir::View::new(state.parsed_tree());
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
            let Some(ty) = self.inputs.node_type(hole_id.into_global_any(module_id)) else {
                continue;
            };
            reifier.anchor(self.site_anchor(state, hole_id.into_any()));
            let Some(filled) = reifier.reify(ty)? else {
                continue;
            };

            // replace the hole in place with the reified spelling
            *reifier.tree.get_mut(hole_id) = reifier.tree.get(filled).clone();
        }

        Ok(())
    }

    /// Fill unannotated binding declarators with their solved types.
    fn fill_declarators(
        &self,
        state: &CheckModuleState,
        reifier: &mut Reifier<'_, '_>,
    ) -> CompilerResult<()> {
        let view = dir::View::new(state.parsed_tree());
        for (declarator_id, declarator) in view.iter_nodes_of_type::<dir::Declarator>() {
            if declarator.ty.is_some() {
                continue;
            }
            if !matches!(view.get(declarator.pattern), dir::Pattern::Binding { .. }) {
                continue;
            }

            let Some(ty) = self.declaration_site_type(state, declarator.pattern.into_any()) else {
                continue;
            };
            reifier.anchor(self.site_anchor(state, declarator.pattern.into_any()));
            let Some(annotation) = reifier.reify(ty)? else {
                continue;
            };

            reifier.tree.get_mut(declarator_id).ty = Some(annotation);
        }

        Ok(())
    }

    /// Fill unannotated named parameters with their solved types.
    fn fill_parameters(
        &self,
        state: &CheckModuleState,
        reifier: &mut Reifier<'_, '_>,
    ) -> CompilerResult<()> {
        let view = dir::View::new(state.parsed_tree());
        for (parameter_id, parameter) in view.iter_nodes_of_type::<dir::Parameter>() {
            if parameter.declared_type().is_some() || parameter.is_comptime() {
                continue;
            }
            if !matches!(
                parameter,
                dir::Parameter::Named { .. } | dir::Parameter::VariadicNamed { .. }
            ) {
                continue;
            }

            let Some(ty) = self.declaration_site_type(state, parameter_id.into_any()) else {
                continue;
            };
            reifier.anchor(self.site_anchor(state, parameter_id.into_any()));
            let Some(annotation) = reifier.reify(ty)? else {
                continue;
            };

            match reifier.tree.get_mut(parameter_id) {
                dir::Parameter::Named { declared_type, .. }
                | dir::Parameter::VariadicNamed { declared_type, .. } => {
                    *declared_type = Some(annotation);
                }
                _ => unreachable!("filled parameter forms are filtered above"),
            }
        }

        Ok(())
    }

    /// Fill unannotated function declaration returns with solved types.
    fn fill_returns(
        &self,
        state: &CheckModuleState,
        reifier: &mut Reifier<'_, '_>,
    ) -> CompilerResult<()> {
        let view = dir::View::new(state.parsed_tree());
        for (declaration_id, declaration) in view.iter_nodes_of_type::<dir::Declaration>() {
            let dir::Declaration::Function(function) = declaration else {
                continue;
            };
            if !Reifier::returns_fillable(&function.signature) || function.body.is_none() {
                continue;
            }

            let Some(annotation) = self.reify_return(state, reifier, declaration_id.into_any())?
            else {
                continue;
            };
            let dir::Declaration::Function(function) = reifier.tree.get_mut(declaration_id) else {
                unreachable!("function declarations keep their kind");
            };

            function.signature.return_type = Some(annotation);
        }

        Ok(())
    }

    /// Fill unannotated member sites with their solved types.
    fn fill_members(
        &self,
        state: &CheckModuleState,
        reifier: &mut Reifier<'_, '_>,
    ) -> CompilerResult<()> {
        let view = dir::View::new(state.parsed_tree());
        for (member_id, member) in view.iter_nodes_of_type::<dir::Member>() {
            match member {
                // fill unannotated field types
                dir::Member::Field {
                    declared_type: None,
                    ..
                } => {
                    let Some(ty) = self.declaration_site_type(state, member_id.into_any()) else {
                        continue;
                    };
                    reifier.anchor(self.site_anchor(state, member_id.into_any()));
                    let Some(annotation) = reifier.reify(ty)? else {
                        continue;
                    };
                    let dir::Member::Field { declared_type, .. } = reifier.tree.get_mut(member_id)
                    else {
                        unreachable!("member fields keep their kind");
                    };

                    *declared_type = Some(annotation);
                }
                // fill unannotated method returns
                dir::Member::Method {
                    signature, body, ..
                } if Reifier::returns_fillable(signature) && body.is_some() => {
                    let Some(annotation) =
                        self.reify_return(state, reifier, member_id.into_any())?
                    else {
                        continue;
                    };
                    let dir::Member::Method { signature, .. } = reifier.tree.get_mut(member_id)
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

    /// Reify the solved return type of one function declaration site.
    fn reify_return(
        &self,
        state: &CheckModuleState,
        reifier: &mut Reifier<'_, '_>,
        site: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::TypeExpression>>> {
        let Some(ty) = self.declaration_site_type(state, site) else {
            return Ok(None);
        };

        // look through the closure environment to the function contract
        let mut contract = self.resolve_root(ty)?;
        loop {
            match self.ty(contract)? {
                dir::Type::Closure(closure) => {
                    contract = self.resolve_root(closure.function)?;
                }
                dir::Type::Function(_) => break,
                _ => return Ok(None),
            }
        }
        let dir::Type::Function(function) = self.ty(contract)?.clone() else {
            unreachable!("the contract loop stops on function types");
        };

        reifier.anchor(self.site_anchor(state, site));
        match function.return_type {
            Some(return_type) => reifier.reify(return_type),
            // an omitted semantic return spells void
            None => Ok(Some(reifier.insert_keyword(dir::TypeLiteral::Void))),
        }
    }

    /// Return the solved type bound at one declaration site.
    fn declaration_site_type(
        &self,
        state: &CheckModuleState,
        site: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalTypeId> {
        let module_id = state.module.id;
        let symbol = state
            .bindings
            .declaration_symbol(site.into_global(module_id))?;

        self.inputs.symbol_type(symbol.into_global(module_id))
    }

    /// Return the source anchor span of one annotated site.
    fn site_anchor(&self, state: &CheckModuleState, site: dir::LocalNodeIdAny) -> Span {
        state.authored_span(site)
    }

    /// Spell recorded implicit coercions as explicit cast expressions.
    fn fill_coercions(
        &self,
        state: &CheckModuleState,
        reifier: &mut Reifier<'_, '_>,
    ) -> CompilerResult<()> {
        let module_id = state.module.id;
        let coercions = self
            .coercions
            .iter()
            .filter(|(node, _)| node.module_id == module_id)
            .map(|(node, coercion)| (*node, *coercion))
            .collect::<Vec<_>>();

        for (node, coercion) in coercions {
            // only authored expression nodes spell casts
            if node.local_id.ty != dir::NodeType::Expression || !state.is_authored(node.local_id) {
                continue;
            }

            reifier.anchor(self.site_anchor(state, node.local_id));
            let Some(target_type) = reifier.reify(coercion.target)? else {
                continue;
            };

            // wrap the coerced value in an explicit cast
            let id = dir::LocalNodeId::<dir::Expression>::new(node.local_id.id);
            let original = reifier.tree.get(id).clone();
            let span = state.authored_span(node.local_id);
            let expression = reifier.tree.insert(original, span);
            *reifier.tree.get_mut(id) = dir::Expression::As {
                expression,
                target_type,
            };
        }

        Ok(())
    }
}
