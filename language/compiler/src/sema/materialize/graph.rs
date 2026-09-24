use destack_core::FxIndexSet;
use destack_dir as dir;
use smallvec::SmallVec;

use super::instance::InstanceWorklist;
use crate::sema::{CheckState, Origin};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Record the reductions, representations, and instances one type graph names.
    pub(in crate::sema) fn walk_type_graph(
        &mut self,
        ty: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        let mut pending = vec![ty];
        while let Some(id) = pending.pop() {
            if !worklist.walked.insert(id) {
                continue;
            }

            // record this type's reduction, representation and the instance its application names
            let ty = self.ty(id)?;
            self.write_reduction(id, &ty)?;
            self.write_representation(id, source)?;
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

    /// Record the reduction and representation of every head one type graph names.
    pub(super) fn walk_reduction_graph(
        &mut self,
        ty: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        let mut pending = vec![ty];
        let mut visited = FxIndexSet::default();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            let ty = self.ty(id)?;
            self.write_reduction(id, &ty)?;
            self.write_representation(id, source)?;
            self.for_each_type_child(id.module_id, &ty, |child| pending.push(child))?;
        }

        Ok(())
    }

    /// Record whether one type of this module copies and the ownership its head defaults to.
    fn write_representation(
        &mut self,
        id: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        if id.module_id != self.module_id {
            return Ok(());
        }
        let origin = self.anchored_origin(source)?;
        self.decide_copy(origin, id, &mut SmallVec::new())?;
        let ownership = self.ownership(id)?;
        self.module
            .representations_tail
            .set_ownership(id, ownership.into());

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

                // lower a bare alias through its symbol, an applied alias through its identity
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
        // pair a bare generic head with no instance while a parameter lacks its default
        if application.arguments.is_empty()
            && let Some(template) = self.symbol_template(application.symbol)?
        {
            for parameter in self.generic_template_parameters(template)? {
                let binding =
                    self.generic_parameter(parameter)?
                        .ok_or_else(|| CompilerError::Internal {
                            message: format!(
                                "a template parameter {parameter:?} without its binding"
                            ),
                        })?;
                if binding.is_writable()
                    && binding.default.is_none()
                    && binding.memory_parameter() != Some(dir::MemoryParameter::Region)
                {
                    return Ok(None);
                }
            }
        }
        let substitution = self.instance_substitution(id.module_id, application)?;

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

    /// Rewrite one instance argument with every region bound to its positional literal.
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

        // bind a region term's extent to the next position, a union member by member
        if self.memory_kind(id)? == Some(dir::MemoryParameter::Region)
            && !matches!(self.ty(id)?, dir::Type::Union(_))
        {
            // keep a declared region parameter, bound early by its declaration
            if let dir::Type::Parameter(parameter) = self.ty(id)?
                && self.generic_parameter(parameter)?.is_some_and(|binding| {
                    binding.kind == dir::GenericParameterKind::Memory(dir::MemoryParameter::Region)
                })
            {
                return Ok(id);
            }

            // bind the next position to each region argument
            let position = *next_position;
            *next_position += 1;
            let extent = self.lifetime_literal(dir::Lifetime::Bound(position))?;

            return match self.ty(id)? {
                dir::Type::Region(pair) => self.intern_type(dir::Type::Region(dir::RegionType {
                    extent,
                    space: pair.space,
                })),
                _ if matches!(self.lifetime_of(id)?, Some(dir::Lifetime::Bound(_))) => Ok(extent),
                _ => Ok(id),
            };
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
}
