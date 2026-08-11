use destack_dir as dir;

use crate::check::{BodyState, FlowSite, PlaceUse};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Select one tagged template through its tag's callable value.
    pub(in crate::check) fn select_tagged_template(
        &mut self,
        site: FlowSite,
        tag: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();

        // read the tag's callable shape
        let tag_node = tag.into_global_any(module);
        let tag_site = self.visit_site(tag_node)?;
        let tag_type = self.infer_node_type(tag_site, PlaceUse::Read)?;
        let signature = match self.ty(tag_type)? {
            dir::Type::FunctionSignature(_) => Some(tag_type),
            _ => self.callable_signature(tag_type)?,
        };
        let Some(signature) = signature else {
            self.report_not_callable(origin, tag_type)?;
            self.commit_decision(node, dir::Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(());
        };
        let return_type = match self.signature_head(signature)? {
            Some(function) => function.return_type,
            _ => {
                return Err(CompilerError::Internal {
                    message: format!("tagged template signature {signature:?} is not callable"),
                });
            }
        };

        // produce the tag's return value
        let result = match return_type {
            Some(return_type) => return_type,
            None => self.intern_type(dir::Type::Void)?,
        };
        let call = dir::Call {
            target: dir::CallTarget::Expression {
                generic_arguments: Vec::new(),
            },
            callable_type: signature,
            arguments: Vec::new(),
            return_type: result,
        };
        let resolution = dir::OperationResolution::One(call);

        self.commit_decision(node, dir::Decision::Call(resolution))?;
        self.commit_node_type(node, result)?;

        Ok(())
    }
}
