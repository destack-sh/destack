use std::slice::from_ref;

use destack_artifact::DirResolved;
use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
use rustc_hash::FxHashMap;

use crate::CompilerResult;
use crate::check::{
    Cause, CauseKind, CheckState, Constraint, GenericTemplateId, Obligation, Origin, PlaceUse,
    Relation, Task, TemplatePass, TypeSubstitution, WalkState, WellFormedTypeObligation,
};

impl CheckState<'_> {
    /// Declare every declaration template in one module.
    ///
    /// Example:
    /// ```ds
    /// class Box<T> {}
    /// ```
    pub(in crate::check) fn declare_module_templates(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        // bind nominal references before templates can read them
        self.bind_module_reference_types(module)?;

        self.visit_module_templates(module, TemplatePass::Declare)
    }

    /// Walk every declared template's bounds, defaults, and predicates.
    pub(in crate::check) fn walk_module_templates(
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
        let input = self.module(module);
        let parsed = input.parsed.clone();
        let expanded = input.expanded.clone();
        let tree = dir::View::with_patches(&parsed.tree, from_ref(&expanded.patch));

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
    /// ```ds
    /// export function value(): number { 1 }
    /// ```
    pub(in crate::check) fn walk_module_bodies(&mut self, module: ModuleId) -> CompilerResult<()> {
        let input = self.module(module);
        let parsed = input.parsed.clone();
        let expanded = input.expanded.clone();
        let tree = dir::View::with_patches(&parsed.tree, from_ref(&expanded.patch));

        // walk and queue each root's bodies, inducing body-written lifetimes
        let mut walk = WalkState::new(module, tree, self);
        for root in &expanded.roots {
            walk.visit_body_expression(*root)?;
            walk.queue_module_expression(*root)?;
            walk.check.induce_signature_lifetimes()?;
        }

        walk.commit()?;
        self.judge_written_types(module)?;

        Ok(())
    }

    /// Judge written types: application bounds, predicates, and wellformed rows.
    fn judge_written_types(&mut self, module: ModuleId) -> CompilerResult<()> {
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
        written.extend(self.node_types.iter().map(|(node, ty)| (*node, *ty)));

        let mut queued = FxIndexSet::default();
        for (source, ty) in written {
            if source.try_into_typed::<dir::TypeExpression>().is_err() {
                continue;
            }

            // judge application rows for argument bounds and predicates
            //  here, the one judgment site for built applications
            let mut head = ty;
            while let dir::Type::Refined(refined) = self.ty(head)? {
                head = self.type_refined(head.module_id, refined)?.base;
            }
            if let dir::Type::Application(instance) = self.ty(head)?
                && queued.insert(head)
            {
                let scope = self.enclosing_declared_template(source)?;
                self.queue_application_bounds(source, head, instance, scope)?;
            }

            let checked = matches!(self.operation_head(ty)?, Some(dir::TypeOperation::Index(_)))
                || matches!(
                    self.ty(ty)?,
                    dir::Type::Form(dir::FormType {
                        form: dir::Form::Placed { .. },
                        ..
                    })
                );
            if !checked {
                continue;
            }

            self.push_obligation(
                Obligation::WellFormedType(WellFormedTypeObligation { source, ty }),
                None,
            );
        }

        // declared symbol rows carry alias and annotation values
        for (symbol, ty) in symbols {
            if let dir::Type::Application(instance) = self.ty(ty)? {
                // commit written symbol values so bound failures close them,
                //  canonicalized symbols keep their canonical rows
                let existing = self.symbol_type_maybe(symbol);
                if existing.is_none() || existing == Some(ty) {
                    self.commit_declaration_type(symbol, ty)?;
                }
                if queued.insert(ty) {
                    let source = self.symbol_source(symbol)?;
                    let scope = self.loaded_symbol_template(symbol);
                    self.queue_application_bounds(source, ty, instance, scope)?;
                }
            }
        }

        Ok(())
    }

    /// Return the template of the declaration enclosing one node.
    fn enclosing_declared_template(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        let module = source.module_id;
        let Some(state) = self.module_maybe(module) else {
            return Ok(None);
        };
        let ancestors = state
            .parsed
            .tree
            .parents()
            .walk_parents_by_id(source.local_id.id);
        for ancestor in ancestors {
            let node = dir::LocalNodeId::<dir::Declaration>::new(ancestor).into_any();
            let Some(symbol) = self.module(module).declaration_symbol(node) else {
                continue;
            };
            // read loaded templates and signature heads only, demanding a
            //  definition would re-report its tagged derivation diagnostics
            if let Some(template) = self.loaded_symbol_template(symbol) {
                return Ok(Some(template));
            }
            if let Some(ty) = self.symbol_type_maybe(symbol)
                && let Some(head) = self.signature_head(ty)?
                && let Some(template) = head.template
            {
                return Ok(Some(template));
            }
        }

        Ok(None)
    }

    /// Queue bound and predicate constraints for one application row.
    fn queue_application_bounds(
        &mut self,
        source: dir::GlobalNodeIdAny,
        application: dir::GlobalTypeId,
        instance: dir::GenericApplication,
        scope: Option<GenericTemplateId>,
    ) -> CompilerResult<()> {
        let Some(template) = self.loaded_symbol_template(instance.symbol) else {
            return Ok(());
        };

        // resolve the template's declared parameters and applied arguments
        let parameters = self.generic_template_parameters(template)?;
        let arguments = self
            .type_ids(application.module_id, instance.arguments)?
            .to_vec();

        // build the substitution mapping parameters onto their arguments
        let substitution = TypeSubstitution {
            bindings: parameters
                .iter()
                .zip(&arguments)
                .map(|(parameter, argument)| {
                    dir::GenericArgumentBinding::new(*parameter, *argument)
                })
                .collect(),
            receiver: None
        };

        // enqueue parameter bounds as ordinary type relations
        for (index, (parameter, argument)) in parameters
            .iter()
            .copied()
            .zip(arguments.iter().copied())
            .enumerate()
        {
            let Some(constraint) = self
                .generic_parameter(parameter)
                .filter(|binding| binding.memory_parameter().is_none())
                .and_then(|binding| binding.constraint)
            else {
                continue;
            };
            let constraint = self.substitute_type(constraint, &substitution)?;

            if self.type_flags(argument)?.has_infer() {
                continue;
            }
            // skip this-typed bounds, they judge at conformance sites with a receiver
            if self.type_flags(argument)?.has_this() || self.type_flags(constraint)?.has_this() {
                continue;
            }

            let argument_source = self
                .written_generic_argument(source, index)
                .unwrap_or(source);
            let origin = Origin::Node(argument_source, scope);
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Bound { parameter }));
            self.push_constraint(Constraint::generic_bound(
                origin,
                argument,
                constraint,
                application,
                cause,
            ));
        }

        // enqueue declared where predicates with substituted sides,
        //  leaving predicates over this to conformance sites
        for predicate in self.template_predicates(Some(template)) {
            if self.type_flags(predicate.left)?.has_this() {
                continue;
            }
            let left = self.substitute_type(predicate.left, &substitution)?;
            let right = self.substitute_type(predicate.right, &substitution)?;
            let origin = Origin::Node(source, scope);
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            self.push_constraint(Constraint::r#type(
                origin,
                Relation::Satisfies,
                left,
                right,
                cause,
            ));
        }

        Ok(())
    }

    /// Return one written application's generic argument node by position.
    fn written_generic_argument(
        &self,
        source: dir::GlobalNodeIdAny,
        index: usize,
    ) -> Option<dir::GlobalNodeIdAny> {
        let module = self.module_maybe(source.module_id)?;
        let tree = dir::View::with_patches(&module.parsed.tree, from_ref(&module.expanded.patch));

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
    /// ```ds
    /// export function value(): number { 1 }
    /// ```
    pub(in crate::check) fn walk_module(&mut self, module: ModuleId) -> CompilerResult<()> {
        let input = self.module(module);
        let parsed = input.parsed.clone();
        let expanded = input.expanded.clone();
        let tree = dir::View::with_patches(&parsed.tree, from_ref(&expanded.patch));

        // walk and queue module roots, referenced declarations first
        let roots = ordered_roots(&input.resolved, &input.bindings, &tree, &expanded.roots);
        let mut walk = WalkState::new(module, tree, self);
        for root in &roots {
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
                    // drop the whole root on a static gate, checking the
                    //  let expression's ordinary decorators as values
                    if !walk.decide_static_presence((*root).into_any())? {
                        continue;
                    }
                    walk.walk_let_bindings(*root)?;
                }
                _ => continue,
            }

            walk.queue_module_expression(*root)?;
            walk.check.induce_signature_lifetimes()?;
        }

        walk.commit()?;

        Ok(())
    }
}

impl WalkState<'_, '_> {
    /// Queue one module-scope expression for inference.
    fn queue_module_expression(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let node = expression.into_global_any(self.module);
        if self.check.is_absent(node) {
            return Ok(());
        }

        // transcribe declarations / bindings only
        if self.check.is_declaration()
            && !matches!(
                self.tree.get(expression),
                dir::Expression::Declaration(_) | dir::Expression::Let { .. }
            )
        {
            return Ok(());
        }

        let expressions = match self.tree.get(expression) {
            dir::Expression::Declaration(declaration) => match self.tree.get(*declaration) {
                dir::Declaration::Global(declaration) => Some(declaration.expressions.clone()),
                dir::Declaration::Module(declaration) => Some(declaration.expressions.clone()),
                _ => None,
            },
            _ => None,
        };

        // queue expressions nested by global and module declarations
        if let Some(expressions) = expressions {
            for expression in expressions {
                self.queue_module_expression(expression)?;
            }

            return Ok(());
        }

        let site = self.node_site(expression)?;
        self.check.queue_task(Task::Infer {
            site,
            use_: PlaceUse::Read,
        });

        Ok(())
    }
}

/// Return one module's roots with referenced declarations ordered first.
///
/// Reference edges come from the resolve stage: a root walks after the
/// roots declaring the symbols it references, so induced parameters exist
/// when applications reach them. Cyclic groups keep their source order.
fn ordered_roots(
    resolved: &DirResolved,
    bindings: &dir::BindingTable<'_>,
    tree: &dir::View<'_>,
    roots: &[dir::LocalNodeId<dir::Expression>],
) -> Vec<dir::LocalNodeId<dir::Expression>> {
    // map each root to its ordinal for edge building
    let mut ordinals = FxHashMap::default();
    for (ordinal, root) in roots.iter().enumerate() {
        ordinals.insert(root.id, ordinal);
    }

    // climb one node to the root that owns it
    let owner = |node: u32| -> Option<usize> {
        let mut current = node;
        loop {
            if let Some(ordinal) = ordinals.get(&current) {
                return Some(*ordinal);
            }
            current = tree.get_parent_id(current)?;
        }
    };

    // collect reference edges between distinct roots
    let module = resolved.references.module_id;
    let mut edges: Vec<FxIndexSet<usize>> = vec![FxIndexSet::default(); roots.len()];
    for (node, reference) in &resolved.references.target_by_node {
        let dir::Reference::Bound(symbols) = reference else {
            continue;
        };
        let Some(consumer) = owner(node.local_id.id) else {
            continue;
        };
        for symbol in symbols {
            if symbol.module_id != module {
                continue;
            }
            let declaration = bindings.get_symbol(symbol.local_id).declaration;
            let Some(target) = declaration.and_then(|node| owner(node.local_id.id)) else {
                continue;
            };
            if target != consumer {
                edges[consumer].insert(target);
            }
        }
    }

    // emit referenced roots before their consumers in source order
    let mut ordered = Vec::with_capacity(roots.len());
    let mut states = vec![VisitState::Fresh; roots.len()];
    let mut stack = Vec::new();
    for start in 0..roots.len() {
        if states[start] != VisitState::Fresh {
            continue;
        }
        stack.push((start, 0));
        while let Some((root, next)) = stack.pop() {
            if states[root] == VisitState::Emitted {
                continue;
            }
            states[root] = VisitState::Visiting;
            if let Some(target) = edges[root].get_index(next) {
                stack.push((root, next + 1));
                if states[*target] == VisitState::Fresh {
                    stack.push((*target, 0));
                }

                continue;
            }
            states[root] = VisitState::Emitted;
            ordered.push(roots[root]);
        }
    }

    ordered
}

/// The visit state of one root during dependency ordering.
#[derive(Clone, Copy, PartialEq, Eq)]
enum VisitState {
    /// Not yet reached.
    Fresh,
    /// On the visit stack, cycles fall back to source order.
    Visiting,
    /// Emitted into the ordered list.
    Emitted,
}
