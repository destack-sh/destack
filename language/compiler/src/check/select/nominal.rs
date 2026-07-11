use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, BodyState, Cause, CauseKind, FlowPointId, Origin, Relation, answer};

impl BodyState<'_, '_> {
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
            return self.commit_rejected_pattern(node);
        }

        // select tagged owner.case patterns before ordinary newtype unwraps
        if let Some(head) = answer!(self.tagged_pattern_head(origin, module, ty)?) {
            return self.select_tagged_variant_pattern(node, origin, flow, scope, head, fields);
        }

        // resolve the written nominal tag like a construction head
        let tag = answer!(self.written_construct_tag(origin, module, ty)?);
        let tag = answer!(self.reduce_type_head(origin, tag)?);
        let instance = match self.ty(tag)? {
            dir::Type::Instance(instance) => instance,
            _ => return self.reject_pattern(node, origin, tag),
        };

        // unwrap the substituted newtype backing
        let backing = match self.definition(instance.symbol)? {
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
        let generic_arguments =
            self.symbol_generic_argument_bindings(instance.symbol, &arguments)?;
        self.commit_pattern(
            node,
            dir::PatternResolution::Project(dir::PatternProjectionResolution {
                projection: dir::Projection::NewtypePayload {
                    symbol: instance.symbol,
                    generic_arguments,
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
            return self.commit_rejected_pattern(node);
        }

        // select tagged owner.case patterns before nominal fields
        if let Some(head) = answer!(self.tagged_pattern_head(origin, module, ty)?) {
            return self.select_tagged_variant_pattern(node, origin, flow, scope, head, fields);
        }

        // resolve the written nominal tag like a construction head
        let tag = answer!(self.written_construct_tag(origin, module, ty)?);
        let tag = answer!(self.reduce_type_head(origin, tag)?);
        let instance = match self.ty(tag)? {
            dir::Type::Instance(instance) => instance,
            _ => return self.reject_pattern(node, origin, tag),
        };

        // bind the pattern instantiation from the matched input
        let input = answer!(self.node_type(node.into_any())?);
        let input = answer!(self.value_beneath_forms(origin, input)?);
        let mut matched = input;
        if let Some(backing) = answer!(self.newtype_backing(origin, input)?) {
            matched = answer!(self.reduce_type_head(origin, backing)?);
        }
        let arms: SmallVec<[dir::GlobalTypeId; 4]> = match self.ty(matched)? {
            dir::Type::Union(union) => {
                SmallVec::from_slice(self.type_ids(matched.module_id, union.elements)?)
            }
            _ => SmallVec::from_slice(&[matched]),
        };
        for arm in arms {
            let arm = answer!(self.reduce_type_head(origin, arm)?);
            if let dir::Type::Instance(arm_instance) = self.ty(arm)?
                && arm_instance.symbol == instance.symbol
            {
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                answer!(self.constrain_type(cause, Relation::Equal, arm, tag)?);
                break;
            }
        }

        // project declared fields off the matched declaration
        let (fields, rest) =
            answer!(self.project_named_fields(node, origin, flow, scope, tag, fields)?);
        let arguments = self.type_ids(tag.module_id, instance.arguments)?.to_vec();
        let generic_arguments =
            self.symbol_generic_argument_bindings(instance.symbol, &arguments)?;
        self.commit_pattern(
            node,
            dir::PatternResolution::Destructure(dir::PatternDestructureResolution::Nominal(
                dir::PatternNominalDestructureResolution {
                    symbol: instance.symbol,
                    generic_arguments,
                    fields,
                    rest,
                },
            )),
        )
    }
}
