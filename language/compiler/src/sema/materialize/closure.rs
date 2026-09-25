use tspp_core::FxIndexMap;
use tspp_dir as dir;

use crate::sema::{CheckState, TypeSubstitution};
use crate::{CompilerError, CompilerResult};

use super::entry::{Entry, Materialization};
use super::instance::InstanceWorklist;

/// The depth an instantiation chain may reach before it reads as polymorphic recursion.
pub(in crate::sema) const INSTANCE_DEPTH_LIMIT: u32 = 128;

impl CheckState<'_> {
    /// Materialize the closure of instances.
    pub(in crate::sema) fn materialize_instance_closure(
        &mut self,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        // walk the worklist until it drains
        let mut next = 0;
        while let Some((key, instance)) = worklist.instance_at(next) {
            worklist.reaching = Some(next);
            next += 1;
            let Some(source) = self
                .module
                .generics_tail
                .get_local_instance(instance)
                .map(|instance| instance.source)
            else {
                continue;
            };
            self.materialize_instance(&key, source, worklist)?;
        }
        worklist.reaching = None;

        Ok(())
    }

    /// Walk one instance's signature and body under its substitution.
    fn materialize_instance(
        &mut self,
        key: &dir::InstanceKey,
        source: dir::GlobalNodeIdAny,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        let substitution = TypeSubstitution {
            bindings: key.arguments.iter().copied().collect(),
            receiver: key.receiver,
        };
        let materialization = Materialization {
            substitution: Some(&substitution),
            anchor: Some(source),
        };

        // materialize the types the signature and body hold
        let entries = self.template_entries(key.symbol)?;
        self.counters.instances += 1;
        self.counters.instance_entries += entries.len() as u64;
        for entry in entries {
            match entry {
                // the body's types close at instantiation, the instantiate pass substituting them
                Entry::Symbol(..) | Entry::Node(..) => {}
                Entry::Definition(_, definition) => {
                    self.materialize_payload(&materialization, source, *definition, worklist)?;
                }
                // reach the instances a body decision selects
                Entry::Decision(_, decision) => {
                    self.intern_selected_instances(&materialization, source, &decision, worklist)?;
                }
                Entry::Place(_, place) => {
                    self.materialize_payload(&materialization, source, place, worklist)?;
                }
                Entry::Coercion(_, coercion) => {
                    self.materialize_payload(&materialization, source, coercion, worklist)?;
                }
            }
        }

        // close the instantiations the body performs at the substitution
        let Some(nodes) = self.template_body(key.symbol)? else {
            return Ok(());
        };
        let mut instantiations = Vec::new();
        for node in nodes {
            instantiations.extend(self.instantiations_at(node)?);
        }
        for instantiation in instantiations {
            let receiver = instantiation
                .key
                .receiver
                .map(|receiver| self.substitute_type(receiver, &substitution))
                .transpose()?;
            let mut bindings = Vec::with_capacity(instantiation.key.arguments.len());
            for binding in &instantiation.key.arguments {
                let argument = self.substitute_type(binding.argument, &substitution)?;
                bindings.push(dir::GenericArgumentBinding::new(
                    binding.parameter,
                    argument,
                ));
            }
            self.intern_instance(
                instantiation.key.symbol,
                receiver,
                bindings,
                source,
                dir::InstanceOrigin::Instantiation,
                worklist,
            )?;
        }
        Ok(())
    }

    /// Return the instantiations one module recorded at a node, the module indexed once.
    fn instantiations_at(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Vec<dir::Instantiation>> {
        let module = node.module_id;
        if !self.instantiations.contains_key(&module) {
            let Some(committed) = self.committed(module)? else {
                return Err(CompilerError::Internal {
                    message: "a template body outside a loaded module".to_string(),
                });
            };
            let mut index: FxIndexMap<dir::GlobalNodeIdAny, Vec<dir::Instantiation>> =
                FxIndexMap::default();
            for instantiation in committed.generics.iter_instantiations() {
                index
                    .entry(instantiation.source)
                    .or_default()
                    .push(instantiation.clone());
            }
            self.instantiations.insert(module, index);
        }

        Ok(self.instantiations[&module]
            .get(&node)
            .cloned()
            .unwrap_or_default())
    }

    /// Intern the instances one body decision selects under the substitution.
    fn intern_selected_instances(
        &mut self,
        materialization: &Materialization<'_>,
        source: dir::GlobalNodeIdAny,
        decision: &dir::Decision,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        let mut written = Vec::new();
        dir::InstanceKeyVisit::visit_instance_keys(decision, &mut |key| {
            if !key.arguments.is_empty() || key.receiver.is_some() {
                written.push(key.clone());
            }
        });
        for key in written {
            let receiver = key
                .receiver
                .map(|receiver| self.materialize_type(materialization, source, receiver, worklist))
                .transpose()?;
            let mut arguments = Vec::with_capacity(key.arguments.len());
            for binding in &key.arguments {
                let argument =
                    self.materialize_type(materialization, source, binding.argument, worklist)?;
                arguments.push(dir::GenericArgumentBinding::new(
                    binding.parameter,
                    argument,
                ));
            }
            self.intern_instance(
                key.symbol,
                receiver,
                arguments,
                source,
                dir::InstanceOrigin::Instantiation,
                worklist,
            )?;
        }

        Ok(())
    }
}
