use destack_dir as dir;

use crate::sema::{CheckState, FlowSite, Origin, PlaceUse, TypeSubstitution};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Select one tagged template through its tag's callable value.
    pub(in crate::sema) fn select_tagged_template(
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
            regions: Vec::new(),
            target: dir::CallableTarget::Expression {
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

    /// Select the calls one interpolated template renders and joins through.
    pub(in crate::sema) fn select_template_calls(
        &mut self,
        site: FlowSite,
        rendered: &[(dir::LocalNodeId<dir::Argument>, dir::GlobalTypeId)],
        string: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // record the calls once, at the visit that closes the template
        let node = site.node;
        if !self.is_checking() || self.decision(node).is_some() {
            return Ok(());
        }

        // render each interpolation through its own display
        let origin = site.origin();
        let module = node.module_id;
        let key = dir::StaticKey::Name(self.strings().intern("display"));
        let mut spans = Vec::with_capacity(rendered.len());
        for (argument, span) in rendered {
            // select the Display call over the interpolated value
            let Some(value) = self.argument_expression(module, *argument) else {
                return Err(CompilerError::Internal {
                    message: "a template span without its argument expression".to_owned(),
                });
            };
            let value_site = self.visit_site(value)?;
            let receiver = self.expression_value(value_site, *span)?;
            let selected = self.select_language_protocol_call(
                origin,
                receiver,
                *span,
                dir::MemberSpace::Instance,
                key,
                dir::LanguageItem::Display,
                &[],
                &[],
                &[],
            )?;

            // report a span outside the display protocol
            let Some((_, call)) = selected else {
                self.report_template_span_not_displayable(value);

                continue;
            };
            let dir::OperationResolution::One(call) = call.resolution else {
                return Err(CompilerError::Internal {
                    message: "a display protocol selected on a union receiver".to_owned(),
                });
            };

            spans.push(call);
        }

        // join the chunks with the rendered spans, leaving an unloaded join undecided
        let Some(build) = self.select_template_join(origin, string)? else {
            return Ok(());
        };

        self.commit_decision(
            node,
            dir::Decision::Template(Box::new(dir::TemplateDecision { spans, build })),
        )
    }

    /// Select the constructor joining one template's chunks with its rendered spans.
    pub(in crate::sema) fn select_template_join(
        &mut self,
        origin: Origin,
        string: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Call>> {
        let symbol = self.language_symbol(dir::LanguageItem::StringFromTemplate)?;
        let Some(call) =
            self.instantiate_symbol_call(origin, symbol, &[], TypeSubstitution::default())?
        else {
            return Ok(None);
        };

        Ok(Some(dir::Call {
            return_type: string,
            ..call
        }))
    }
}
