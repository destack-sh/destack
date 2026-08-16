use std::collections::VecDeque;

use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::sema::{CheckState, Origin, TypeSubstitution};
use crate::{CompilerError, CompilerResult};

/// Cap on instantiation chain depth, guarding polymorphic recursion.
const INSTANCE_DEPTH_LIMIT: u32 = 128;

/// One instance identity: the template and its closed arguments.
type InstanceIdentity = (dir::GlobalSymbolId, Vec<dir::GenericArgumentBinding>);

impl CheckState<'_> {
    /// Close the instances this module's instantiations reach, to a fixpoint.
    pub(in crate::sema) fn materialize_instances(&mut self) -> CompilerResult<()> {
        // read the checked stage this pass materializes
        let checked = self
            .module
            .checked
            .clone()
            .ok_or_else(|| CompilerError::Internal {
                message: "materialize runs over a checked module".to_string(),
            })?;

        // seed from the module-level instantiations check recorded
        let mut seen: FxIndexMap<InstanceIdentity, dir::LocalInstanceId> = FxIndexMap::default();
        let mut queue: VecDeque<(dir::LocalInstanceId, u32)> = VecDeque::new();
        for instantiation in checked.generics.iter_instantiations() {
            if instantiation.owner.is_none() {
                let instantiation = instantiation.clone();
                self.admit_instance(
                    instantiation.selection.symbol,
                    instantiation.selection.arguments,
                    instantiation.source,
                    0,
                    &mut seen,
                    &mut queue,
                )?;
            }
        }

        // close each admitted instance's own instantiations until nothing new appears
        while let Some((instance, depth)) = queue.pop_front() {
            self.close_instance(instance, depth, &mut seen, &mut queue)?;
        }

        Ok(())
    }

    /// Admit one instantiation as an instance, deduplicating structurally.
    fn admit_instance(
        &mut self,
        template: dir::GlobalSymbolId,
        bindings: Vec<dir::GenericArgumentBinding>,
        source: dir::GlobalNodeIdAny,
        depth: u32,
        seen: &mut FxIndexMap<InstanceIdentity, dir::LocalInstanceId>,
        queue: &mut VecDeque<(dir::LocalInstanceId, u32)>,
    ) -> CompilerResult<()> {
        // reject runaway polymorphic recursion at the depth rustc allows
        if depth > INSTANCE_DEPTH_LIMIT {
            return Err(CompilerError::Internal {
                message: format!(
                    "instantiation chain exceeded depth {INSTANCE_DEPTH_LIMIT} closing {template:?}"
                ),
            });
        }

        // resolve the arguments, keeping type parameters and dropping lifetimes
        let origin = Origin::Node(source, None);
        let mut arguments = Vec::new();
        for binding in bindings {
            if self.is_lifetime_parameter(binding.parameter) {
                continue;
            }

            let argument = self.deeply_resolve(origin, binding.argument)?;

            // leave open instantiations to close under their enclosing instance
            let flags = self.type_flags(argument)?;
            if flags.has_parameter() || flags.has_variable() || flags.has_this() {
                return Ok(());
            }

            arguments.push(dir::GenericArgumentBinding::new(
                binding.parameter,
                argument,
            ));
        }

        // a selection without type arguments closes nothing
        if arguments.is_empty() {
            return Ok(());
        }

        // partial selections stay open: an instance binds every declared parameter
        if let Some(template_id) = self.symbol_template(template)?
            && let Some(declared) = self.generic_template(template_id)
        {
            let parameters = declared.parameters.clone();
            for parameter in parameters {
                let parameter = parameter.into_global(template_id.module_id);
                if self.is_lifetime_parameter(parameter) {
                    continue;
                }
                if !arguments
                    .iter()
                    .any(|binding| binding.parameter == parameter)
                {
                    return Ok(());
                }
            }
        }

        // allocate one row per distinct (template, arguments) pair
        let key = (template, arguments.clone());
        if seen.contains_key(&key) {
            return Ok(());
        }

        let instance = self.module.generics_tail.push_instance(dir::Instance {
            selection: dir::Selection::new(template, arguments),
            source,
        });
        seen.insert(key, instance);
        queue.push_back((instance, depth));

        Ok(())
    }

    /// Close one instance: admit its template's instantiations and settle its rows.
    fn close_instance(
        &mut self,
        instance: dir::LocalInstanceId,
        depth: u32,
        seen: &mut FxIndexMap<InstanceIdentity, dir::LocalInstanceId>,
        queue: &mut VecDeque<(dir::LocalInstanceId, u32)>,
    ) -> CompilerResult<()> {
        // read the instance row and the substitution its arguments select
        let row = self
            .module
            .generics_tail
            .get_local_instance(instance)
            .ok_or_else(|| CompilerError::Internal {
                message: "closed an unallocated instance".to_string(),
            })?
            .clone();
        let substitution = TypeSubstitution {
            bindings: row.selection.arguments.iter().copied().collect(),
            receiver: None,
        };
        let origin = Origin::Node(row.source, None);

        // substitute the template's recorded instantiations under this instance
        let mut reached = Vec::new();
        for instantiation in self.template_instantiations(row.selection.symbol)? {
            let mut arguments = Vec::new();
            for binding in instantiation.selection.arguments {
                arguments.push(dir::GenericArgumentBinding::new(
                    binding.parameter,
                    self.substitute_type(binding.argument, &substitution)?,
                ));
            }
            reached.push((instantiation.selection.symbol, arguments));
        }

        // admit the ones that closed, one chain step deeper
        for (template, arguments) in reached {
            self.admit_instance(template, arguments, row.source, depth + 1, seen, queue)?;
        }

        // settle the template's rows under the substitution for this instance
        match self.definition(row.selection.symbol)?.cloned() {
            Some(definition) => {
                self.settle_instance_definition(instance, origin, definition, &substitution)
            }
            None => {
                self.settle_instance_body(instance, origin, row.selection.symbol, &substitution)
            }
        }
    }

    /// Return one template's recorded instantiations, arguments as written.
    fn template_instantiations(
        &self,
        template: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::Instantiation>> {
        // read the own committed rows or the loaded foreign rows
        let owner = Some(template);
        if self.is_own_module(template.module_id) {
            let checked = self
                .module
                .checked
                .as_ref()
                .ok_or_else(|| CompilerError::Internal {
                    message: "materialize runs over a checked module".to_string(),
                })?;

            Ok(checked
                .generics
                .iter_instantiations()
                .filter(|instantiation| instantiation.owner == owner)
                .cloned()
                .collect())
        } else {
            let external = self
                .external_modules
                .get(&template.module_id)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("template module {:?} was not loaded", template.module_id),
                })?;

            Ok(external
                .generics
                .instantiations_of(owner)
                .cloned()
                .collect())
        }
    }

    /// Bind the materialized member types of one type template's definition.
    fn settle_instance_definition(
        &mut self,
        instance: dir::LocalInstanceId,
        origin: Origin,
        definition: dir::Definition,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<()> {
        let mut folded = definition;
        dir::TypeFold::map_types(&mut folded, &mut |ty| {
            self.settle_instance_type(instance, origin, ty, substitution)
        })?;

        Ok(())
    }

    /// Bind the materialized types of one callable template's body.
    fn settle_instance_body(
        &mut self,
        instance: dir::LocalInstanceId,
        origin: Origin,
        template: dir::GlobalSymbolId,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<()> {
        // bodiless templates contribute no rows beyond their own
        let Some(nodes) = self.template_body_nodes(template)? else {
            return Ok(());
        };

        // fold each body node's committed type under the substitution
        for node in nodes {
            if let Some(ty) = self.committed_template_type(template.module_id, node) {
                self.settle_instance_type(instance, origin, ty, substitution)?;
            }
        }

        Ok(())
    }

    /// Substitute and evaluate one template type, recording the row when evaluation moves it.
    fn settle_instance_type(
        &mut self,
        instance: dir::LocalInstanceId,
        origin: Origin,
        ty: dir::GlobalTypeId,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // closed types are identical across instances and stay as written
        let flags = self.type_flags(ty)?;
        if !flags.has_parameter() && !flags.has_this() {
            return Ok(ty);
        }

        // record a row only where evaluation moves the substituted type
        let substituted = self.substitute_type(ty, substitution)?;
        let resolved = self.evaluate_type(origin, substituted)?;
        if resolved != substituted {
            self.module
                .generics_tail
                .bind_instance_type(instance, ty, resolved);
        }

        Ok(resolved)
    }

    /// Return every node inside one callable template's body, when it has one.
    fn template_body_nodes(
        &mut self,
        template: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<Vec<dir::GlobalNodeIdAny>>> {
        // find the template's declaration in its defining module
        let declaration = self
            .binding_table(template.module_id)
            .get_symbol(template.local_id)
            .declaration;
        let Some(declaration) = declaration else {
            return Ok(None);
        };
        let tree = self.template_tree(template.module_id)?;

        // read the body expression behind a plain or member function declaration
        let body = if let Ok(member) = declaration.local_id.try_into_typed::<dir::Member>() {
            match tree.get(member) {
                dir::Member::Method { body, .. } => *body,
                _ => None,
            }
        } else if let Ok(declaration) = declaration.local_id.try_into_typed::<dir::Declaration>() {
            match tree.get(declaration) {
                dir::Declaration::Function(function) => function.body,
                _ => None,
            }
        } else {
            None
        };
        let Some(body) = body else {
            return Ok(None);
        };

        // collect every node in the body subtree
        let mut collector = BodyNodeCollector {
            options: dir::NodeVisitorOptions::default(),
            module: template.module_id,
            nodes: Vec::new(),
        };
        dir::NodeVisitor::visit_expression(&mut collector, tree, body, tree.get(body));

        Ok(Some(collector.nodes))
    }

    /// Return one template module's parsed tree.
    fn template_tree(&self, module: ModuleId) -> CompilerResult<&dir::Tree> {
        if self.is_own_module(module) {
            return Ok(&self.module.parsed.tree);
        }

        self.external_modules
            .get(&module)
            .map(|external| &external.parsed.tree)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("template module {module:?} was not loaded"),
            })
    }

    /// Return one template body node's committed type.
    fn committed_template_type(
        &self,
        module: ModuleId,
        node: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalTypeId> {
        if self.is_own_module(module) {
            return self.module.types.get_node_type_id(node);
        }

        self.external_modules
            .get(&module)?
            .types
            .get_node_type_id(node)
    }
}

/// Visitor collecting every node in one body subtree.
struct BodyNodeCollector {
    /// The visitor options.
    options: dir::NodeVisitorOptions,
    /// The visited module.
    module: ModuleId,
    /// Every collected node, of any kind.
    nodes: Vec<dir::GlobalNodeIdAny>,
}

impl dir::NodeVisitor for BodyNodeCollector {
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    fn visit_any(&mut self, _tree: &dir::Tree, ty: dir::NodeType, id: u32) {
        self.nodes.push(dir::GlobalNodeIdAny {
            module_id: self.module,
            local_id: dir::LocalNodeIdAny::new(id, ty),
        });
    }
}
