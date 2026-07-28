use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, BodyState, FlowSite, PlaceUse, answer};

impl BodyState<'_, '_> {
    /// Infer one borrow expression from its borrowed value and lifetime.
    pub(in crate::check) fn infer_borrow_expression(
        &mut self,
        site: FlowSite,
        mutability: Option<dir::Mutability>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let right_site = self.node_site(right.into_global_any(node.module_id))?;
        let ty = answer!(self.infer_node_type(right_site, PlaceUse::Read)?);
        let value = answer!(self.expression_value(right_site, ty)?);
        let place = answer!(self.value_place(right_site.origin(), value)?);

        // derive borrow form parameters from the place and written mutability
        let lifetime = place.lifetime;
        let requested = match mutability {
            Some(mutability) => mutability.access(),
            None => dir::Access::Mutable,
        };
        let access = self.intern_type(
            node.module_id,
            dir::Type::Memory(dir::MemoryLiteral::Access(requested)),
        )?;

        // require the requested access from the selected place
        let origin = site.origin();
        let is_granted = answer!(self.check.constrain_access_assignable(
            origin,
            place.access,
            access,
        )?);
        if !is_granted {
            let granted = answer!(self.check.access_literal(origin, place.access)?);
            self.check
                .report_borrow_access_not_granted(origin, requested, granted, value.ty)?;
        }

        // wrap the borrowed value and reduce redundant memory forms
        let form = self.intern_borrow(node.module_id, lifetime, access)?;
        let borrowed = self.intern_type(
            node.module_id,
            dir::Type::Form(dir::FormType {
                form,
                value: value.ty,
            }),
        )?;
        let borrowed = answer!(self.reduce_type_head(site.origin(), borrowed)?);
        self.commit_node_type(node.into_any(), borrowed)?;

        Ok(Answer::Ready(()))
    }
}
