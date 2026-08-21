use destack_dir as dir;

use crate::CompilerResult;
use crate::sema::{BodyState, FlowSite, PlaceUse};

impl BodyState<'_, '_> {
    /// Infer one borrow expression from its borrowed value and lifetime.
    pub(in crate::sema) fn infer_borrow_expression(
        &mut self,
        site: FlowSite,
        mutability: Option<dir::Mutability>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let node = site.node.into_typed::<dir::Expression>();
        let right_site = self.visit_site(right.into_global_any(node.module_id))?;
        let ty = self.infer_node_type(right_site, PlaceUse::Read)?;
        let value = self.expression_value(right_site, ty)?;
        let place = self.value_place(right_site.origin(), value)?;

        // derive borrow form parameters from the place and written mutability
        let lifetime = place.lifetime;
        let requested = match mutability {
            Some(mutability) => mutability.access(),
            None => dir::Access::Mutable,
        };
        let access = self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Access(requested)))?;

        // require the requested access from the selected place
        let origin = site.origin();
        let is_granted = self
            .check
            .constrain_access_assignable(origin, place.access, access)?
            .holds();
        if !is_granted {
            let granted = self.check.access_literal(origin, place.access)?;
            self.check
                .report_borrow_access_not_granted(origin, requested, granted, value.ty)?;
        }

        // record the access this borrow requires from the lent place
        if is_granted && let Some(source) = value.node {
            let is_aliased = self.type_is_aliased(origin, value.ty)?;
            self.record_required_access(source, requested, is_aliased);
        }

        // wrap the borrowed value in its borrow form
        let form = self.intern_borrow(lifetime, access)?;
        let borrowed = self.intern_type(dir::Type::Form(dir::FormType {
            form,
            value: value.ty,
        }))?;
        self.commit_node_type(node.into_any(), borrowed)?;

        Ok(())
    }

    /// Record the uses one borrow of a binding value demands from it.
    pub(in crate::sema) fn record_required_access(
        &mut self,
        node: dir::GlobalNodeIdAny,
        requested: dir::Access,
        is_aliased: bool,
    ) {
        // exclusive demand is a place requirement apart from binding mutability
        if requested == dir::Access::Exclusive {
            self.record_access_use(node, dir::BindingUse::EXCLUSIVE);
        }

        // record mutable access to directly stored binding values
        if requested != dir::Access::Readonly && !is_aliased {
            self.record_access_use(node, dir::BindingUse::MUTATE);
        }
    }
}
