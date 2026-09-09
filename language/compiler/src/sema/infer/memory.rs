use destack_dir as dir;

use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{CheckState, FlowSite, PlaceUse, Value};

impl CheckState<'_> {
    /// Infer one borrow expression from its borrowed value and lifetime.
    pub(in crate::sema) fn infer_borrow_expression(
        &mut self,
        site: FlowSite,
        mutability: Option<dir::Mutability>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // read the borrowed value and the place it names
        let node = site.node.into_typed::<dir::Expression>();
        let right_site = self.visit_site(right.into_global_any(node.module_id))?;
        let ty = self.infer_node_type(right_site, PlaceUse::Read)?;
        let value = self.expression_value(right_site, ty)?;
        let place = self.value_place(right_site.origin(), value)?;

        // borrow an inline Copy payload narrowed outside frame storage as a readonly frame copy,
        // which a case change through an alias never reaches
        let place = match self.is_copied_payload(right_site, &value, &place)? {
            true => {
                let mut copy = self.root_place(
                    right_site.origin(),
                    node.module_id,
                    value.ty,
                    dir::Space::Local,
                    dir::Lifetime::Frame,
                )?;
                copy.access = self.access_literal(dir::Access::Readonly)?;

                copy
            }
            false => place,
        };

        // derive borrow form parameters from the place and written mutability
        let lifetime = place.lifetime;
        let requested = match mutability {
            Some(mutability) => mutability.access(),
            None => dir::Access::Mutable,
        };
        let access = self.access_literal(requested)?;

        // require the requested access from the selected place
        let origin = site.origin();
        let is_granted = self
            .constrain_access_assignable(origin, place.access, access)?
            .holds();

        // report a place that withholds the requested access
        if !is_granted {
            let granted = self.access_of(place.access)?;
            self.report_borrow_access_not_granted(origin, requested, granted, value.ty)?;
        }

        // commit the access this borrow requires from the lent place
        if is_granted && let Some(source) = value.node {
            let is_aliased = self.type_is_aliased(origin, value.ty)?;
            self.commit_required_access(source, requested, is_aliased);
        }

        // build the borrow form from the lent place's lifetime and placement
        let placement = self.shallow_resolve(place.placement)?;
        let region = self.intern_region(lifetime, placement)?;
        let form = self.intern_borrow(region, access)?;
        let borrowed = self.intern_type(dir::Type::Form(dir::FormType {
            form,
            value: value.ty,
        }))?;
        let borrowed = self.normalize(origin, borrowed)?;

        self.commit_node_type(node.into_any(), borrowed)?;

        Ok(())
    }

    /// Return whether one borrowed value is an inline Copy union payload read outside frame
    /// storage: narrowed, neither a handle nor a reference, and copyable.
    fn is_copied_payload(
        &mut self,
        site: FlowSite,
        value: &Value,
        place: &dir::PlaceResolution,
    ) -> CompilerResult<bool> {
        let origin = site.origin();
        if self
            .decisions(site.node.module_id)
            .narrowing(site.node)
            .is_none()
        {
            return Ok(false);
        }
        let frame = self.lifetime_literal(dir::Lifetime::Frame)?;
        if self.shallow_resolve(place.lifetime)? == frame {
            return Ok(false);
        }
        if self.type_is_aliased(origin, value.ty)?
            || self
                .form_chain(origin, value.ty)?
                .ownership_form()
                .is_some()
        {
            return Ok(false);
        }

        Ok(self
            .decide_copy(origin, value.ty, &mut SmallVec::new())?
            .holds())
    }

    /// Commit the binding uses one borrow of a binding value requires from it.
    pub(in crate::sema) fn commit_required_access(
        &mut self,
        node: dir::GlobalNodeIdAny,
        requested: dir::Access,
        is_aliased: bool,
    ) {
        // commit the mutable access, mutating the binding storage when it holds the value itself
        if requested == dir::Access::Readonly {
            return;
        }
        self.commit_access_use(node, dir::BindingUse::MUTABLE);
        if !is_aliased {
            self.commit_access_use(node, dir::BindingUse::MUTATE);
        }
    }
}
