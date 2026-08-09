use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{BodyState, FlowSite, PlaceUse};

impl BodyState<'_, '_> {
    /// Infer one borrow expression from its borrowed value and lifetime.
    pub(in crate::check) fn infer_borrow_expression(
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
            .constrain_access_assignable(origin, place.access, access)?;
        if !is_granted {
            let granted = self.check.access_literal(origin, place.access)?;
            self.check
                .report_borrow_access_not_granted(origin, requested, granted, value.ty)?;
        }

        // wrap the borrowed value and reduce redundant memory forms
        let form = self.intern_borrow(lifetime, access)?;
        let borrowed = self.intern_type(dir::Type::Form(dir::FormType {
            form,
            value: value.ty,
        }))?;
        let borrowed = self.reduce_type_head(site.origin(), borrowed)?;
        self.commit_node_type(node.into_any(), borrowed)?;

        Ok(())
    }
}
