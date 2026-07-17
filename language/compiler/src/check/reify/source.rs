use std::sync::Arc;

use destack_dir as dir;
use destack_formatter::format_file_tree;
use destack_repository::{FormatterOptions, Module};
use destack_source::ModuleId;

use crate::check::reify::r#type::TypeReifier;
use crate::check::{CheckModuleState, CheckState, VarianceState};
use crate::{CompilerError, CompilerResult};

/// One module rendered with solved checked types.
pub(in crate::check) struct AnnotatedSource {
    /// The rendered module.
    pub(in crate::check) module: Arc<Module>,
    /// The formatted source text.
    pub(in crate::check) content: String,
}

impl CheckState<'_> {
    /// Render every member module's source with solved checked types.
    pub(in crate::check) fn render_annotated_sources(
        &mut self,
    ) -> CompilerResult<Vec<AnnotatedSource>> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();
        let mut sources = Vec::with_capacity(modules.len());
        for module_id in modules {
            let coercions = self.implicit_coercions(module_id)?;
            if let Some(source) = self.render_annotated_source(module_id, &coercions)? {
                sources.push(source);
            }
        }

        Ok(sources)
    }

    /// Render one member module's source with solved checked types.
    fn render_annotated_source(
        &mut self,
        module_id: ModuleId,
        coercions: &[(dir::GlobalNodeIdAny, dir::Coercion)],
    ) -> CompilerResult<Option<AnnotatedSource>> {
        let state = self.module(module_id);
        let file_id = state.module.file_id;
        let Some(parsed_file) = state.parsed.file(file_id) else {
            return Ok(None);
        };
        let roots = parsed_file.roots.as_slice();

        // write solved types into a cloned source tree
        let tree = SourceReifier::new(self, state).run(coercions)?;

        // print the amended tree through the canonical formatter
        let file = self.compiler.file(self.context, file_id)?;
        let tokens = parsed_file.iter_token_spans().collect::<Vec<_>>();
        let content = format_file_tree(
            file.as_ref(),
            &tree,
            &tokens,
            &parsed_file.comments,
            roots,
            self.strings(),
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
        let types = TypeReifier::new(check, tree, check.strings());

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

    /// Reify type-expression holes that carry solved check facts.
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

            // replace the hole in place with the reified annotation
            *self.types.tree.get_mut(hole_id) = self.types.tree.get(filled).clone();
        }

        Ok(())
    }

    /// Reify declaration-level checked types and induced lifetime parameters.
    fn reify_declarations(&mut self) -> CompilerResult<()> {
        let module_id = self.state.module.id;
        let view = dir::View::new(self.state.source_tree());
        for (declaration_id, declaration) in view.iter_nodes_of_type::<dir::Declaration>() {
            self.reify_declaration_generic_parameters(module_id, declaration_id)?;
            self.reify_declaration_return(declaration_id, declaration)?;
        }

        Ok(())
    }

    /// Reify induced lifetimes and derived variance for one declaration.
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
            if !binding.is_induced_lifetime_parameter() {
                continue;
            }

            self.anchor(declaration_id.into_any());
            let Some(parameter) = self.types.reify_generic_parameter(binding)? else {
                continue;
            };
            parameters.push(parameter);
        }

        let symbol = template.symbol;
        self.reify_declared_parameter_variance(
            module_id,
            declaration_id,
            symbol,
            &template.parameters,
        )?;
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

    /// Fill derived variance onto written type parameters.
    fn reify_declared_parameter_variance(
        &mut self,
        module_id: ModuleId,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        symbol: Option<dir::GlobalSymbolId>,
        parameters: &[dir::LocalGenericParameterId],
    ) -> CompilerResult<()> {
        // only nominal type parameters carry a relating variance
        let written = match self.types.tree.get(declaration_id) {
            dir::Declaration::Struct(_)
            | dir::Declaration::Class(_)
            | dir::Declaration::Enum(_)
            | dir::Declaration::Interface(_) => self.types.tree.get(declaration_id),
            dir::Declaration::Type(declaration) if declaration.is_nominal => {
                self.types.tree.get(declaration_id)
            }
            _ => return Ok(()),
        };
        let Some(written) = written.generic_parameters() else {
            return Ok(());
        };
        let written = written.to_vec();

        let Some(symbol) = symbol else {
            return Ok(());
        };
        let context = self.check.default_symbol_context(symbol);
        for (node, parameter) in written.iter().zip(parameters.iter()) {
            let parameter = dir::GlobalGenericParameterId::new(module_id, *parameter);

            // written modifiers and unmeasured parameters stay as written
            let Some(VarianceState::Derived(derived)) =
                self.check.variances.get(&(parameter, context))
            else {
                continue;
            };
            let Some(modifier) = derived.modifier() else {
                continue;
            };

            // fill the derived modifier onto unannotated type parameters
            match self.types.tree.get_mut(*node) {
                dir::GenericParameter::Type { variance, .. }
                | dir::GenericParameter::VariadicType { variance, .. }
                    if variance.is_none() =>
                {
                    *variance = Some(modifier);
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Reify one function declaration return type.
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

    /// Reify binding declarator checked types.
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

    /// Reify named parameter checked types.
    fn reify_parameters(&mut self) -> CompilerResult<()> {
        let view = dir::View::new(self.state.source_tree());
        for (parameter_id, parameter) in view.iter_nodes_of_type::<dir::Parameter>() {
            if let Some(dir::StaticKey::Name(name)) = parameter.symbol_key()
                && self.check.strings().get(name) == "this"
            {
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

    /// Reify member checked types.
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
                    self.reify_inferred_construct_head(module_id, expression_id, expression)?;
                    self.reify_call_arguments(module_id, expression_id, expression)?;
                }
                dir::Expression::New { .. } | dir::Expression::NewMaybe { .. } => {
                    self.reify_construct_arguments(module_id, expression_id, expression)?;
                }
                dir::Expression::StructExpression { ty, .. } => {
                    self.reify_struct_expression_target(module_id, expression_id, *ty)?;
                }
                dir::Expression::Infer {
                    form: dir::InferForm::Hole,
                    name: None,
                } => self.reify_static_expression_hole(module_id, expression_id)?,
                _ => {}
            }
        }

        Ok(())
    }

    /// Reify one solved static expression hole.
    fn reify_static_expression_hole(
        &mut self,
        module_id: ModuleId,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let source = expression_id.into_global_any(module_id);
        let Some(ty) = self.check.node_type_maybe(source) else {
            return Ok(());
        };

        self.anchor(expression_id.into_any());
        let Some(value) = self.types.reify_static(ty)? else {
            return Ok(());
        };
        *self.types.tree.get_mut(expression_id) = self.types.tree.get(value).clone();

        Ok(())
    }

    /// Reify one omitted newtype constructor head.
    fn reify_inferred_construct_head(
        &mut self,
        module_id: ModuleId,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) -> CompilerResult<()> {
        let dir::Expression::Call { left, .. } = expression else {
            return Ok(());
        };
        if !matches!(
            self.types.tree.get(*left),
            dir::Expression::Infer {
                form: dir::InferForm::Hole,
                name: None,
            }
        ) {
            return Ok(());
        }
        let Some(resolution) = self
            .check
            .resolutions(module_id)
            .construct_resolution(expression_id.into_global_any(module_id))
        else {
            return Ok(());
        };
        let dir::ConstructTarget::Newtype(candidate) = &resolution.target else {
            return Ok(());
        };

        self.anchor((*left).into_any());
        let Some(reified) = self.types.reify_symbol_expression(candidate.symbol) else {
            return Ok(());
        };
        let dir::Expression::Call { left, .. } = self.types.tree.get_mut(expression_id) else {
            unreachable!("call expressions keep their kind");
        };
        *left = reified;

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
        let Some(resolution) = self
            .check
            .resolutions(module_id)
            .call_resolution(expression_id.into_global_any(module_id))
        else {
            return Ok(());
        };

        let arguments = match &resolution.target {
            dir::CallTarget::Expression { generic_arguments } => generic_arguments.as_slice(),
            dir::CallTarget::Symbol(candidate) => candidate.generic_arguments.as_slice(),
            dir::CallTarget::Builtin(_) | dir::CallTarget::Universal(_) => return Ok(()),
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

    /// Reify one struct expression target from its selected type.
    fn reify_struct_expression_target(
        &mut self,
        module_id: ModuleId,
        expression_id: dir::LocalNodeId<dir::Expression>,
        ty: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        let can_reify = match self.types.tree.get(ty) {
            dir::TypeExpression::Infer {
                form: dir::InferForm::Hole,
                ..
            } => true,
            dir::TypeExpression::Reference {
                generic_arguments, ..
            } => generic_arguments.is_empty(),
            _ => false,
        };
        if !can_reify {
            return Ok(());
        };

        let source = ty.into_global_any(module_id);
        let node = expression_id.into_global_any(module_id);
        let target = self
            .check
            .node_type_maybe(source)
            .or_else(|| self.check.node_type_maybe(node));
        let Some(target) = target else {
            return Ok(());
        };

        self.anchor(ty.into_any());
        let Some(reified) = self.types.reify(target)? else {
            return Ok(());
        };
        let reified = self.types.tree.get(reified).clone();
        *self.types.tree.get_mut(ty) = reified;

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
        let Some(resolution) = self.check.resolutions(module_id).construct_resolution(node) else {
            return Ok(());
        };

        let arguments = match &resolution.target {
            dir::ConstructTarget::Class(candidate) => candidate.generic_arguments.as_slice(),
            dir::ConstructTarget::Newtype(candidate) => candidate.generic_arguments.as_slice(),
            dir::ConstructTarget::Variant(candidate) => candidate.generic_arguments.as_slice(),
        };
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
        let mut signature = self.check.settled_root(ty)?;
        loop {
            match self.check.ty(signature)? {
                dir::Type::Function(function) => {
                    signature = self.check.settled_root(function.signature)?;
                }
                dir::Type::FunctionSignature(_) => break,
                _ => return Ok(None),
            }
        }
        let Some(function) = self.check.signature_head(signature)? else {
            unreachable!("the signature loop stops on function signatures");
        };

        self.anchor(site);
        match function.return_type {
            Some(return_type) => self.types.reify(return_type),
            // omitted returns print as void
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

        self.check.symbol_type_maybe(symbol.into_global(module_id))
    }

    /// Anchor synthesized nodes at one source site.
    fn anchor(&mut self, site: dir::LocalNodeIdAny) {
        self.types.anchor(self.state.authored_span(site));
    }

    /// Reify implicit coercions as explicit cast expressions.
    fn reify_coercions(
        &mut self,
        coercions: &[(dir::GlobalNodeIdAny, dir::Coercion)],
    ) -> CompilerResult<()> {
        for (node, coercion) in coercions.iter().copied() {
            if node.local_id.ty != dir::NodeType::Expression {
                continue;
            }
            // carrier widening keeps the written literal (it would just be noisy)
            if coercion.kind == dir::CoercionKind::Widen {
                continue;
            }
            if !self.state.source_tree().has_node_id(node.local_id.id) {
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
