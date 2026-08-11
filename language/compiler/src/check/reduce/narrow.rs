use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{CheckState, GenericParameterId, Origin, Relation};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Narrow one receiver through a successful static property-membership test.
    pub(in crate::check) fn narrow_membership_receiver(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let receiver = self.normalize(origin, receiver)?;

        // filter union alternatives independently by their declared member sets
        if let dir::Type::Union(union) = self.ty(receiver)? {
            let elements = self.type_ids(receiver.module_id, union.elements)?.to_vec();
            let mut narrowed = Vec::with_capacity(elements.len());
            for element in elements {
                let element = self.narrow_membership_receiver(origin, element, key)?;
                if !matches!(self.ty(element)?, dir::Type::Never) {
                    narrowed.push(element);
                }
            }

            return self.normalized_union_type(narrowed);
        }

        // declared members preserve the receiver exactly
        let lookup = self.body().lookup_inherent_member(
            origin,
            origin.module(),
            receiver,
            dir::MemberSpace::Instance,
            key,
        )?;
        if self.body().member_read_type(origin, &lookup)?.is_some() {
            return Ok(receiver);
        }

        // closed member sets prove that the successful branch is unreachable
        if !self.may_have_additional_member(origin, receiver, key)? {
            let never = self.intern_type(dir::Type::Never)?;

            return Ok(never);
        }

        // open member sets retain the receiver and the property established at runtime
        let unknown = self.intern_type(dir::Type::Unknown)?;
        let member = self.field_shape_type(key, unknown)?;
        let member_is_narrower =
            self.decide_relation(origin, Relation::Satisfies, member, receiver)?;
        let narrowed = if member_is_narrower {
            member
        } else {
            self.normalized_intersection_type([receiver, member])?
        };

        Ok(narrowed)
    }

    /// Filter the declared alternatives of one type through a runtime predicate.
    pub(in crate::check) fn narrow_type_alternatives(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        is_positive: bool,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let source = self.normalize(origin, source)?;
        let alternatives = match self.variant_types(origin.module(), source)? {
            Some(variants) => variants,
            None => match self.ty(source)? {
                dir::Type::Union(union) => {
                    self.type_ids(source.module_id, union.elements)?.to_vec()
                }
                _ => return Ok(None),
            },
        };

        // keep original alternatives whose projected predicate remains inhabited
        let mut kept = Vec::with_capacity(alternatives.len());
        for alternative in alternatives {
            let narrowed = self.narrow_element(origin, alternative, target, is_positive)?;
            let narrowed = self.normalize(origin, narrowed)?;
            if !matches!(self.ty(narrowed)?, dir::Type::Never) {
                kept.push(alternative);
            }
        }

        let narrowed = match kept.as_slice() {
            [] => self.intern_type(dir::Type::Never)?,
            [single] => *single,
            _ => self.normalized_union_type(kept)?,
        };

        Ok(Some(narrowed))
    }

    /// Return the finite runtime domain of one rigid parameter.
    pub(in crate::check) fn parameter_domain(
        &mut self,
        origin: Origin,
        parameter: GenericParameterId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // select one finite bound that enumerates the domain
        let bounds = self.parameter_bounds(origin, parameter)?;
        let mut enumerated = None;
        for bound in &bounds {
            let bound = self.deeply_normalize(origin, *bound)?;
            if self.union_arms(origin, bound)?.is_some() {
                enumerated = Some(bound);

                break;
            }
        }
        let Some(enumerated) = enumerated else {
            return Ok(None);
        };

        // intersect the enumerated alternatives with every active bound
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
    pub(in crate::check) fn reduce_narrowing(
        &mut self,
        origin: Origin,
        narrow: dir::NarrowType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let source = self.normalize(origin, narrow.source)?;
        let target = self.normalize(origin, narrow.target)?;

        // preserve precise case identity for enum and Tagged owners
        let variants = self.variant_types(origin.module(), source)?;

        // untagged newtypes narrow through their substituted runtime backing
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
                dir::Type::Union(union) => {
                    SmallVec::<[_; 4]>::from_slice(self.type_ids(source.module_id, union.elements)?)
                }
                dir::Type::Variable(_) | dir::Type::Parameter(_) => SmallVec::from_slice(&[source]),
                // open operations cannot be distributed over
                dir::Type::Operation(operation) => {
                    let variables = self.type_variables(source)?;
                    if variables.is_empty() {
                        // closed operations expand before narrowing distributes
                        let expanded = self.deeply_normalize(origin, source)?;
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

                        // irreducible template patterns narrow like single arms
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

        let mut kept = Vec::with_capacity(elements.len());

        // filter each arm through the guard relation
        for element in elements {
            let narrowed = self.narrow_element(origin, element, target, narrow.is_positive)?;
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

    /// Narrow one source arm through one runtime target.
    fn narrow_element(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        is_positive: bool,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let module = origin.module();

        // narrow an erased value through its checked runtime domain
        if let dir::Type::Dynamic(dynamic) = self.ty(source)? {
            let constraint = dynamic.constraint;
            let narrowed_constraint =
                self.narrow_element(origin, constraint, target, is_positive)?;
            let narrowed = if matches!(self.ty(narrowed_constraint)?, dir::Type::Never) {
                narrowed_constraint
            } else if narrowed_constraint == constraint || !is_positive {
                source
            } else {
                narrowed_constraint
            };

            return Ok(narrowed);
        }

        // disjoint arms can be decided without assignability
        if !self.types_may_overlap(origin, source, target)? {
            let narrowed = if is_positive {
                self.intern_type(dir::Type::Never)?
            } else {
                source
            };

            return Ok(narrowed);
        }

        // keep or remove the source arm on an exact match
        if self.decide_relation(origin, Relation::Subtype, source, target)? {
            let narrowed = if is_positive {
                source
            } else {
                self.intern_type(dir::Type::Never)?
            };

            return Ok(narrowed);
        }

        // unmatched Tagged variants expose their backing to structural predicates
        if let dir::Type::Variant(variant) = self.ty(source)?
            && let Some(backing) = self.tagged_variant_backing(module, &variant)?
        {
            let narrowed = self.narrow_element(origin, backing, target, is_positive)?;
            let narrowed = if matches!(self.ty(narrowed)?, dir::Type::Never) {
                narrowed
            } else if narrowed == backing {
                source
            } else {
                self.normalized_intersection_type([source, narrowed])?
            };

            return Ok(narrowed);
        }

        // select the target when it is narrower, otherwise preserve both constraints
        let is_top_like = self.decide_relation(origin, Relation::Subtype, target, source)?;
        let narrowed = if is_positive && is_top_like {
            target
        } else if is_positive {
            self.normalized_intersection_type([source, target])?
        } else {
            source
        };

        Ok(narrowed)
    }
}
