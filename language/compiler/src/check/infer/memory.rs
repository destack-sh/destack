use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, BodyState, FlowSite, Origin, PlaceUse, answer};

impl BodyState<'_, '_> {
    /// Materialize one fresh value in its contextual place.
    pub(in crate::check) fn materialize_fresh_value(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        target: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        // take one common explicit contextual place, leaving bare values bare
        let place = match target {
            Some(target) => answer!(self.contextual_place(origin, target)?),
            None => None,
        };
        let Some(place) = place else {
            return Ok(Answer::Ready(value));
        };
        let value = self.check.resolve_relative_place(origin, value, place)?;

        Ok(Answer::Ready(value))
    }

    /// Return the common concrete place offered by one contextual type.
    fn contextual_place(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let target = self.check.settled_root(target)?;
        let scope = match self.check.type_flags(target)?.has_parameter() {
            true => self.check.origin_scope(origin)?,
            false => None,
        };
        if let Some(place) = self.check.contextual_places.get(&(target, scope)) {
            return Ok(Answer::Ready(*place));
        }

        let answer = self.compute_contextual_place(origin, target)?;
        if let Answer::Ready(place) = answer
            && self.check.type_variables(target)?.is_empty()
        {
            self.check.contextual_places.insert((target, scope), place);
        }

        Ok(answer)
    }

    /// Compute the common concrete place offered by one contextual type.
    fn compute_contextual_place(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Answer::Ready(target) = self.check.reduce_type_head(origin, target)? else {
            return Ok(Answer::Ready(None));
        };
        let chain = self.check.form_chain(origin, target)?;
        if let Some(place) = chain.place() {
            return Ok(Answer::Ready(Some(place)));
        }

        // optional and other unions contribute the place of located members
        let base = chain.base();
        let dir::Type::Union(union) = self.ty(base)? else {
            return Ok(Answer::Ready(None));
        };
        let elements = self.type_ids(base.module_id, union.elements)?.to_vec();
        let mut common = None;
        for element in elements {
            let Some(place) = answer!(self.contextual_place(origin, element)?) else {
                continue;
            };
            match common {
                None => common = Some(place),
                Some(current) if current == place => {}
                Some(_) => return Ok(Answer::Ready(None)),
            }
        }

        Ok(Answer::Ready(common))
    }

    /// Infer one move expression from its moved value.
    pub(in crate::check) fn infer_move_expression(
        &mut self,
        site: FlowSite,
        mutability: Option<dir::Mutability>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let right_site = self.node_site(right.into_global_any(module))?;
        let value = answer!(self.infer_node_type(right_site, PlaceUse::Read)?);

        // preserve immutable move syntax as a readonly owned value
        let value = if mutability == Some(dir::Mutability::Immutable) {
            self.intern_type(
                module,
                dir::Type::Form(dir::FormType {
                    form: dir::Form::Readonly,
                    value,
                }),
            )?
        } else {
            value
        };

        // wrap the moved value in owned form
        let owned = self.intern_type(
            module,
            dir::Type::Form(dir::FormType {
                form: dir::Form::Owned,
                value,
            }),
        )?;
        self.commit_node_type(node.into_any(), owned)?;

        Ok(Answer::Ready(()))
    }

    /// Infer one borrow expression from its borrowed value and lifetime.
    pub(in crate::check) fn infer_borrow_expression(
        &mut self,
        site: FlowSite,
        mutability: Option<dir::Mutability>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let right_site = self.node_site(right.into_global_any(node.module_id))?;
        let value = answer!(self.infer_node_type(right_site, PlaceUse::Read)?);
        let Some(_) = answer!(self.select_assign_place(right_site, right, PlaceUse::Read)?) else {
            return Ok(Answer::Ready(()));
        };

        // derive borrow form parameters from the place and written mutability
        let expression = right.into_global(node.module_id);
        let lifetime = answer!(self.expression_lifetime(expression, value)?);
        let requested = match mutability {
            Some(mutability) => mutability.access(),
            None => dir::Access::Mutable,
        };
        let access = self.intern_type(
            node.module_id,
            dir::Type::Memory(dir::MemoryLiteral::Access(requested)),
        )?;

        // the borrow site grants only what the source's form allows
        let origin = site.origin();
        let chain = self.check.form_chain(origin, value)?;
        if !chain.is_open() {
            let ownership = match self.check.form_ownership(origin, &chain)? {
                Answer::Ready(ownership) => ownership,
                Answer::Pending(_) => None,
            };
            let granted = if chain.is_readonly() {
                requested == dir::Access::Readonly
            } else if ownership == Some(dir::Ownership::Managed) {
                self.check
                    .managed_acquisition_granted(Some(requested), chain.place())?
            } else {
                true
            };
            if !granted {
                // the two ungranted cases cap at readonly and mutable
                let ceiling = match chain.is_readonly() {
                    true => dir::Access::Readonly,
                    false => dir::Access::Mutable,
                };
                self.check
                    .report_borrow_access_not_granted(origin, requested, ceiling, value)?;
            }
        }

        // wrap the borrowed value and reduce redundant memory forms
        let form = self.intern_borrow(node.module_id, lifetime, access)?;
        let borrowed = self.intern_type(
            node.module_id,
            dir::Type::Form(dir::FormType { form, value }),
        )?;
        let borrowed = answer!(self.reduce_type_head(site.origin(), borrowed)?);
        self.commit_node_type(node.into_any(), borrowed)?;

        Ok(Answer::Ready(()))
    }
}
