use std::iter;

use destack_dir as dir;
use smallvec::SmallVec;

use super::instance::InstanceWorklist;
use crate::sema::{CheckState, Origin, TypeSubstitution, Verdict};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return the bounds one parameter assumes, their elided arguments filled.
    pub(in crate::sema) fn filled_parameter_bounds(
        &mut self,
        origin: Origin,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let mut bounds = Vec::new();
        for declared in self.declared_parameter_bounds(parameter)? {
            self.fill_bound(origin, declared, &mut bounds)?;
        }

        Ok(bounds)
    }

    /// Append the bounds one declared bound fills to, each type once.
    fn fill_bound(
        &mut self,
        origin: Origin,
        declared: dir::GlobalTypeId,
        bounds: &mut Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        // keep a memory domain as its own bound
        let declared = self.normalize(origin, declared)?;
        if self.memory_kind(declared)?.is_some() || self.is_memory_literal(declared)? {
            return self.push_bound(declared, bounds);
        }

        // fill the elided arguments of each interface bound
        for bound in self.interface_bounds(origin, declared)? {
            let filled = match self.ty(bound)? {
                dir::Type::Application(application) => {
                    self.fill_elided_application(bound.module_id, &application)?
                }
                _ => None,
            };
            self.push_bound(filled.unwrap_or(bound), bounds)?;
        }

        // bound a copying union by Copy
        if matches!(self.ty(declared)?, dir::Type::Union(_))
            && self.decide_copy(origin, declared, &mut SmallVec::new())? == Verdict::Holds
        {
            let copy = self.language_type(dir::LanguageItem::Copy, &[])?;
            self.push_bound(copy, bounds)?;
        }

        Ok(())
    }

    /// Append one bound unless the list already holds the same type.
    fn push_bound(
        &self,
        bound: dir::GlobalTypeId,
        bounds: &mut Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        for known in bounds.iter() {
            if self.is_same_type(*known, bound)? {
                return Ok(());
            }
        }
        bounds.push(bound);

        Ok(())
    }

    /// Return whether one type is a memory literal domain a parameter may be bounded by.
    fn is_memory_literal(&self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        Ok(match self.ty(ty)? {
            dir::Type::Literal(dir::Literal::String(value)) => {
                dir::Access::from_text(self.strings().get(value)).is_some()
            }
            _ => false,
        })
    }

    /// Record the filled bounds of every own template parameter.
    pub(in crate::sema) fn fill_parameter_bounds(&mut self) -> CompilerResult<()> {
        let module = self.module_id;
        let parameters: Vec<_> = self
            .module
            .generics
            .with_tail(&self.module.generics_tail)
            .iter_parameters()
            .filter(|(_, binding)| binding.is_instance_parameter())
            .map(|(parameter, binding)| (parameter.into_global(module), binding.source))
            .collect();
        for (parameter, source) in parameters {
            let origin = self.anchored_origin(source)?;
            let bounds = self.filled_parameter_bounds(origin, parameter)?;
            if bounds.is_empty() {
                continue;
            }
            let list = self.intern_type_ids(&bounds)?;
            self.module
                .generics_tail
                .set_parameter_bounds(parameter.local_id, list);
        }

        self.fill_assumed_bounds()
    }

    /// Record the filled where-clause bounds every template assumes for its parameters.
    fn fill_assumed_bounds(&mut self) -> CompilerResult<()> {
        let module = self.module_id;
        let templates: Vec<_> = self
            .module
            .generics
            .with_tail(&self.module.generics_tail)
            .iter_templates()
            .map(|(template, declared)| (template, declared.source, declared.predicates.clone()))
            .collect();
        for (template, source, predicates) in templates {
            let origin = self.anchored_origin(source)?;
            let mut assumed: Vec<(dir::GlobalGenericParameterId, Vec<dir::GlobalTypeId>)> =
                Vec::new();
            for predicate in predicates {
                if predicate.relation != dir::WhereRelation::Satisfies {
                    continue;
                }
                let dir::Type::Parameter(parameter) = self.ty(predicate.left)? else {
                    continue;
                };

                // append the filled bound to its parameter's list
                let index = match assumed
                    .iter()
                    .position(|(subject, _)| *subject == parameter)
                {
                    Some(index) => index,
                    None => {
                        assumed.push((parameter, Vec::new()));
                        assumed.len() - 1
                    }
                };
                self.fill_bound(origin, predicate.right, &mut assumed[index].1)?;
            }
            for (parameter, bounds) in assumed {
                if bounds.is_empty() || parameter.module_id != module {
                    continue;
                }
                let list = self.intern_type_ids(&bounds)?;
                self.module
                    .generics_tail
                    .set_assumed_bounds(template, parameter.local_id, list);
            }
        }

        Ok(())
    }

    /// Record the witnesses the bounds of one instance's arguments reach.
    pub(super) fn walk_bound_witnesses(
        &mut self,
        key: &dir::InstanceKey,
        source: dir::GlobalNodeIdAny,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        let origin = self.anchored_origin(source)?;
        for binding in &key.arguments {
            // a bound reads `this` as the bounded argument itself
            let substitution = TypeSubstitution {
                bindings: key.arguments.iter().copied().collect(),
                receiver: Some(binding.argument),
            };
            for declared in self.filled_parameter_bounds(origin, binding.parameter)? {
                let bound = self.substitute_type(declared, &substitution)?;
                for interface in self.interface_bounds(origin, bound)? {
                    self.write_witness(binding.argument, interface, source, worklist)?;
                }
            }
        }

        Ok(())
    }

    /// Return the interface applications one bound names through intersections and unions.
    fn interface_bounds(
        &mut self,
        origin: Origin,
        bound: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        Ok(match self.ty(bound)? {
            dir::Type::Application(application)
                if self.symbol_kind(application.symbol)?.is_interface() =>
            {
                vec![bound]
            }
            dir::Type::Reference(reference)
                if self.symbol_kind(reference.symbol)?.is_interface() =>
            {
                let application = dir::GenericApplication {
                    symbol: reference.symbol,
                    arguments: dir::TypeListId::EMPTY,
                };

                vec![self.intern_type(dir::Type::Application(application))?]
            }
            dir::Type::Intersection(intersection) => {
                let elements = self
                    .type_ids(bound.module_id, intersection.elements)?
                    .to_vec();
                let mut interfaces = Vec::new();
                for element in elements {
                    interfaces.extend(self.interface_bounds(origin, element)?);
                }

                interfaces
            }
            // keep the interface applications every union arm reaches
            dir::Type::Union(union) => {
                let elements = self.type_ids(bound.module_id, union.elements)?.to_vec();
                let mut shared: Option<Vec<(dir::GlobalSymbolId, dir::GlobalTypeId)>> = None;
                for element in elements {
                    let reached = self.reached_interfaces(origin, element)?;
                    shared = Some(match shared {
                        None => reached,
                        Some(shared) => {
                            let mut kept = Vec::with_capacity(shared.len());
                            for (symbol, application) in shared {
                                let mut is_shared = false;
                                for (_, other) in &reached {
                                    if self.is_same_type(application, *other)? {
                                        is_shared = true;
                                        break;
                                    }
                                }
                                if is_shared {
                                    kept.push((symbol, application));
                                }
                            }

                            kept
                        }
                    });
                }
                let shared = shared.ok_or_else(|| CompilerError::Internal {
                    message: "a union bound without arms".to_string(),
                })?;

                shared.into_iter().map(|(_, interface)| interface).collect()
            }
            _ => Vec::new(),
        })
    }

    /// Return the interfaces one type reaches by declaration: those it names, then their ancestors.
    fn reached_interfaces(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<(dir::GlobalSymbolId, dir::GlobalTypeId)>> {
        let mut reached = Vec::new();
        for interface in self.interface_bounds(origin, ty)? {
            // add the named interface, then each inherited interface new to the list
            let applications = self.heritage_closure(origin, interface)?.applications;
            let inherited = applications.into_iter().map(|application| application.ty);
            for application in iter::once(interface).chain(inherited) {
                let (_, instance) =
                    self.nominal_application_maybe(application)?
                        .ok_or_else(|| CompilerError::Internal {
                            message: "an interface heritage application without a nominal head"
                                .to_string(),
                        })?;
                let is_new = !reached.iter().any(|(symbol, _)| *symbol == instance.symbol);
                if is_new && self.symbol_kind(instance.symbol)?.is_interface() {
                    reached.push((instance.symbol, application));
                }
            }
        }

        Ok(reached)
    }
}
