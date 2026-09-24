use destack_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, FlowSite, PlaceUse};

impl CheckState<'_> {
    /// Infer one borrow expression from its borrowed value and lifetime.
    pub(in crate::sema) fn infer_borrow_expression(
        &mut self,
        site: FlowSite,
        access: Option<dir::Access>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // read the borrowed value and the place it names
        let node = site.node.into_typed::<dir::Expression>();
        let right_site = self.visit_site(right.into_global_any(node.module_id))?;
        let ty = self.infer_node_type(right_site, PlaceUse::Read)?;
        let value = self.expression_value(right_site, ty)?;
        let requested = access.unwrap_or(dir::Access::BARE);
        let origin = site.origin();

        // borrow a narrowed inline payload at the rung excluding case changes
        let is_narrowed = self
            .decisions(right_site.node.module_id)
            .narrowing(right_site.node)
            .is_some();
        let chain = self.form_chain(origin, value.ty)?;
        let is_managed = self.form_ownership(origin, &chain)? == Some(dir::Ownership::Managed);
        let required = match is_narrowed && !is_managed {
            true => requested.join(dir::Access::Immutable),
            false => requested,
        };
        let place = self.borrowed_place(origin, value, Some(required), false)?;

        // derive borrow form parameters from the place and the annotated access
        let lifetime = place.lifetime;
        let access = self.access_literal(requested)?;

        // require the required access from the selected place
        let required_type = self.access_literal(required)?;
        let is_granted = self
            .constrain_access_assignable(origin, place.access, required_type)?
            .holds();

        // report a place that withholds the required access
        if !is_granted {
            let granted = self.access_of(place.access)?;
            self.report_borrow_access_not_granted(origin, required, granted, value.ty)?;
        }

        // commit the access this borrow requires from the lent place
        if is_granted && let Some(source) = value.node {
            let is_aliased = self.type_is_aliased(origin, value.ty)?;
            self.commit_required_access(source, requested, is_aliased);
        }

        // build the borrow form from the lent place's lifetime and placement
        let placement = self.shallow_resolve(place.placement)?;
        let region = self.intern_region(lifetime, placement)?;
        let borrowed = self.borrow_value(region, access, value.ty)?;
        let borrowed = self.normalize(origin, borrowed)?;

        self.commit_node_type(node.into_any(), borrowed)?;

        Ok(())
    }

    /// Commit the binding uses one borrow of a binding value requires from it.
    pub(in crate::sema) fn commit_required_access(
        &mut self,
        node: dir::GlobalNodeIdAny,
        requested: dir::Access,
        is_aliased: bool,
    ) {
        // retain exclusion requirements independently of mutation
        if requested.excludes() {
            self.commit_access_use(node, dir::BindingUse::EXCLUSIVE);
        }

        // stop at a request without writes
        if !requested.writes() {
            return;
        }

        // record writes to the binding when it stores the borrowed value directly
        self.commit_access_use(node, dir::BindingUse::MUTABLE);
        if !is_aliased {
            self.commit_access_use(node, dir::BindingUse::MUTATE);
        }
    }
}
