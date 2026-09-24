use destack_dir as dir;
use destack_source::ModuleId;

use crate::sema::{CheckState, TypeSubstitution};
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

        // materialize each body under its receiver
        for (owner, member) in members {
            // read the receiver the owner declares
            let receiver = match self.definition(owner)?.as_deref() {
                Some(dir::Definition::Extension(extension)) => Some(extension.target.r#type()),
                Some(dir::Definition::Interface(_)) => None,
                _ => {
                    let application = self.declaration_instance(owner)?;

                    Some(self.intern_type(dir::Type::Application(application))?)
                }
            };
            let substitution = TypeSubstitution {
                receiver,
                ..TypeSubstitution::default()
            };
            let materialization = Materialization {
                substitution: Some(&substitution),
                anchor: None,
            };
            self.materialize_template_entries(&materialization, member, worklist)?;
        }

        Ok(())
    }

    /// Return every node inside the member initializers one type template declares.
    pub(super) fn template_initializer_body(
        &self,
        template: dir::GlobalSymbolId,
        definition: &dir::Definition,
    ) -> CompilerResult<Vec<dir::GlobalNodeIdAny>> {
        let Some(committed) = self.committed(template.module_id)? else {
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

    /// Return the body expression behind one plain or member function template, when it has one.
    pub(super) fn template_body_expression(
        &mut self,
        template: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<(&dir::Tree, dir::LocalNodeId<dir::Expression>)>> {
        let committed =
            self.committed(template.module_id)?
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("an unloaded template module {:?}", template.module_id),
                })?;
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

        Ok(body.map(|body| (tree, body)))
    }

    /// Return every node inside one callable template's body, when it has one.
    pub(super) fn template_body(
        &mut self,
        template: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<Vec<dir::GlobalNodeIdAny>>> {
        let Some((tree, body)) = self.template_body_expression(template)? else {
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

    /// Return the interface declaring one interface member symbol.
    pub(in crate::sema) fn interface_member_owner(
        &self,
        member: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        Ok(self
            .committed(member.module_id)?
            .and_then(|committed| committed.definitions.member(member))
            .and_then(|(owner, definition, _)| {
                matches!(definition, dir::Definition::Interface(_)).then_some(owner)
            }))
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
