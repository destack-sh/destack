use crate::CompilerResult;
use crate::check::{Answer, CheckState, FlowSite, PlaceUse, answer};
use destack_dir as dir;

impl CheckState<'_> {
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

        // derive borrow form parameters from the place and written mutability
        let lifetime = answer!(self.borrowed_expression_lifetime(node, right, value)?);
        let access = match mutability {
            Some(mutability) => mutability.access(),
            None => dir::Access::Mutable,
        };
        let access = self.intern_type(
            node.module_id,
            dir::Type::Memory(dir::MemoryLiteral::Access(access)),
        )?;

        // wrap the borrowed value and reduce redundant memory forms
        let borrowed = self.intern_type(
            node.module_id,
            dir::Type::Form(dir::FormType {
                form: dir::Form::Borrowed { lifetime, access },
                value,
            }),
        )?;
        let borrowed = answer!(self.reduce_type_head(site.origin(), borrowed)?);
        self.commit_node_type(node.into_any(), borrowed)?;

        Ok(Answer::Ready(()))
    }
}
