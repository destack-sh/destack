use tspp_core::FxIndexSet;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::CompilerResult;
use crate::sema::{
    Cause, CauseKind, CheckState, GenericTemplateId, InferMode, Obligation, Origin, PlaceUse,
    Relation, RelationCheck, RestParameterObligation, Settle, TemplatePass, TypeSubstitution,
    WalkState, WellFormedTypeObligation,
};

impl CheckState<'_> {
    /// Declare every declaration template in one module.
    ///
    /// Example:
    /// ```tspp
    /// class Box<T> {}
    /// ```
    pub(in crate::sema) fn declare_module_templates(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        // bind nominal references before templates can read them
        self.commit_module_reference_types(module)?;

        self.visit_module_templates(module, TemplatePass::Declare)
    }

    /// Walk every declared template's bounds, defaults, and predicates.
    pub(in crate::sema) fn walk_module_templates(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        self.visit_module_templates(module, TemplatePass::Walk)
    }

    /// Visit every declaration template in one module.
    fn visit_module_templates(
        &mut self,
        module: ModuleId,
        pass: TemplatePass,
    ) -> CompilerResult<()> {
        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::new(&parsed.tree).patched(&expanded.patch);

        // walk the module over its patched view
        let mut walk = WalkState::new(module, tree, self);

        // visit declaration templates before any body walks
        for root in &expanded.roots {
            walk.visit_expression_templates(*root, tree.get(*root), pass)?;
        }
        walk.commit()?;

        Ok(())
    }

    /// Visit DIR and collect check constraints and obligations.
    ///
    /// Example:
    /// ```tspp
    /// export function value(): number { 1 }
    /// ```
    pub(in crate::sema) fn walk_module_bodies(&mut self, module: ModuleId) -> CompilerResult<()> {
        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::new(&parsed.tree).patched(&expanded.patch);

        // walk and type each root in source order
        let mut walk = WalkState::new(module, tree, self);
        for root in &expanded.roots {
            walk.with_body_scope(|walk| {
                // register root decorators and gate absent roots
                if !walk.walk_decorators(root.into_any())? {
                    return Ok(());
                }
                walk.type_body_root(module, *root)?;
                walk.flush_flows()
            })?;
        }

        // check function and member block bodies in discovery order
        let mut next_body = 0;
        let mut next_block = 0;
        loop {
            let function = walk
                .check
                .functions
                .get_index(next_body)
                .map(|(_, function)| function.clone());
            if let Some(function) = function {
                next_body += 1;
                let return_type = function.return_type;
                walk.check
                    .with_body_scope(|check| function.check(check, InferMode::Regular, None))?;

                // settle a declaration's inferred return hole from what its body returned
                if let Some(return_type) = return_type {
                    let roots = walk.check.collect_open_variables([return_type])?;
                    if !roots.is_empty() {
                        walk.check.settle_variables(&roots, Settle::All)?;
                    }
                }
                walk.flush_flows()?;

                continue;
            }

            // type queued member blocks like module roots
            let Some(block) = walk.check.blocks.get(next_block).copied() else {
                break;
            };
            next_block += 1;
            walk.check.with_body_scope(|check| {
                let site = check.visit_site(block)?;
                check.attempt_node(site, PlaceUse::Read, None)
            })?;
            walk.flush_flows()?;
        }

        // commit the walk, then oblige every written type
        walk.commit()?;
        self.oblige_written_types(module)?;

        Ok(())
    }

    /// Oblige written types: application bounds, predicates, and well-formed entries.
    fn oblige_written_types(&mut self, module: ModuleId) -> CompilerResult<()> {
        // collect the declared and committed type entries of this module
        let mut written = Vec::new();
        let mut symbols = Vec::new();
        if let Some(declared) = &self.module(module).declared {
            written.extend(declared.types.node_types());
            symbols.extend(declared.types.symbol_types());
            for (symbol, definition) in declared.definitions.iter_definitions() {
                if let dir::Definition::TypeAlias(alias) = definition {
                    symbols.push((symbol, alias.value));
                }
            }
        }
        let committed = self.node_types.nodes();
        for node in committed {
            if let Some(ty) = self.node_types.get(&node) {
                written.push((node, ty));
            }
        }

        // oblige each written type entry once
        let mut obliged = FxIndexSet::default();
        for (source, ty) in written {
            // oblige declared and inferred rest parameters when checking the module
            if let Ok(parameter) = source.try_into_typed::<dir::Parameter>() {
                if matches!(
                    self.module(source.module_id).view().get(parameter.local_id),
                    dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. }
                ) {
                    let scope = self.template_at_node(source)?;
                    self.push_obligation(
                        Obligation::RestParameter(RestParameterObligation { source, ty }),
                        scope,
                    )?;
                }

                continue;
            }

            // validate written type expressions through their operations and applications
            if source.try_into_typed::<dir::TypeExpression>().is_err() {
                continue;
            }

            // oblige the argument bounds and predicates of a built application
            let mut head = self.shallow_resolve(ty)?;
            while let dir::Type::Refined(refined) = self.ty(head)? {
                let base = self.type_refined(head.module_id, refined)?.base;
                head = self.shallow_resolve(base)?;
            }
            if let dir::Type::Application(instance) = self.ty(head)?
                && obliged.insert(head)
            {
                let scope = self.template_at_node(source)?;
                self.collect_application_bounds(source, head, instance, scope)?;
            }

            // oblige index operations to be well-formed
            let resolved = self.shallow_resolve(ty)?;
            let checked = matches!(
                self.operation_head(resolved)?,
                Some(dir::TypeOperation::Index(_))
            );
            if !checked {
                continue;
            }

            let scope = self.template_at_node(source)?;
            self.push_obligation(
                Obligation::WellFormedType(WellFormedTypeObligation { source, ty }),
                scope,
            )?;
        }

        // oblige the alias and annotation values of declared symbols
        for (symbol, ty) in symbols {
            if let dir::Type::Application(instance) = self.ty(ty)? {
                // commit written symbol values so bound failures close them
                let existing = self.symbol_type_maybe(symbol)?;
                if existing.is_none() || existing == Some(ty) {
                    self.commit_declaration_type(symbol, ty)?;
                }
                if obliged.insert(ty) {
                    let source = self.symbol_source(symbol)?;
                    let scope = self.symbol_template(symbol)?;
                    self.collect_application_bounds(source, ty, instance, scope)?;
                }
            }
        }

        Ok(())
    }

    /// Collect bound and predicate constraints for one application entry.
    fn collect_application_bounds(
        &mut self,
        source: dir::GlobalNodeIdAny,
        application: dir::GlobalTypeId,
        instance: dir::GenericApplication,
        scope: Option<GenericTemplateId>,
    ) -> CompilerResult<()> {
        let Some(template) = self.symbol_template(instance.symbol)? else {
            return Ok(());
        };

        // resolve the template's declared parameters and applied arguments
        let parameters = self.generic_template_parameters(template)?;
        let arguments = self.type_ids(application.module_id, instance.arguments)?;

        // build the substitution mapping parameters onto their arguments
        let substitution = TypeSubstitution {
            bindings: parameters
                .iter()
                .zip(arguments)
                .map(|(parameter, argument)| {
                    dir::GenericArgumentBinding::new(*parameter, *argument)
                })
                .collect(),
            receiver: None,
        };

        // collect parameter bounds as ordinary type relations
        for (index, (parameter, argument)) in parameters
            .iter()
            .copied()
            .zip(arguments.iter().copied())
            .enumerate()
        {
            self.report_argument_cardinality(source, index, scope, parameter, argument)?;

            let Some(constraint) = self
                .generic_parameter(parameter)?
                .filter(|binding| binding.memory_parameter().is_none())
                .and_then(|binding| binding.constraint)
            else {
                continue;
            };
            let constraint = self.substitute_type(constraint, &substitution)?;

            if self.type_flags(argument)?.has_infer() {
                continue;
            }
            // skip this-typed bounds, they oblige at conformance sites with a receiver
            if self.type_flags(argument)?.has_this() || self.type_flags(constraint)?.has_this() {
                continue;
            }

            let argument_source = self.generic_argument_node(source, index).unwrap_or(source);
            let origin = Origin::Node(argument_source, scope);
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Bound { parameter }));
            self.push_relation(RelationCheck::generic_bound(
                origin,
                argument,
                constraint,
                application,
                cause,
            ))?;
        }

        // collect the declared where predicates with substituted sides
        for predicate in self.template_predicates(Some(template))? {
            if self.type_flags(predicate.left)?.has_this() {
                continue;
            }
            let left = self.substitute_type(predicate.left, &substitution)?;
            let right = self.substitute_type(predicate.right, &substitution)?;
            let origin = Origin::Node(source, scope);
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            self.push_relation(RelationCheck::new(
                origin,
                Relation::Subtype,
                left,
                right,
                cause,
            ))?;
        }

        Ok(())
    }

    /// Report one written argument the declaration's exact value slot rejects.
    fn report_argument_cardinality(
        &mut self,
        source: dir::GlobalNodeIdAny,
        index: usize,
        scope: Option<GenericTemplateId>,
        parameter: dir::GlobalGenericParameterId,
        argument: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        if !self.is_static_const_parameter(parameter)? {
            return Ok(());
        }

        let argument_source = self.generic_argument_node(source, index).unwrap_or(source);
        let origin = Origin::Node(argument_source, scope);

        if !self.has_one_cardinality(origin, argument)? {
            self.report_argument_not_exact_value(argument_source, argument, parameter)?;
        }

        Ok(())
    }

    /// Return one written application's generic argument node by position.
    pub(in crate::sema) fn generic_argument_node(
        &self,
        source: dir::GlobalNodeIdAny,
        index: usize,
    ) -> Option<dir::GlobalNodeIdAny> {
        let module = self.module_maybe(source.module_id)?;
        let tree = dir::View::new(&module.parsed.tree).patched(&module.expanded.patch);

        // alias declarations apply in their value expression
        let node = match source.try_into_typed::<dir::Declaration>() {
            Ok(declaration) => match tree.get(declaration.local_id) {
                dir::Declaration::Type(alias) => alias.value,
                _ => return None,
            },
            Err(_) => {
                source
                    .try_into_typed::<dir::TypeExpression>()
                    .ok()?
                    .local_id
            }
        };

        // read the generic argument list off the referenced type expression
        let (dir::TypeExpression::Reference {
            generic_arguments, ..
        }
        | dir::TypeExpression::Member {
            generic_arguments, ..
        }) = tree.get(node)
        else {
            return None;
        };

        Some(
            generic_arguments
                .get(index)?
                .into_global_any(source.module_id),
        )
    }

    /// Visit DIR and collect check constraints and obligations.
    ///
    /// Example:
    /// ```tspp
    /// export function value(): number { 1 }
    /// ```
    pub(in crate::sema) fn walk_module(&mut self, module: ModuleId) -> CompilerResult<()> {
        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::new(&parsed.tree).patched(&expanded.patch);

        // walk module roots in source order
        let mut walk = WalkState::new(module, tree, self);
        for root in &expanded.roots {
            walk.induced_owner = None;
            walk.with_scope(|walk| {
                match tree.get(*root) {
                    dir::Expression::Declaration(declaration) => {
                        let declaration = *declaration;
                        walk.enter_node(*root)?;
                        let void = walk.intern_type(dir::Type::Void)?;
                        walk.commit_node_type(*root, void)?;
                        walk.walk_declaration(declaration, &tree.get(declaration).clone())?;
                    }
                    dir::Expression::Let { .. } => {
                        walk.enter_node(*root)?;

                        // drop the whole root on a static gate
                        if !walk.decide_static_presence((*root).into_any())? {
                            return Ok(());
                        }
                        walk.walk_let_bindings(*root)?;
                    }
                    _ => return Ok(()),
                }

                walk.flush_flows()?;

                walk.type_body_root(module, *root)
            })?;
        }

        walk.commit()?;

        Ok(())
    }
}

impl WalkState<'_, '_> {
    /// Type one walked body root, recursing into block members.
    fn type_body_root(
        &mut self,
        module: ModuleId,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        if let dir::Expression::Declaration(declaration) = self.tree.get(node) {
            let declaration = *declaration;
            let members = match self.tree.get(declaration) {
                dir::Declaration::Global(block) => Some(block.expressions.clone()),
                dir::Declaration::Module(block) => Some(block.expressions.clone()),
                _ => None,
            };
            if let Some(members) = members {
                for member in members {
                    self.type_body_root(module, member)?;
                }

                return Ok(());
            }
        }

        // type the root itself through the body visitor
        let root_node = node.into_global_any(module);
        let site = self.check.visit_site(root_node)?;
        self.check.attempt_node(site, PlaceUse::Read, None)?;

        Ok(())
    }
}
