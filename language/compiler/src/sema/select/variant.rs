use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{CheckState, Origin};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Select one written enum variant pattern.
    pub(in crate::sema) fn select_variant_type_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        module: ModuleId,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<bool> {
        let Some((owner, symbol, key)) = self.variant_pattern_owner(module, ty)? else {
            return Ok(false);
        };
        let Some(case) = self.variant_case(symbol, key)? else {
            let key = self.format_static_key(&key);
            self.report_pattern_variant_missing(origin, key, owner)?;
            self.commit_rejected_pattern(node)?;

            return Ok(true);
        };

        // commit the selected enum case
        self.select_variant_pattern(node, origin, case, fields)?;

        Ok(true)
    }

    /// Return the enum owner and variant key named by one pattern.
    fn variant_pattern_owner(
        &mut self,
        module: ModuleId,
        ty: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Option<(dir::GlobalTypeId, dir::GlobalSymbolId, dir::StaticKey)>> {
        let source = ty.into_global_any(module);
        let expression = self.module(module).view().get(ty).clone();
        let (owner, key) = match expression {
            // read an explicitly segmented member type
            dir::TypeExpression::Member { left, name, .. } => {
                let owner = self.require_node_type(left.into_global_any(module))?;
                let key = dir::StaticKey::Name(name);

                (owner, key)
            }
            // read a projected path reference
            dir::TypeExpression::Reference { path, .. } => {
                let reference = self.module(module).resolved.references.get(source).cloned();
                let Some(dir::Reference::Projected {
                    base: dir::ReferenceTarget::Symbol(base),
                    from,
                }) = reference
                else {
                    // bound references name declarations, never variant cases
                    return Ok(None);
                };
                let Some(name) = path.segments.get(from as usize).copied() else {
                    return Ok(None);
                };
                let owner = self.symbol_type(base)?;
                let key = dir::StaticKey::Name(name);

                (owner, key)
            }
            // read the selected member type directly
            _ => {
                let ty = self.require_node_type(source)?;
                let Some(member) = self.member_head(ty)? else {
                    return Ok(None);
                };

                (member.owner, member.key)
            }
        };

        // require an enum owner for variant pattern syntax
        let symbol = match self.ty(owner)? {
            // accept one instantiated enum
            dir::Type::Application(instance) => instance.symbol,
            // accept its declaration reference
            dir::Type::Reference(reference) => reference.symbol,
            // reject every non-nominal owner
            _ => return Ok(None),
        };
        if !matches!(
            self.definition(symbol)?.as_deref(),
            Some(dir::Definition::Enum(_))
        ) {
            return Ok(None);
        }

        Ok(Some((owner, symbol, key)))
    }

    /// Return the variant case named by one expression pattern.
    pub(in crate::sema) fn variant_expression_case(
        &mut self,
        module: ModuleId,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::VariantCase>> {
        let dir::Expression::Member {
            left,
            name: Some(name),
            ..
        } = self.module(module).view().get(value).clone()
        else {
            return Ok(None);
        };

        // peel explicit application from the owner declaration reference
        let mut owner = left;
        while let dir::Expression::Instantiation { left, .. } =
            self.module(module).view().get(owner)
        {
            owner = *left;
        }

        // select the referenced enum case
        let Some(owner) = self.reference_symbol(owner.into_global_any(module))? else {
            return Ok(None);
        };
        let case = self.variant_case(owner, dir::StaticKey::Name(name))?;

        Ok(case)
    }

    /// Return one variant case from its declaration symbol and key.
    fn variant_case(
        &mut self,
        owner: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::VariantCase>> {
        let member = match self.definition(owner)?.as_deref() {
            Some(dir::Definition::Enum(definition)) => {
                definition.variant_by_key(key).map(|variant| variant.symbol)
            }
            _ => None,
        };
        let case = member.map(|variant| dir::VariantCase {
            owner,
            key,
            variant,
        });

        Ok(case)
    }

    /// Select one enum variant pattern.
    pub(in crate::sema) fn select_variant_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        case: dir::VariantCase,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<()> {
        let input = self.require_node_type(node.into_any())?;
        let owners = self.variant_owners(origin, input, &case)?;

        // require the selected case to inhabit the matched input
        if owners.is_empty() {
            let owner = self.format_symbol(case.owner);
            let key = self.format_static_key(&case.key);
            let variant = format!("{owner}.{key}");
            self.report_pattern_variant_not_in_type(origin, variant, input)?;

            return self.commit_rejected_pattern(node);
        }

        self.select_enum_member_pattern(node, origin, case, &owners, fields)
    }

    /// Return enum owner instances visible in the matched input.
    fn variant_owners(
        &mut self,
        origin: Origin,
        input: dir::GlobalTypeId,
        case: &dir::VariantCase,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let input = self.strip_form(origin, input)?;
        let alternatives = match self.union_arms(origin, input)? {
            Some(arms) => arms.into_vec(),
            None => vec![input],
        };
        let mut owners = Vec::new();

        // collect each alternative that can still carry the selected case
        for alternative in alternatives {
            let owner = match self.ty(alternative)? {
                // accept a value of the whole enum
                dir::Type::Application(instance) if instance.symbol == case.owner => {
                    Some(alternative)
                }
                // accept a value already narrowed to this exact variant
                dir::Type::Variant(variant) if variant.variant == case.variant => {
                    let dir::Type::Application(instance) = self.ty(variant.owner)? else {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "variant type {alternative:?} has non-application owner {:?}",
                                variant.owner
                            ),
                        });
                    };

                    (instance.symbol == case.owner).then_some(variant.owner)
                }
                // ignore unrelated alternatives
                _ => None,
            };

            // keep the matching owner instance
            if let Some(owner) = owner {
                owners.push(owner);
            }
        }

        Ok(owners)
    }
}
