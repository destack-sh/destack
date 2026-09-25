use tspp_dir as dir;
use tspp_mir as mir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult};

/// The routing of one member access over union arms.
pub(in crate::lower) struct UnionDispatch {
    /// Each arm's receiver adjustment chain.
    pub(in crate::lower) chains: Vec<Vec<dir::ReceiverAdjustment>>,
    /// The number of shared steps before the dispatch projection.
    pub(in crate::lower) prefix: usize,
    /// The block each arm's case switches to.
    pub(in crate::lower) targets: Vec<(u32, mir::LocalNodeId<mir::Block>)>,
}

impl FunctionLowerer<'_, '_, '_> {
    /// Return the void case of one variant representation: the absent case it stores.
    pub(in crate::lower) fn absent_case(&mut self, representation: mir::TypeId) -> Option<u32> {
        let void = self.builder.tree_mut().intern_type(mir::Type::Void);

        let mir::Type::Variant { cases, .. } = self.builder.tree().type_definition(representation)
        else {
            return None;
        };

        cases
            .iter()
            .position(|case| case.ty == void)
            .map(|index| index as u32)
    }

    /// Return the case of one variant representation that stores one type.
    pub(in crate::lower) fn variant_case(
        &self,
        variant: mir::TypeId,
        ty: mir::TypeId,
    ) -> CompilerResult<u32> {
        let mir::Type::Variant { cases, .. } = self.builder.tree().type_definition(variant) else {
            return Err(self.internal("a variant case outside a variant type"));
        };
        let Some(case) = cases.iter().position(|case| case.ty == ty) else {
            return Err(self.internal("a variant case outside the variant's cases"));
        };

        Ok(case as u32)
    }

    /// Return the null and undefined cases of one variant representation, in case order.
    pub(in crate::lower) fn nullish_cases(&mut self, representation: mir::TypeId) -> Vec<u32> {
        let null = self
            .lower
            .singleton_type(self.builder.tree_mut(), &dir::Literal::Null);
        let tree = self.builder.tree();
        let mir::Type::Variant { cases, .. } = tree.type_definition(representation) else {
            return Vec::new();
        };

        cases
            .iter()
            .enumerate()
            .filter(|(_, case)| case.ty == null || matches!(tree.get(case.ty), mir::Type::Void))
            .map(|(index, _)| index as u32)
            .collect()
    }

    /// Return the undefined value one nullish representation stores, when it stores one.
    pub(in crate::lower) fn absent_value(
        &mut self,
        representation: mir::TypeId,
    ) -> Option<mir::Value> {
        let case = self.absent_case(representation)?;

        Some(self.builder.variant_new(representation, case, None))
    }

    /// Return the case index of one member among its union's members.
    pub(in crate::lower) fn case(
        &self,
        members: &[dir::GlobalTypeId],
        member: dir::GlobalTypeId,
    ) -> CompilerResult<u32> {
        members
            .iter()
            .position(|candidate| *candidate == member)
            .map(|index| index as u32)
            .ok_or_else(|| self.internal("a selected arm outside its union"))
    }

    /// Read one value through the newtype layers wrapping its variant.
    pub(in crate::lower) fn read_through_newtypes(
        &mut self,
        mut value: mir::Value,
    ) -> CompilerResult<mir::Value> {
        // read the sole field of each newtype layer in turn
        loop {
            let ty = self.value_representation(value)?;
            let ty = mir::Substitution::resolve(ty, self.builder.tree_mut());
            if !matches!(self.builder.tree().get(ty), mir::Type::Newtype { .. }) {
                break;
            }
            value = self.builder.field_get(value, 0);
        }

        Ok(value)
    }

    /// Return the storage index of one language item's named member.
    pub(in crate::lower) fn language_member_field(
        &mut self,
        ty: dir::GlobalTypeId,
        member: &str,
    ) -> CompilerResult<u32> {
        // read the struct storage the type lowers to
        let representation = self.lower_type(ty)?;
        let storage = self.builder.tree_mut().storage_type(representation);
        let mir::Type::Struct { fields, .. } = self.builder.tree().get(storage).clone() else {
            return Err(CompilerError::Internal {
                message: "a language member outside struct storage".to_string(),
            });
        };

        // name the member through the language item the type applies
        let dir::Type::Application(application) = self.lower.ty(ty)? else {
            return Err(CompilerError::Internal {
                message: "a language member outside an applied nominal".to_string(),
            });
        };
        let Some(item) = self.lower.language_item(application.symbol) else {
            return Err(CompilerError::Internal {
                message: "a language member outside a language item".to_string(),
            });
        };
        let declared_member = item.member(member);
        let dir::StaticKey::Name(name) = declared_member.key else {
            unreachable!("a language member named by text")
        };

        // find the field storing that member
        let index = fields
            .iter()
            .position(|field| self.builder.tree().get(*field).name == Some(name))
            .ok_or_else(|| CompilerError::Internal {
                message: format!("a language item without its '{member}' member"),
            })?;

        Ok(index as u32)
    }

    /// Return the storage index of one union case's payload.
    pub(in crate::lower) fn union_case_value_field(
        &mut self,
        member: dir::GlobalTypeId,
    ) -> CompilerResult<u32> {
        self.language_member_field(member, "value")
    }

    /// Route recorded union arms, each case switched to a block of its own past the shared steps.
    pub(in crate::lower) fn union_dispatch(
        &mut self,
        arms: &[dir::MemberAccess],
    ) -> CompilerResult<UnionDispatch> {
        let mut chains = Vec::with_capacity(arms.len());
        for arm in arms {
            chains.push(self.union_arm_receiver(arm)?.adjustments.clone());
        }
        let (prefix, union) = self.union_dispatch_split(&chains)?;
        let targets = self.union_dispatch_targets(&chains, prefix, union)?;

        Ok(UnionDispatch {
            chains,
            prefix,
            targets,
        })
    }

    /// Split recorded arm chains at the dispatch projection they share.
    fn union_dispatch_split(
        &self,
        chains: &[Vec<dir::ReceiverAdjustment>],
    ) -> CompilerResult<(usize, dir::GlobalTypeId)> {
        // find the first chain's dispatch projection
        let Some(first) = chains.first() else {
            return Err(CompilerError::Internal {
                message: "a union dispatch without arm chains".to_string(),
            });
        };
        let dispatched = first
            .iter()
            .position(|step| matches!(step, dir::ReceiverAdjustment::UnionPayload { .. }));
        let Some(prefix) = dispatched else {
            return Err(CompilerError::Internal {
                message: "a union member arm without its dispatch projection".to_string(),
            });
        };

        // require every chain to share the steps ahead of the projection
        if !chains.iter().all(|chain| {
            chain.get(..prefix + 1).is_some_and(|steps| {
                steps[..prefix] == chains[0][..prefix]
                    && matches!(steps[prefix], dir::ReceiverAdjustment::UnionPayload { .. })
            })
        }) {
            return Err(CompilerError::Internal {
                message: "a union member arm outside the shared dispatch projection".to_string(),
            });
        }
        let dir::ReceiverAdjustment::UnionPayload { union, .. } = chains[0][prefix] else {
            unreachable!("the dispatch step is selected as a union projection");
        };

        Ok((prefix, union))
    }

    /// Route each recorded arm chain to a block at its canonical case.
    fn union_dispatch_targets(
        &mut self,
        chains: &[Vec<dir::ReceiverAdjustment>],
        prefix: usize,
        union: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<(u32, mir::LocalNodeId<mir::Block>)>> {
        let members = self.lower.union_members(union)?;
        let mut targets = Vec::with_capacity(chains.len());
        for chain in chains {
            let dir::ReceiverAdjustment::UnionPayload { arm: case, .. } = chain[prefix] else {
                unreachable!("an arm chain without a dispatch step at its prefix");
            };
            let position = self.case(&members, case)?;
            targets.push((position, self.builder.block()));
        }

        Ok(targets)
    }

    /// Return one union arm's adjusted receiver.
    pub(in crate::lower) fn union_arm_receiver<'access>(
        &self,
        arm: &'access dir::MemberAccess,
    ) -> CompilerResult<&'access dir::AdjustedReceiver> {
        let adjusted = match &arm.target {
            dir::MemberTarget::Field(field) => match &field.receiver {
                dir::MemberReceiver::Direct(adjusted) => Some(adjusted),
                _ => None,
            },
            dir::MemberTarget::Projection { receiver, .. } => Some(receiver),
            _ => None,
        };

        adjusted.ok_or_else(|| CompilerError::Internal {
            message: "a union member arm without an adjusted receiver".to_string(),
        })
    }

    /// Return the case one condition selects on a variant operand read through its references.
    pub(in crate::lower) fn union_case_test(
        &mut self,
        operand: &dir::PredicateOperand,
        condition: &dir::PredicateCondition,
    ) -> CompilerResult<Option<u32>> {
        // read a direct operand down to the union it names a case of
        let dir::PredicateOperand::Direct(operand) = operand else {
            return Ok(None);
        };
        let operand = match self.lower.indirection(*operand, &self.scope)? {
            Some(layer) => layer.stored,
            None => self.lower.stored(*operand)?,
        };
        let Some(members) = self.lower.union_members_maybe(operand)? else {
            return Ok(None);
        };

        match condition {
            // select the member the condition names
            dir::PredicateCondition::Type(target) | dir::PredicateCondition::Subtype(target) => {
                let index = members.iter().position(|member| member == target);

                Ok(index.map(|index| index as u32))
            }
            // select the member of the tested primitive type
            dir::PredicateCondition::Primitive(primitive) => {
                let primitive = dir::Type::Primitive(*primitive);
                for (index, member) in members.iter().enumerate() {
                    if self.lower.ty(*member)? == primitive {
                        return Ok(Some(index as u32));
                    }
                }

                Ok(None)
            }
            // select no case under any other condition
            _ => Ok(None),
        }
    }
}
