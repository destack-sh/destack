use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Decision, FlowPointId, Origin, answer};

impl CheckState<'_> {
    /// Select one newtype pattern, unwrapping the substituted backing.
    pub(in crate::check) fn select_newtype_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        self.check_pattern_bindings(module, fields)?;

        if !self.check_pattern_rest_fields(module, fields) {
            self.commit_decision(node.into_any(), Decision::Rejected)?;

            return Ok(Answer::Ready(()));
        }

        // select tagged owner.case patterns before ordinary newtype unwraps
        if let Some(head) = answer!(self.tagged_pattern_head(origin, module, ty)?) {
            return self.select_tagged_variant_pattern(node, origin, flow, scope, head, fields);
        }

        // reduce the written nominal tag
        let tag_node = ty.into_global_any(module);
        let tag = answer!(self.committed_node_type(tag_node)?);
        let tag = answer!(self.reduce_type_head(origin, tag)?);
        let instance = match self.ty(tag)? {
            dir::Type::Instance(instance) => instance,
            _ => return self.reject_pattern(node, origin, tag),
        };

        // unwrap the substituted newtype backing
        let backing = match self.definition(instance.symbol) {
            Some(dir::Definition::Newtype(definition)) => definition.value,
            _ => return self.reject_pattern(node, origin, tag),
        };
        let substitution = self
            .instance_substitution(tag.module_id, &instance)?
            .with_receiver(tag);
        let backing = self.substitute_type(origin.module(), backing, &substitution)?;

        // flow the backing into the wrapped hole
        let value = fields
            .first()
            .and_then(|field| match self.module(module).view().get(*field) {
                dir::PatternField::Positional { pattern } => Some(*pattern),
                _ => None,
            });
        if let Some(value) = value {
            self.project_pattern_input(flow, scope, backing, value.into_global_any(module))?;
        }

        let arguments = self.type_ids(tag.module_id, instance.arguments)?.to_vec();
        self.commit_pattern(
            node,
            dir::PatternResolution::Project(dir::PatternProjectionResolution {
                projection: dir::Projection::NewtypePayload {
                    symbol: instance.symbol,
                    generic_arguments: self
                        .symbol_generic_argument_bindings(instance.symbol, &arguments)?,
                    ty: backing,
                },
                pattern: value.map(|value| value.into_global_any(module)),
            }),
        )
    }

    /// Select one nominal object pattern, projecting declared fields.
    pub(in crate::check) fn select_nominal_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        self.check_pattern_bindings(module, fields)?;

        if !self.check_pattern_rest_fields(module, fields) {
            self.commit_decision(node.into_any(), Decision::Rejected)?;

            return Ok(Answer::Ready(()));
        }

        // reduce the written nominal tag
        let tag_node = ty.into_global_any(module);
        let tag = answer!(self.committed_node_type(tag_node)?);
        let tag = answer!(self.reduce_type_head(origin, tag)?);
        let instance = match self.ty(tag)? {
            dir::Type::Instance(instance) => instance,
            _ => return self.reject_pattern(node, origin, tag),
        };

        // project declared fields off the matched declaration
        let (fields, rest) =
            answer!(self.project_named_fields(node, origin, flow, scope, tag, fields)?);
        let arguments = self.type_ids(tag.module_id, instance.arguments)?.to_vec();
        self.commit_pattern(
            node,
            dir::PatternResolution::Destructure(dir::PatternDestructureResolution::Nominal(
                dir::PatternNominalDestructureResolution {
                    symbol: instance.symbol,
                    generic_arguments: self
                        .symbol_generic_argument_bindings(instance.symbol, &arguments)?,
                    fields,
                    rest,
                },
            )),
        )
    }
}
