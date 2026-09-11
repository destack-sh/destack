use destack_core::FxIndexSet;
use destack_dir as dir;

use super::instance::InstanceWorklist;
use crate::CompilerResult;
use crate::sema::{CheckState, Origin};

impl CheckState<'_> {
    /// Record what one type graph reaches: an instance per application, members per union.
    pub(in crate::sema) fn walk_type_graph(
        &mut self,
        ty: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        let mut pending = vec![ty];
        let mut visited = FxIndexSet::default();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }

            // record this type's reduction and the instance its application names
            let ty = self.ty(id)?;
            self.write_reduction(id, &ty)?;
            if let Some(reduced) = self.module.reduction(id) {
                pending.push(reduced);
            }
            if let dir::Type::Application(application) = &ty {
                self.intern_application(id, application, source, worklist)?;
            }
            // pair a bare reference to a defaulted template like an empty application
            else if let dir::Type::Reference(reference) = &ty
                && self.symbol_template(reference.symbol)?.is_some()
            {
                let application = dir::GenericApplication {
                    symbol: reference.symbol,
                    arguments: dir::TypeListId::EMPTY,
                };
                self.intern_application(id, &application, source, worklist)?;
            }

            // leave a stuck computation until instantiation reduces it
            if matches!(ty, dir::Type::Operation(_) | dir::Type::Member(_)) {
                continue;
            }

            self.for_each_type_child(id.module_id, &ty, |child| pending.push(child))?;
        }

        Ok(())
    }

    /// Record the reductions of every written head one type graph names.
    pub(super) fn walk_reduction_graph(&mut self, ty: dir::GlobalTypeId) -> CompilerResult<()> {
        let mut pending = vec![ty];
        let mut visited = FxIndexSet::default();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            let ty = self.ty(id)?;
            self.write_reduction(id, &ty)?;
            self.for_each_type_child(id.module_id, &ty, |child| pending.push(child))?;
        }

        Ok(())
    }

    /// Record the type one written head lowers through, once, where it differs from the head.
    fn write_reduction(&mut self, id: dir::GlobalTypeId, ty: &dir::Type) -> CompilerResult<()> {
        // keep a written reduction and leave open and error types as written
        let module = self.module_id;
        if id.module_id != module || self.module.reduction(id).is_some() {
            return Ok(());
        }
        let flags = self.type_flags(id)?;
        if flags.has_variable() || flags.has_error() {
            return Ok(());
        }

        // reduce set constructors, computations, and the heads naming an alias or a newtype
        let mut target = id;
        let mut is_union = matches!(ty, dir::Type::Union(_));
        match ty {
            dir::Type::Union(_)
            | dir::Type::Intersection(_)
            | dir::Type::Operation(_)
            | dir::Type::Member(_) => {}
            dir::Type::Application(dir::GenericApplication { symbol, .. })
            | dir::Type::Reference(dir::TypeReference { symbol, .. }) => {
                let (body, is_newtype) = match self.definition(*symbol)?.as_deref() {
                    Some(dir::Definition::Newtype(newtype)) => (newtype.backing, true),
                    Some(dir::Definition::TypeAlias(alias)) => (alias.value, false),
                    _ => return Ok(()),
                };
                is_union = matches!(self.ty(body)?, dir::Type::Union(_));

                // a bare alias name lowers through its symbol, an applied alias through its
                // identity unless its value computes
                let is_bare = match ty {
                    dir::Type::Application(application) => application.arguments.is_empty(),
                    _ => true,
                };
                if !is_newtype && (is_bare || !self.alias_value_computes(body, &mut Vec::new())?) {
                    return Ok(());
                }

                // a newtype keeps its identity, an applied union backing serving its members
                if is_newtype {
                    let dir::Type::Application(application) = ty else {
                        return Ok(());
                    };
                    if application.arguments.is_empty() || !is_union {
                        return Ok(());
                    }
                    let Some(body) = self.newtype_backing_body(id.module_id, application)? else {
                        return Ok(());
                    };
                    target = body;
                }
            }
            _ => return Ok(()),
        }

        // reduce the head to its normal form, a union to its canonical members in their order
        let anchor = self.module.bound.module_node.into_global(module);
        let origin = Origin::Node(anchor, None);
        let head = match is_union {
            true => target,
            false => self.normalize(origin, target)?,
        };
        let reduced = match is_union || matches!(self.ty(head)?, dir::Type::Union(_)) {
            true => match self.canonical_union_members(origin, head)? {
                Some(members) => {
                    let elements = self.intern_type_ids(&members)?;

                    self.intern_type(dir::Type::Union(dir::UnionType { elements }))?
                }
                None => return Ok(()),
            },
            false => head,
        };
        if reduced != id {
            self.module.types_tail.set_reduction(id, reduced);
        }

        Ok(())
    }

    /// Return whether one alias value computes.
    fn alias_value_computes(
        &mut self,
        value: dir::GlobalTypeId,
        visited: &mut Vec<dir::GlobalSymbolId>,
    ) -> CompilerResult<bool> {
        let symbol = match self.ty(value)? {
            dir::Type::Union(_)
            | dir::Type::Intersection(_)
            | dir::Type::Operation(_)
            | dir::Type::Member(_) => return Ok(true),
            dir::Type::Application(application) => application.symbol,
            dir::Type::Reference(reference) => reference.symbol,
            _ => return Ok(false),
        };
        if visited.contains(&symbol) {
            return Ok(false);
        }
        visited.push(symbol);
        match self.definition(symbol)?.as_deref() {
            Some(dir::Definition::TypeAlias(alias)) => {
                let value = alias.value;

                self.alias_value_computes(value, visited)
            }
            _ => Ok(false),
        }
    }

    /// Intern one written application as an instance through its canonical pairing.
    fn intern_application(
        &mut self,
        id: dir::GlobalTypeId,
        application: &dir::GenericApplication,
        source: dir::GlobalNodeIdAny,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<Option<dir::LocalInstanceId>> {
        // read the instance this application already closed to
        if let Some(instance) = self.module.generics_tail.application_instance(id) {
            return Ok(Some(instance));
        }
        // keep the instance an intrinsic alias's representation reads
        if self.is_transparent_alias(application.symbol)?
            && !self.is_intrinsic_alias(application.symbol)?
        {
            return Ok(None);
        }
        // skip an application that pairs with no instance
        let Ok(substitution) = self.instance_substitution(id.module_id, application) else {
            return Ok(None);
        };

        // intern the instance the application pairs with
        let instance = self.intern_instance(
            application.symbol,
            None,
            substitution.bindings.to_vec(),
            source,
            dir::InstanceOrigin::Application,
            worklist,
        )?;

        // record the instance this closed application selects
        if let Some(instance) = instance {
            self.module
                .generics_tail
                .bind_application_instance(id, instance);
        }

        Ok(instance)
    }

    /// Rewrite one instance argument with every region bound to its positional literal, the
    /// instance key erasing the regions it was reached with.
    pub(super) fn erase_argument_regions(
        &mut self,
        id: dir::GlobalTypeId,
        visiting: &mut Vec<dir::GlobalTypeId>,
        next_position: &mut u32,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // keep a revisited cycle member as written
        if visiting.contains(&id) {
            return Ok(id);
        }

        // bind a region term to the next position
        if self.memory_kind(id)? == Some(dir::MemoryParameter::Region) {
            let position = *next_position;
            *next_position += 1;

            return self.lifetime_literal(dir::Lifetime::Bound(position));
        }

        // rebuild the children with their lifetimes bound
        visiting.push(id);
        let ty = self.ty(id)?;
        let rebuilt = self.map_type_children(id.module_id, ty, &mut |state, child| {
            state.erase_argument_regions(child, visiting, next_position)
        })?;
        visiting.pop();

        self.intern_type(rebuilt)
    }

    /// Ground the induced place and space parameters one argument carries at local.
    pub(super) fn ground_induced_memory_argument(
        &mut self,
        argument: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // collect the induced place terms the argument reaches
        let mut induced = Vec::new();
        let mut pending = vec![argument];
        let mut visited = FxIndexSet::default();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }

            // keep each parameter its binding induces a place or space for
            let kind = self.ty(id)?;
            if let dir::Type::Parameter(parameter) = &kind
                && self.generic_parameter(*parameter)?.is_some_and(|binding| {
                    matches!(
                        binding.induced_memory_parameter(),
                        Some(dir::MemoryParameter::Place | dir::MemoryParameter::Space)
                    )
                })
            {
                induced.push(id);
            }

            self.for_each_type_child(id.module_id, &kind, |child| pending.push(child))?;
        }

        // replace each induced place term with the ambient local space
        let mut argument = argument;
        for from in induced {
            let to = self.local_place()?;
            argument = self.replace_type(argument, from, to)?;
        }

        Ok(argument)
    }
}
