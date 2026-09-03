use destack_dir as dir;
use destack_source::ModuleId;

use crate::sema::{CheckState, Origin, TypeSubstitution};
use crate::{CompilerError, CompilerResult};

use super::entry::Materialization;
use super::instance::InstanceWorklist;

impl CheckState<'_> {
    /// Materialize the `this`-typed entries of every non-generic member body under its owner.
    pub(in crate::sema) fn materialize_member_bodies(
        &mut self,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        // collect the methods the module's own definitions declare
        let mut members = Vec::new();
        for (owner, definition) in self.module.iter_definitions() {
            for member in definition.members() {
                if let dir::DefinitionMember::Method(method) = member {
                    members.push((owner, method.symbol));
                }
            }
        }

        // materialize each body outside a generic; instances cover the rest
        for (owner, member) in members {
            if self.symbol_has_type_parameters(owner)? || self.symbol_has_type_parameters(member)? {
                continue;
            }

            // an extension body reads `this` as the target, a nominal body as the owner itself
            let receiver = match self.definition(owner)?.cloned() {
                Some(dir::Definition::Extension(extension)) => extension.target.r#type(),
                _ => self.intern_type(dir::Type::Application(dir::GenericApplication {
                    symbol: owner,
                    arguments: dir::TypeListId::EMPTY,
                }))?,
            };
            let substitution = TypeSubstitution::default().with_receiver(receiver);
            let materialization = Materialization {
                substitution: Some(&substitution),
                owner_self: None,
                instance: None,
                anchor: None,
                depth: 0,
            };
            self.materialize_template_entries(&materialization, member, worklist)?;
        }

        Ok(())
    }

    /// Return whether one symbol's template declares type parameters.
    pub(in crate::sema) fn symbol_has_type_parameters(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let Some(template_id) = self.symbol_template(symbol)? else {
            return Ok(false);
        };
        let Some(template) = self.generic_template(template_id) else {
            return Ok(false);
        };

        // accept any parameter beyond the memory kinds
        let parameters = template.parameters.clone();
        for parameter in parameters {
            let parameter = parameter.into_global(template_id.module_id);
            let is_memory = self
                .generic_parameter(parameter)
                .is_some_and(|binding| binding.memory_parameter().is_some());
            if !is_memory {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return one symbol's committed type in its loaded module.
    pub(super) fn committed_symbol_type(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        self.committed(symbol.module_id)?
            .types
            .get_symbol_type_id(symbol)
    }

    /// Return every node inside the member initializers one type template declares.
    pub(super) fn template_initializer_body(
        &self,
        template: dir::GlobalSymbolId,
        definition: &dir::Definition,
    ) -> CompilerResult<Vec<dir::GlobalNodeIdAny>> {
        let Some(committed) = self.committed(template.module_id) else {
            return Ok(Vec::new());
        };
        let tree = committed.tree;

        // collect every node of each declared member initializer
        let mut collector = BodyNodeCollector {
            module: template.module_id,
            nodes: Vec::new(),
        };
        for member in definition.members() {
            // read the member declaration behind a field or an associated const
            let symbol = match member {
                dir::DefinitionMember::Field(field) => field.symbol,
                dir::DefinitionMember::AssociatedConst(constant) => constant.symbol,
                _ => continue,
            };
            let Some(declaration) = committed.bindings.get_symbol(symbol.local_id).declaration
            else {
                continue;
            };
            let Ok(declaration) = declaration.local_id.try_into_typed::<dir::Member>() else {
                continue;
            };

            // keep the members the source writes an initializer for
            let initializer = match tree.get(declaration) {
                dir::Member::Field { default, .. }
                | dir::Member::AssociatedConst { value: default, .. } => *default,
                _ => None,
            };
            let Some(initializer) = initializer else {
                continue;
            };

            dir::NodeVisitor::visit_expression(
                &mut collector,
                tree,
                initializer,
                tree.get(initializer),
            );
        }

        Ok(collector.nodes)
    }

    /// Return every node inside one callable template's body, when it has one.
    pub(super) fn template_body(
        &mut self,
        template: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<Vec<dir::GlobalNodeIdAny>>> {
        let committed =
            self.committed(template.module_id)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("an unloaded template module {:?}", template.module_id),
                })?;

        // read the body expression behind a plain or member function declaration
        let Some(declaration) = committed.bindings.get_symbol(template.local_id).declaration else {
            return Ok(None);
        };
        let tree = committed.tree;
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
            module: template.module_id,
            nodes: Vec::new(),
        };
        dir::NodeVisitor::visit_expression(&mut collector, tree, body, tree.get(body));

        Ok(Some(collector.nodes))
    }

    /// Return the concrete receiver type one instance binds `this` to.
    pub(super) fn instance_receiver(
        &mut self,
        template: dir::GlobalSymbolId,
        origin: Origin,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // give a type template itself applied to the instance arguments
        if self.definition(template)?.is_some() {
            let arguments: Vec<_> = substitution.arguments().collect();
            let receiver = if arguments.is_empty() {
                match self.committed_symbol_type(template) {
                    Some(ty) => ty,
                    None => return Ok(None),
                }
            } else {
                let arguments = self.intern_type_ids(&arguments)?;
                self.intern_type(dir::Type::Application(dir::GenericApplication {
                    symbol: template,
                    arguments,
                }))?
            };

            return Ok(Some(self.evaluate_type(origin, receiver)?));
        }

        // give a member callable its owner's target
        let Some(owner) = self.member_owner(template) else {
            return Ok(None);
        };
        let target = match self.definition(owner)?.cloned() {
            Some(dir::Definition::Extension(extension)) => extension.target.r#type(),
            _ => match self.committed_symbol_type(owner) {
                Some(ty) => ty,
                None => return Ok(None),
            },
        };
        let receiver = self.substitute_type(target, substitution)?;

        Ok(Some(self.evaluate_type(origin, receiver)?))
    }

    /// Return the definition declaring one member symbol in its loaded module.
    pub(in crate::sema) fn member_owner(
        &self,
        member: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalSymbolId> {
        let committed = self.committed(member.module_id)?;
        let (owner, _, _) = committed.definitions.member(member)?;

        Some(owner)
    }

    /// Return one template's recorded instantiations, nested closure owners included.
    pub(super) fn template_instantiations(
        &self,
        template: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::Instantiation>> {
        let committed =
            self.committed(template.module_id)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("an unloaded template module {:?}", template.module_id),
                })?;

        // keep the instantiations owned at or lexically inside the template
        let instantiations = committed
            .generics
            .iter_instantiations()
            .filter(|instantiation| {
                let Some(owner) = instantiation.owner else {
                    return false;
                };
                if owner == template {
                    return true;
                }
                if owner.module_id != template.module_id {
                    return false;
                }

                committed
                    .bindings
                    .symbol_path(owner.local_id)
                    .symbols()
                    .contains(&template.local_id)
            })
            .cloned()
            .collect();

        Ok(instantiations)
    }

    /// Return the self application type of one interface member's owner.
    pub(super) fn interface_owner_self(
        &mut self,
        member: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // serve the memo
        if let Some(base) = self.interface_owners.get(&member) {
            return Ok(*base);
        }

        // find the interface declaring this member
        let base = self
            .interface_member_owner(member)
            .and_then(|owner| self.committed_symbol_type(owner));
        self.interface_owners.insert(member, base);

        Ok(base)
    }

    /// Return the interface declaring one interface member symbol.
    pub(super) fn interface_member_owner(
        &self,
        member: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalSymbolId> {
        let committed = self.committed(member.module_id)?;
        let (owner, definition, _) = committed.definitions.member(member)?;

        matches!(definition, dir::Definition::Interface(_)).then_some(owner)
    }
}

/// Visitor collecting every node in one body subtree.
struct BodyNodeCollector {
    /// The visited module.
    module: ModuleId,
    /// Every collected node, of any kind.
    nodes: Vec<dir::GlobalNodeIdAny>,
}

impl dir::NodeVisitor for BodyNodeCollector {
    fn visit_any(&mut self, _tree: &dir::Tree, ty: dir::NodeType, id: u32) {
        self.nodes.push(dir::GlobalNodeIdAny {
            module_id: self.module,
            local_id: dir::LocalNodeIdAny::new(id, ty),
        });
    }
}
