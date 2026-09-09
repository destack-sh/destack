use destack_dir as dir;

use super::instance::InstanceWorklist;
use crate::CompilerResult;
use crate::sema::{CheckState, Origin, TypeSubstitution};

impl CheckState<'_> {
    /// Return the bounds one parameter assumes, their elided arguments filled.
    pub(in crate::sema) fn filled_parameter_bounds(
        &mut self,
        origin: Origin,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let mut bounds = Vec::new();
        for declared in self.declared_parameter_bounds(parameter)? {
            bounds.extend(self.filled_bound(origin, declared)?);
        }

        Ok(bounds)
    }

    /// Return one declared bound, its elided arguments filled.
    fn filled_bound(
        &mut self,
        origin: Origin,
        declared: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let mut bounds = Vec::new();
        let declared = self.normalize(origin, declared)?;
        for bound in self.interface_bounds(declared)? {
            let filled = match self.ty(bound)? {
                dir::Type::Application(application) => {
                    self.fill_elided_application(bound.module_id, &application)?
                }
                _ => None,
            };
            bounds.push(filled.unwrap_or(bound));
        }
        if self.memory_kind(declared)?.is_some() || self.is_memory_literal(declared)? {
            bounds.push(declared);
        }

        Ok(bounds)
    }

    /// Return whether one type is a memory literal domain a parameter may be bounded by.
    fn is_memory_literal(&self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        Ok(match self.ty(ty)? {
            dir::Type::Literal(dir::Literal::String(value)) => {
                dir::Space::from_text(value).is_some() || dir::Access::from_text(value).is_some()
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
                let filled = self.filled_bound(origin, predicate.right)?;
                match assumed
                    .iter_mut()
                    .find(|(subject, _)| *subject == parameter)
                {
                    Some((_, bounds)) => bounds.extend(filled),
                    None => assumed.push((parameter, filled)),
                }
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
                for interface in self.interface_bounds(bound)? {
                    self.write_witness(binding.argument, interface, source, worklist)?;
                }
            }
        }

        Ok(())
    }

    /// Return the interface applications one bound names, itself or each intersection operand.
    fn interface_bounds(
        &mut self,
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
                let elements = self.type_ids(bound.module_id, intersection.elements)?;
                let mut interfaces = Vec::new();
                for element in elements {
                    interfaces.extend(self.interface_bounds(*element)?);
                }

                interfaces
            }
            _ => Vec::new(),
        })
    }
}
