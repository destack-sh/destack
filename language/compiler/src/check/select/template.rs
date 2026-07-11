use destack_dir as dir;

use crate::check::{Answer, BodyState, Decision, FlowSite, PlaceUse, answer};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Select one tagged template through its tag's callable value.
    pub(in crate::check) fn select_tagged_template(
        &mut self,
        site: FlowSite,
        tag: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();

        // reduce the tag's callable shape
        let tag_node = tag.into_global_any(module);
        let tag_site = self.node_site(tag_node)?;
        let tag_type = answer!(self.infer_node_type(tag_site, PlaceUse::Read)?);
        let tag_type = answer!(self.reduce_type_head(origin, tag_type)?);
        let signature = match self.ty(tag_type)? {
            dir::Type::FunctionSignature(_) => Some(tag_type),
            _ => self.callable_signature(tag_type)?,
        };
        let Some(signature) = signature else {
            self.report_not_callable(origin, tag_type)?;
            self.commit_decision(node, Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(Answer::Ready(()));
        };
        let return_type = match self.signature_head(signature)? {
            Some(function) => function.return_type,
            _ => {
                return Err(CompilerError::Internal {
                    message: format!("tagged template signature {signature:?} is not callable"),
                });
            }
        };

        // the application produces the tag's return value
        let result = match return_type {
            Some(return_type) => return_type,
            None => self.intern_type(module, dir::Type::Void)?,
        };
        let resolution = dir::CallResolution::new(
            dir::CallTarget::Expression {
                generic_arguments: Vec::new(),
            },
            Some(signature),
            Vec::new(),
            Vec::new(),
            result,
        );
        self.commit_decision(node, Decision::Call(resolution))?;
        self.commit_node_type(node, result)?;

        Ok(Answer::Ready(()))
    }
}
