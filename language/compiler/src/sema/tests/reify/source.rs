use std::sync::Arc;

use tspp_artifact::{
    ArtifactKey, DiagnosticAnchor, DiagnosticContext, DiagnosticDisplay, DiagnosticError,
    DiagnosticLike, DiagnosticRecord, DirBound, DirChecked, DirDeclared, DirElaborated,
    DirExpanded, DirImported, DirParsed, DirResolved, DirView, EnvironmentBound,
    EnvironmentDeclared,
};
use tspp_dir as dir;
use tspp_formatter::format_file_tree;
use tspp_repository::{FormatterOptions, ProviderContext, Revision};
use tspp_source::{DiagnosticLabel, ModuleId, ProfileId};

use crate::sema::{CheckModuleState, CheckState, ExternalModuleTable, Pass};
use crate::{Compiler, CompilerError, CompilerResult};

use super::r#type::TypeReifier;

impl CheckState<'_> {
    /// Render the checked module source with solved types.
    fn render_checked_source(&mut self) -> CompilerResult<String> {
        let module_id = self.module_id;
        let state = self.module(module_id);
        let decisions = state.decisions.clone();
        let coercions = state.coercions.clone();
        let generics = state.generics.clone();

        self.render_source(module_id, decisions, coercions, generics)
    }

    /// Render one module source with solved checked types.
    fn render_source(
        &mut self,
        module_id: ModuleId,
        decisions: dir::DecisionTable<'static>,
        coercions: dir::CoercionTable<'static>,
        generics: dir::GenericTable<'static>,
    ) -> CompilerResult<String> {
        let state = self.module(module_id);
        let file_id = state.module.file_id;
        let uri = state.module.uri.clone();
        let parsed = Arc::clone(&state.parsed);
        let Some(parsed_file) = parsed.file(file_id) else {
            return Err(CompilerError::Internal {
                message: format!("checked module has no parsed source: {uri}"),
            });
        };
        let roots = parsed_file.roots.as_slice();

        // write solved types into a cloned source tree
        let reifier = SourceReifier::new(self, module_id, &parsed, decisions, coercions, generics);
        let tree = reifier.run()?;

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
            message: format!("annotated formatting failed for {uri}: {error}"),
        })?;

        Ok(content)
    }
}

impl Compiler {
    /// Render one checked module from its retained DIR artifacts.
    pub(crate) fn render_checked_source(
        &self,
        revision: Revision,
        artifact: ArtifactKey,
        module: ModuleId,
        profile: ProfileId,
    ) -> CompilerResult<String> {
        let context = SourceRenderContext { revision, artifact };
        let artifacts = self.repository.artifact_reader(revision);
        let environment_bound = artifacts
            .read::<EnvironmentBound>(profile)
            .map_err(CompilerError::from)?;
        let environment_declared = artifacts
            .read::<EnvironmentDeclared>(profile)
            .map_err(CompilerError::from)?;
        let environment = self.environment(context.revision())?;

        // load the retained checked module
        let view = DirView::checked(
            artifacts.read::<DirParsed>(module)?,
            artifacts.read::<DirBound>((module, profile))?,
            artifacts.read::<DirImported>((module, profile))?,
            artifacts.read::<DirExpanded>((module, profile))?,
            artifacts.read::<DirResolved>((module, profile))?,
            artifacts.read::<DirDeclared>((module, profile))?,
            artifacts.read::<DirElaborated>((module, profile))?,
            artifacts.read::<DirChecked>((module, profile))?,
        );
        let types = view.types().clone();
        let lists = dir::TypeListArena::following(
            types
                .segments()
                .last()
                .unwrap_or_else(|| unreachable!("a DIR view without type segments")),
        );
        let externals = ExternalModuleTable::default();
        let state = CheckModuleState::load(
            self,
            context.revision(),
            module,
            profile,
            view,
            &types,
            &lists,
        )?;

        // open check lookup over the retained tables and every module the checked rows mention
        let mut check = CheckState::new(
            self,
            &context,
            &artifacts,
            profile,
            environment_bound,
            Some(environment_declared),
            environment,
            state,
            &externals,
            Pass::Check,
            false,
        );
        check.import_external_modules()?;

        check.render_checked_source()
    }
}

/// Provider context for one checked-source render.
struct SourceRenderContext {
    /// The exact repository revision being rendered.
    revision: Revision,
    /// The checked artifact being rendered.
    artifact: ArtifactKey,
}

impl DiagnosticContext for SourceRenderContext {
    /// Reject diagnostic labels from source rendering.
    fn label(
        &self,
        _anchor: &DiagnosticAnchor,
        _message: Option<String>,
    ) -> Result<DiagnosticLabel, DiagnosticError> {
        Err(DiagnosticError::InvalidDiagnostic {
            message: "checked source rendering produced a diagnostic label".to_string(),
        })
    }

    /// Reject diagnostic displays from source rendering.
    fn display(&self, _display: DiagnosticDisplay) -> Result<String, DiagnosticError> {
        Err(DiagnosticError::InvalidDiagnostic {
            message: "checked source rendering produced a diagnostic display".to_string(),
        })
    }
}

impl ProviderContext for SourceRenderContext {
    /// Return the exact repository revision being rendered.
    fn revision(&self) -> Revision {
        self.revision
    }

    /// Return the checked artifact being rendered.
    fn artifact_key(&self) -> ArtifactKey {
        self.artifact
    }

    /// Reject diagnostics from source rendering.
    fn emit_diagnostics(&self, diagnostics: Vec<DiagnosticRecord>) {
        assert!(
            diagnostics.is_empty(),
            "checked source rendering produced diagnostics"
        );
    }

    /// Reject diagnostics from source rendering.
    fn emit(&self, _diagnostic: &dyn DiagnosticLike) -> Result<(), DiagnosticError> {
        Err(DiagnosticError::InvalidDiagnostic {
            message: "checked source rendering produced a diagnostic".to_string(),
        })
    }
}

/// Source reification pass for one checked module.
struct SourceReifier<'a, 'b> {
    /// The module being formatted.
    module_id: ModuleId,
    /// The parsed module the formatted tree copies.
    parsed: &'a DirParsed,
    /// The type-expression reifier writing synthesized nodes.
    types: TypeReifier<'a, 'b>,
    /// The exact inference decisions retained by checked DIR.
    decisions: dir::DecisionTable<'static>,
    /// The exact implicit coercions retained by checked DIR.
    coercions: dir::CoercionTable<'static>,
    /// The generic entries retained by checked DIR.
    generics: dir::GenericTable<'static>,
}

impl<'a, 'b> SourceReifier<'a, 'b> {
    /// Create a source reifier for one module.
    fn new(
        check: &'a mut CheckState<'b>,
        module_id: ModuleId,
        parsed: &'a DirParsed,
        decisions: dir::DecisionTable<'static>,
        coercions: dir::CoercionTable<'static>,
        generics: dir::GenericTable<'static>,
    ) -> Self {
        let tree = parsed.tree.clone();
        let strings = check.strings();
        let types = TypeReifier::new(check, tree, strings);

        Self {
            module_id,
            parsed,
            decisions,
            coercions,
            generics,
            types,
        }
    }

    /// Run source reification and return the amended tree.
    fn run(mut self) -> CompilerResult<dir::Tree> {
        self.reify_type_expressions()?;
        self.reify_declarations()?;
        self.reify_declarators()?;
        self.reify_parameters()?;
        self.reify_members()?;
        self.reify_expressions()?;
        self.reify_coercions()?;

        Ok(self.types.tree)
    }

    /// Reify type expression holes from their solved check results.
    fn reify_type_expressions(&mut self) -> CompilerResult<()> {
        let module_id = self.module_id;
        let view = dir::View::new(&self.parsed.tree);
        for (hole_id, expression) in view.iter_nodes::<dir::TypeExpression>() {
            if !matches!(
                expression,
                dir::TypeExpression::Infer {
                    form: dir::InferForm::Hole,
                    ..
                }
            ) {
                continue;
            }

            // read the hole's solved type from its node
            let ty = self
                .types
                .check
                .require_node_type(hole_id.into_global_any(module_id))?;
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
        let module_id = self.module_id;
        let view = dir::View::new(&self.parsed.tree);
        for (declaration_id, declaration) in view.iter_nodes::<dir::Declaration>() {
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
        // declared templates index by symbol, anonymous ones by source
        let source = declaration_id.into_global_any(module_id);
        let symbol = self
            .types
            .check
            .module(module_id)
            .declaration_symbol(declaration_id.into_any());
        let template = match symbol {
            Some(symbol) => self.generics.template_by_symbol(symbol),
            None => self.generics.template_by_source(source),
        };
        let Some(template_id) = template else {
            return Ok(());
        };
        let parameters = self.generics.get_template(template_id).parameters.clone();

        let mut induced = Vec::new();
        for parameter in &parameters {
            let binding = self.generics.get_parameter(*parameter).clone();
            if !binding.is_induced_region_parameter() {
                continue;
            }

            self.anchor(declaration_id.into_any());
            let id = parameter.into_global(module_id);
            let Some(parameter) = self.types.reify_generic_parameter(id, &binding)? else {
                continue;
            };
            induced.push(parameter);
        }

        self.reify_declared_parameter_variance(declaration_id, &parameters)?;
        if induced.is_empty() {
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
        generic_parameters.extend(induced);

        Ok(())
    }

    /// Fill derived variance onto written type parameters.
    fn reify_declared_parameter_variance(
        &mut self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        parameters: &[dir::LocalGenericParameterId],
    ) -> CompilerResult<()> {
        // annotate variance on nominal type parameters only
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

        for (node, parameter) in written.iter().zip(parameters.iter()) {
            // written modifiers and unmeasured parameters stay as written
            let Some(modifier) = self.generics.variance(*parameter) else {
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
        let view = dir::View::new(&self.parsed.tree);
        for (declarator_id, declarator) in view.iter_nodes::<dir::Declarator>() {
            if !matches!(view.get(declarator.pattern), dir::Pattern::Binding { .. }) {
                continue;
            }

            let Some(ty) = self.declaration_site_type(declarator.pattern.into_any())? else {
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
        let view = dir::View::new(&self.parsed.tree);
        for (parameter_id, parameter) in view.iter_nodes::<dir::Parameter>() {
            if let Some(dir::StaticKey::Name(name)) = parameter.symbol_key()
                && self.types.check.strings().get(name) == "this"
            {
                continue;
            }
            if !matches!(
                parameter,
                dir::Parameter::Named { .. } | dir::Parameter::VariadicNamed { .. }
            ) {
                continue;
            }

            let Some(ty) = self.declaration_site_type(parameter_id.into_any())? else {
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
        let view = dir::View::new(&self.parsed.tree);
        for (member_id, member) in view.iter_nodes::<dir::Member>() {
            match member {
                // reify field types
                dir::Member::Field { .. } => {
                    let Some(ty) = self.declaration_site_type(member_id.into_any())? else {
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
        let module_id = self.module_id;
        let view = dir::View::new(&self.parsed.tree);
        for (expression_id, expression) in view.iter_nodes::<dir::Expression>() {
            match expression {
                dir::Expression::Call { .. } => {
                    self.reify_inferred_construct_head(module_id, expression_id, expression)?;
                    self.reify_call_arguments(module_id, expression_id, expression)?;
                }
                dir::Expression::New { .. } => {
                    self.reify_construct_arguments(module_id, expression_id)?;
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
        let ty = self.types.check.require_node_type(source)?;

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
            .decisions
            .construct_decision(expression_id.into_global_any(module_id))
        else {
            return Ok(());
        };
        let dir::ConstructTarget::Newtype { key, .. } = &resolution.target else {
            return Ok(());
        };
        let symbol = key.symbol;

        self.anchor((*left).into_any());
        let Some(reified) = self.types.reify_symbol_expression(symbol)? else {
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
            .decisions
            .call_decision(expression_id.into_global_any(module_id))
        else {
            return Ok(());
        };

        let dir::OperationResolution::One(call) = resolution else {
            return Ok(());
        };
        let mut arguments = call.target.generic_arguments().to_vec();

        // print only the arguments selected at this invocation
        let callee = left.into_global_any(module_id);
        let ty = self.types.check.require_node_type(callee)?;
        if let Some(signature) = self.types.check.signature_head(ty)? {
            let fixed = self
                .types
                .check
                .signature_arguments(ty.module_id, signature.arguments)?;
            arguments.retain(|argument| {
                !fixed
                    .iter()
                    .any(|binding| binding.parameter == argument.parameter)
            });
        }
        if arguments.is_empty() {
            return Ok(());
        }

        self.anchor(expression_id.into_any());
        let Some(generic_arguments) = self.types.reify_generic_argument_bindings(&arguments)?
        else {
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

        let node = expression_id.into_global_any(module_id);
        let target = self.types.check.require_node_type(node)?;

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
    ) -> CompilerResult<()> {
        let dir::Expression::New {
            left,
            generic_arguments,
            ..
        } = self.types.tree.get(expression_id)
        else {
            unreachable!("construction expressions are selected by the caller");
        };
        if !generic_arguments.is_empty() {
            return Ok(());
        }

        // retain arguments already bound by the constructor value
        let callee = left.into_global_any(module_id);
        let callee_type = self.types.check.require_node_type(callee)?;
        if let dir::Type::Reference(reference) = self.types.check.ty(callee_type)?
            && !self
                .types
                .check
                .type_ids(callee_type.module_id, reference.arguments)?
                .is_empty()
        {
            return Ok(());
        }

        let node = expression_id.into_global_any(module_id);
        let Some(resolution) = self.decisions.construct_decision(node) else {
            return Ok(());
        };

        let arguments = match &resolution.target {
            dir::ConstructTarget::Class { key, .. } | dir::ConstructTarget::Newtype { key, .. } => {
                key.arguments.clone()
            }
        };
        if arguments.is_empty() {
            return Ok(());
        }

        self.anchor(expression_id.into_any());
        let Some(arguments) = self.types.reify_generic_argument_bindings(&arguments)? else {
            return Ok(());
        };
        let dir::Expression::New {
            generic_arguments, ..
        } = self.types.tree.get_mut(expression_id)
        else {
            unreachable!("construction expressions keep their kind");
        };
        *generic_arguments = arguments;

        Ok(())
    }

    /// Reify the solved return type of one function declaration site.
    fn reify_return(
        &mut self,
        site: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::TypeExpression>>> {
        let Some(ty) = self.declaration_site_type(site)? else {
            return Ok(None);
        };

        // look through function values to their signature
        let mut signature = self.types.check.shallow_resolve(ty)?;
        loop {
            match self.types.check.ty(signature)? {
                dir::Type::Function(function) => {
                    signature = self.types.check.shallow_resolve(function.signature)?;
                }
                dir::Type::FunctionSignature(_) => break,
                _ => return Ok(None),
            }
        }
        let Some(function) = self.types.check.signature_head(signature)? else {
            unreachable!("the signature loop stops on function signatures");
        };

        self.anchor(site);
        match function.return_type {
            Some(return_type) => self.types.reify(return_type),
            // print an omitted return as void
            None => Ok(Some(self.types.insert_keyword(dir::TypeLiteral::Void))),
        }
    }

    /// Return the solved type bound at one declaration site.
    fn declaration_site_type(
        &self,
        site: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let module_id = self.module_id;
        let Some(symbol) = self
            .types
            .check
            .module(module_id)
            .bindings
            .declaration_symbol(site.into_global(module_id))
        else {
            return Ok(None);
        };
        self.types
            .check
            .symbol_type_maybe(symbol.into_global(module_id))
    }

    /// Anchor synthesized nodes at one source site.
    fn anchor(&mut self, site: dir::LocalNodeIdAny) {
        let span = self.parsed.tree.source_index.get_main_or_enclosing(site.id);
        self.types.anchor(span);
    }

    /// Reify implicit coercions as explicit cast expressions.
    fn reify_coercions(&mut self) -> CompilerResult<()> {
        let coercions = self.coercions.clone();
        for (node, coercion) in coercions.coercions() {
            if node.local_id.ty != dir::NodeType::Expression {
                continue;
            }

            // keep numeric literal widening implicit, an explicit cast already spelled in source
            let renders = coercion.origin == dir::CastOrigin::Implicit
                && coercion.adjustments.iter().any(|adjustment| {
                    !matches!(adjustment, dir::CoercionAdjustment::Materialize { .. })
                });
            if !renders {
                continue;
            }
            if !self.parsed.tree.has_node_id(node.local_id.id) {
                continue;
            }

            self.anchor(node.local_id);
            let Some(target_type) = self.types.reify(coercion.target())? else {
                continue;
            };

            // wrap the coerced value in an explicit cast
            let id = dir::LocalNodeId::<dir::Expression>::new(node.local_id.id);
            let original = self.types.tree.get(id).clone();
            let expression = self.types.tree.insert_from(original, id);
            *self.types.tree.get_mut(id) = dir::Expression::As {
                expression,
                target_type,
            };
        }

        Ok(())
    }
}
