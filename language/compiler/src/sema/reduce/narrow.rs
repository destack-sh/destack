use smallvec::SmallVec;
use tspp_dir as dir;

use crate::sema::{CheckState, GenericParameterId, Origin, Relation, Verdict};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Narrow one receiver through a successful static property-membership test.
    pub(in crate::sema) fn narrow_membership_receiver(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let receiver = self.normalize(origin, receiver)?;

        // filter union arms independently by their declared member sets
        if let dir::Type::Union(union) = self.ty(receiver)? {
            let arms: SmallVec<[_; 8]> = self.type_ids(receiver.module_id, union.elements)?.into();
            let mut narrowed = Vec::with_capacity(arms.len());
            for arm in arms {
                let arm = self.narrow_membership_receiver(origin, arm, key)?;
                if !matches!(self.ty(arm)?, dir::Type::Never) {
                    narrowed.push(arm);
                }
            }

            return self.normalized_union_type(narrowed);
        }

        // preserve the receiver exactly for declared members
        let lookup = self.lookup_inherent_member(
            origin,
            origin.module(),
            receiver,
            dir::MemberSpace::Instance,
            key,
        )?;
        if self.member_read_type(&lookup)?.is_some() {
            return Ok(receiver);
        }

        // make the successful branch unreachable for closed member sets
        if !self.may_have_additional_member(origin, receiver, key)? {
            let never = self.intern_type(dir::Type::Never)?;

            return Ok(never);
        }

        // retain the receiver alongside the property established at runtime
        let unknown = self.intern_type(dir::Type::Unknown)?;
        let member = self.field_shape_type(key, unknown)?;
        let member_is_narrower = self
            .decide_relation(origin, Relation::Subtype, member, receiver)?
            .holds();
        let narrowed = if member_is_narrower {
            member
        } else {
            self.normalized_intersection_type([receiver, member])?
        };

        Ok(narrowed)
    }

    /// Filter one source's physical arms through a tested member value.
    pub(in crate::sema) fn narrow_arms(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        keys: &[dir::StaticKey],
        target: dir::GlobalTypeId,
        is_positive: bool,
    ) -> CompilerResult<Result<Option<dir::GlobalTypeId>, dir::TypeVariableId>> {
        let subject = self.normalize(origin, source)?;

        // enumerate the cases an enum owner names
        let mut newtype = None;
        let arms = if let Some(variants) = self.variant_types(subject)? {
            variants
        }
        // enumerate the physical arms every other source carries
        else {
            let (payload, steps) = self.project_newtype_receiver(origin, subject)?;
            let Some(arms) = self.union_arms(origin, payload)? else {
                return Ok(Ok(None));
            };
            newtype = (!steps.is_empty()).then_some(subject);

            arms.into_vec()
        };

        // count the arms before narrowing
        let arm_count = arms.len();

        // keep original arms whose tested member remains inhabited
        let mut kept = Vec::with_capacity(arms.len());
        for arm in arms {
            let narrowed = match self.narrow_member(origin, arm, keys, target, is_positive)? {
                Ok(narrowed) => narrowed,
                Err(variable) => return Ok(Err(variable)),
            };
            let narrowed = self.normalize(origin, narrowed)?;
            if !matches!(self.ty(narrowed)?, dir::Type::Never) {
                kept.push(arm);
            }
        }

        // join the surviving arms back into one type
        let narrowed = match (kept.as_slice(), newtype) {
            ([], _) => self.intern_type(dir::Type::Never)?,
            (_, Some(newtype)) if kept.len() == arm_count => newtype,
            // the newtype's memory form wraps the intersection of its payload with the arms
            (_, Some(newtype)) => {
                let mut bases = Vec::with_capacity(kept.len());
                for arm in kept {
                    bases.push(self.form_chain(origin, arm)?.base());
                }
                let arms = self.normalized_union_type(bases)?;
                let base = self.form_chain(origin, newtype)?.base();
                let narrowed = self.normalized_intersection_type([base, arms])?;

                self.replace_form_value(origin, newtype, narrowed)?
            }
            ([single], None) => *single,
            (_, None) => self.normalized_union_type(kept)?,
        };

        Ok(Ok(Some(narrowed)))
    }

    /// Return the finite runtime domain of one rigid parameter.
    pub(in crate::sema) fn parameter_domain(
        &mut self,
        origin: Origin,
        parameter: GenericParameterId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // select one finite bound that enumerates the domain
        let bounds = self.parameter_bounds(origin, parameter)?;
        let mut enumerated = None;
        for bound in &bounds {
            let bound = self.deeply_resolve(origin, *bound)?;
            if self.union_arms(origin, bound)?.is_some() {
                enumerated = Some(bound);

                break;
            }
        }

        // require an enumerated domain
        let Some(enumerated) = enumerated else {
            return Ok(None);
        };

        // intersect the enumerated arms with every active bound
        let domain = self.normalized_intersection_type(bounds)?;
        let narrowing = dir::NarrowType {
            source: enumerated,
            target: domain,
            is_positive: true,
        };
        let Some(domain) = self.reduce_narrowing(origin, narrowing)? else {
            let shown = self.intern_type(dir::Type::Parameter(parameter))?;

            return Err(CompilerError::Internal {
                message: format!(
                    "parameter {} has an irreducible domain {}",
                    self.format_type(shown),
                    self.format_type(domain),
                ),
            });
        };

        // reject an empty narrowed domain
        if matches!(self.ty(domain)?, dir::Type::Never) {
            return Ok(None);
        }

        Ok(Some(domain))
    }

    /// Evaluate one runtime guard narrowing.
    pub(in crate::sema) fn reduce_narrowing(
        &mut self,
        origin: Origin,
        narrow: dir::NarrowType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let source = self.normalize(origin, narrow.source)?;
        let target = self.normalize(origin, narrow.target)?;

        // preserve precise case identity for enum owners
        let variants = self.variant_types(source)?;

        // narrow newtypes through their substituted runtime backing
        if variants.is_none()
            && let Some(instance) = self.decompose_newtype(origin, source)?
        {
            let backing = self.normalize(origin, instance.backing)?;

            return self.reduce_narrowing(
                origin,
                dir::NarrowType {
                    source: backing,
                    target,
                    is_positive: narrow.is_positive,
                },
            );
        }

        // narrow the payload beneath memory forms, then rebuild the forms
        if matches!(self.ty(source)?, dir::Type::Form(_)) {
            let value = self.strip_form(origin, source)?;
            let target_value = self.strip_form(origin, target)?;
            let operation = self.intern_operation(dir::TypeOperation::Narrow(dir::NarrowType {
                source: value,
                target: target_value,
                is_positive: narrow.is_positive,
            }))?;
            let narrowed = self.normalize(origin, operation)?;
            if matches!(self.ty(narrowed)?, dir::Type::Operation(_)) {
                return Ok(None);
            }
            if matches!(self.ty(narrowed)?, dir::Type::Never) {
                return Ok(Some(narrowed));
            }

            let rebuilt = self.replace_form_value(origin, source, narrowed)?;

            return Ok(Some(rebuilt));
        }

        // distribute over variant and union valued sources
        let elements = match variants {
            Some(variants) => SmallVec::from_vec(variants),
            None => match self.ty(source)? {
                // distribute over the canonical flat members, seeing through alias arms
                dir::Type::Union(union) => match self.canonical_union_members(origin, source)? {
                    Some(members) => members,
                    None => SmallVec::<[_; 4]>::from_slice(
                        self.type_ids(source.module_id, union.elements)?,
                    ),
                },
                dir::Type::Variable(_) | dir::Type::Parameter(_) => SmallVec::from_slice(&[source]),
                // expand or defer operations before distributing
                dir::Type::Operation(operation) => {
                    let variables = self.type_variables(source)?;
                    if variables.is_empty() {
                        // expand closed operations before distributing
                        let expanded = self.deeply_resolve(origin, source)?;
                        if expanded != source {
                            return self.reduce_narrowing(
                                origin,
                                dir::NarrowType {
                                    source: expanded,
                                    target: narrow.target,
                                    is_positive: narrow.is_positive,
                                },
                            );
                        }

                        // narrow irreducible template patterns as single arms
                        if matches!(
                            self.type_operation(source.module_id, operation)?,
                            dir::TypeOperation::TemplateLiteral(_)
                        ) {
                            SmallVec::from_slice(&[source])
                        } else {
                            return Ok(None);
                        }
                    } else {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "open variables {variables:?} reached narrowing of {source:?}"
                            ),
                        });
                    }
                }
                _ => SmallVec::from_slice(&[source]),
            },
        };

        // filter each arm through the guard relation
        let mut kept = Vec::with_capacity(elements.len());
        for element in elements {
            // defer the whole operation on an undecided arm
            let narrowed = match self.narrow_arm(origin, element, target, narrow.is_positive)? {
                Ok(narrowed) => narrowed,
                Err(_) => return Ok(None),
            };
            let narrowed = self.normalize(origin, narrowed)?;
            if matches!(self.ty(narrowed)?, dir::Type::Never) {
                continue;
            }
            if !kept.contains(&narrowed) {
                kept.push(narrowed);
            }
        }

        // rebuild the filtered result
        let joined = match kept.as_slice() {
            [] => self.intern_type(dir::Type::Never)?,
            [single] => *single,
            _ => self.normalized_union_type(kept)?,
        };

        Ok(Some(joined))
    }

    /// Return the variable that blocks an ambiguous relation between one source and target.
    fn narrow_blocking_variable(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<dir::TypeVariableId> {
        self.collect_open_variables([source, target])?
            .first()
            .copied()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "narrowing {source:?} against {target:?} is ambiguous without an open variable"
                ),
            })
    }

    /// Narrow one arm through its tested member chain, or the variable blocking it.
    fn narrow_member(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        keys: &[dir::StaticKey],
        target: dir::GlobalTypeId,
        is_positive: bool,
    ) -> CompilerResult<Result<dir::GlobalTypeId, dir::TypeVariableId>> {
        // test the final value once the chain ends
        let [key, rest @ ..] = keys else {
            return self.narrow_arm(origin, source, target, is_positive);
        };

        // project the tested member, comparing values beneath memory forms
        let lookup = self.lookup_inherent_member(
            origin,
            origin.module(),
            source,
            dir::MemberSpace::Instance,
            *key,
        )?;
        if let Some(projected) = self.member_read_type(&lookup)? {
            let projected = self.strip_form(origin, projected)?;
            let projected = self.normalize(origin, projected)?;

            // continue the chain through the projected member
            let narrowed = match self.narrow_member(origin, projected, rest, target, is_positive)? {
                Ok(narrowed) => narrowed,
                blocked @ Err(_) => return Ok(blocked),
            };

            // retain the whole arm while its tested member stays inhabited
            let narrowed = if matches!(self.ty(narrowed)?, dir::Type::Never) {
                narrowed
            } else {
                source
            };

            return Ok(Ok(narrowed));
        }

        // fail the positive test for closed member sets and pass every negative one
        if !self.may_have_additional_member(origin, source, *key)? {
            let narrowed = if is_positive {
                self.intern_type(dir::Type::Never)?
            } else {
                source
            };

            return Ok(Ok(narrowed));
        }

        // keep the whole arm for an open member set
        Ok(Ok(source))
    }

    /// Narrow one source arm through one runtime target, or the variable blocking it.
    fn narrow_arm(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        is_positive: bool,
    ) -> CompilerResult<Result<dir::GlobalTypeId, dir::TypeVariableId>> {
        // narrow an erased value through its checked runtime domain
        if let dir::Type::Dynamic(dynamic) = self.ty(source)? {
            let constraint = dynamic.constraint;
            let narrowed_constraint =
                match self.narrow_arm(origin, constraint, target, is_positive)? {
                    Ok(narrowed) => narrowed,
                    blocked @ Err(_) => return Ok(blocked),
                };
            let narrowed = if matches!(self.ty(narrowed_constraint)?, dir::Type::Never) {
                narrowed_constraint
            } else if narrowed_constraint == constraint || !is_positive {
                source
            } else {
                narrowed_constraint
            };

            return Ok(Ok(narrowed));
        }

        // decide disjoint arms without assignability
        if !self.types_may_overlap(origin, source, target)? {
            let narrowed = if is_positive {
                self.intern_type(dir::Type::Never)?
            } else {
                source
            };

            return Ok(Ok(narrowed));
        }

        // keep or remove the source arm on an exact match, deferring an undecided one
        match self.decide_relation(origin, Relation::Subtype, source, target)? {
            Verdict::Holds => {
                let narrowed = if is_positive {
                    source
                } else {
                    self.intern_type(dir::Type::Never)?
                };

                return Ok(Ok(narrowed));
            }
            Verdict::Ambiguous => {
                let variable = self.narrow_blocking_variable(source, target)?;

                return Ok(Err(variable));
            }
            Verdict::Fails => {}
        }

        // select the target when it is narrower, otherwise preserve both constraints
        let is_top_like = match self.decide_relation(origin, Relation::Subtype, target, source)? {
            Verdict::Holds => true,
            Verdict::Fails => false,
            Verdict::Ambiguous => {
                let variable = self.narrow_blocking_variable(source, target)?;

                return Ok(Err(variable));
            }
        };
        let narrowed = if is_positive && is_top_like {
            target
        } else if is_positive {
            self.normalized_intersection_type([source, target])?
        } else {
            source
        };

        Ok(Ok(narrowed))
    }
}
