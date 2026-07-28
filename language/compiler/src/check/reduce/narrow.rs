use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Dependency, Origin, Relation, answer};

impl CheckState<'_> {
    /// Narrow one receiver through a successful static property-membership test.
    pub(in crate::check) fn narrow_membership_receiver(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let receiver = answer!(self.reduce_type_head(origin, receiver)?);

        // filter union alternatives independently by their declared member sets
        if let dir::Type::Union(union) = self.ty(receiver)? {
            let elements = self.type_ids(receiver.module_id, union.elements)?.to_vec();
            let mut narrowed = Vec::with_capacity(elements.len());
            for element in elements {
                let element = answer!(self.narrow_membership_receiver(origin, element, key)?);
                if !matches!(self.ty(element)?, dir::Type::Never) {
                    narrowed.push(element);
                }
            }

            return Ok(Answer::Ready(
                self.normalized_union_type(origin.module(), narrowed)?,
            ));
        }

        // declared members preserve the receiver exactly
        let lookup = answer!(self.body().lookup_inherent_member(
            origin,
            origin.module(),
            receiver,
            dir::MemberSpace::Instance,
            key,
        )?);
        if self.body().member_read_type(origin, &lookup)?.is_some() {
            return Ok(Answer::Ready(receiver));
        }

        // closed member sets prove that the successful branch is unreachable
        if !answer!(self.may_have_additional_member(origin, receiver, key)?) {
            let never = self.intern_type(origin.module(), dir::Type::Never)?;

            return Ok(Answer::Ready(never));
        }

        // open member sets retain the receiver and the property established at runtime
        let unknown = self.intern_type(origin.module(), dir::Type::Unknown)?;
        let member = self.field_shape_type(origin.module(), key, unknown)?;
        let member_is_narrower =
            answer!(self.decide_relation(origin, Relation::Satisfies, member, receiver)?);
        let narrowed = if member_is_narrower {
            member
        } else {
            self.normalized_intersection_type(origin.module(), [receiver, member])?
        };

        Ok(Answer::Ready(narrowed))
    }

    /// Filter the declared alternatives of one type through a runtime predicate.
    pub(in crate::check) fn narrow_type_alternatives(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        is_positive: bool,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let source = answer!(self.reduce_type_head(origin, source)?);
        let alternatives = match self.variant_types(origin.module(), source)? {
            Some(variants) => variants,
            None => match self.ty(source)? {
                dir::Type::Union(union) => {
                    self.type_ids(source.module_id, union.elements)?.to_vec()
                }
                _ => return Ok(Answer::Ready(None)),
            },
        };

        // keep original alternatives whose projected predicate remains inhabited
        let mut kept = Vec::with_capacity(alternatives.len());
        for alternative in alternatives {
            let narrowed =
                answer!(self.narrow_element(origin, alternative, target, is_positive,)?);
            let narrowed = answer!(self.reduce_type_head(origin, narrowed)?);
            if !matches!(self.ty(narrowed)?, dir::Type::Never) {
                kept.push(alternative);
            }
        }

        let narrowed = match kept.as_slice() {
            [] => self.intern_type(origin.module(), dir::Type::Never)?,
            [single] => *single,
            _ => self.normalized_union_type(origin.module(), kept)?,
        };

        Ok(Answer::Ready(Some(narrowed)))
    }

    /// Evaluate one runtime guard narrowing.
    pub(super) fn reduce_narrowing(
        &mut self,
        origin: Origin,
        narrow: dir::NarrowType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let source = answer!(self.reduce_type_head(origin, narrow.source)?);
        let target = answer!(self.reduce_type_head(origin, narrow.target)?);

        // preserve precise case identity for enum and Tagged owners
        let variants = self.variant_types(origin.module(), source)?;

        // untagged newtypes narrow through their substituted runtime backing
        if variants.is_none()
            && let Some(instance) = self.decompose_newtype(origin, source)?
        {
            let backing = answer!(self.reduce_type_head(origin, instance.backing)?);

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
            let value = answer!(self.strip_form(origin, source)?);
            let target_value = answer!(self.strip_form(origin, target)?);
            let operation = self.intern_operation(
                origin.module(),
                dir::TypeOperation::Narrow(dir::NarrowType {
                    source: value,
                    target: target_value,
                    is_positive: narrow.is_positive,
                }),
            )?;
            let narrowed = answer!(self.reduce_type_head(origin, operation)?);
            if matches!(self.ty(narrowed)?, dir::Type::Operation(_)) {
                return Ok(Answer::Ready(None));
            }
            if matches!(self.ty(narrowed)?, dir::Type::Never) {
                return Ok(Answer::Ready(Some(narrowed)));
            }
            let rebuilt = answer!(self.replace_form_value(origin, source, narrowed)?);

            return Ok(Answer::Ready(Some(rebuilt)));
        }

        // distribute over variant and union valued sources
        let elements = match variants {
            Some(variants) => SmallVec::from_vec(variants),
            None => match self.ty(source)? {
                dir::Type::Union(union) => {
                    SmallVec::<[_; 4]>::from_slice(self.type_ids(source.module_id, union.elements)?)
                }
                dir::Type::Variable(_) | dir::Type::Parameter(_) => SmallVec::from_slice(&[source]),
                // stuck operations wait for their blocking variables
                dir::Type::Operation(operation) => {
                    let variables = self.type_variables(source)?;
                    if variables.is_empty() {
                        // closed operations expand before narrowing distributes
                        let expanded = answer!(self.reduce_type(origin, source)?);
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
                            return Ok(Answer::Ready(None));
                        }
                    } else {
                        return Ok(Answer::pending(
                            variables.into_iter().map(Dependency::Variable),
                        ));
                    }
                }
                _ => SmallVec::from_slice(&[source]),
            },
        };

        let module = origin.module();
        let mut kept = Vec::with_capacity(elements.len());

        // filter each arm through the guard relation
        for element in elements {
            let narrowed =
                answer!(self.narrow_element(origin, element, target, narrow.is_positive)?);
            let narrowed = answer!(self.reduce_type_head(origin, narrowed)?);
            if matches!(self.ty(narrowed)?, dir::Type::Never) {
                continue;
            }
            if !kept.contains(&narrowed) {
                kept.push(narrowed);
            }
        }

        // rebuild the filtered result
        let joined = match kept.as_slice() {
            [] => self.intern_type(module, dir::Type::Never)?,
            [single] => *single,
            _ => self.normalized_union_type(module, kept)?,
        };

        Ok(Answer::Ready(Some(joined)))
    }

    /// Narrow one source arm through one runtime target.
    fn narrow_element(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        is_positive: bool,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let module = origin.module();

        // narrow an erased value through its checked runtime domain
        if let dir::Type::Dynamic(dynamic) = self.ty(source)? {
            let constraint = dynamic.constraint;
            let narrowed_constraint =
                answer!(self.narrow_element(origin, constraint, target, is_positive)?);
            let narrowed = if matches!(self.ty(narrowed_constraint)?, dir::Type::Never) {
                narrowed_constraint
            } else if narrowed_constraint == constraint || !is_positive {
                source
            } else {
                narrowed_constraint
            };

            return Ok(Answer::Ready(narrowed));
        }

        // disjoint arms can be decided without assignability
        if !answer!(self.types_may_overlap(origin, source, target)?) {
            let narrowed = if is_positive {
                self.intern_type(module, dir::Type::Never)?
            } else {
                source
            };

            return Ok(Answer::Ready(narrowed));
        }

        // exact matches keep or remove the source arm; narrowing asks a
        //  constraint question, so reads stay per use and never move values
        if answer!(self.decide_relation(origin, Relation::Subtype, source, target)?) {
            let narrowed = if is_positive {
                source
            } else {
                self.intern_type(module, dir::Type::Never)?
            };

            return Ok(Answer::Ready(narrowed));
        }

        // unmatched Tagged variants expose their backing to structural predicates
        if let dir::Type::Variant(variant) = self.ty(source)?
            && let Some(backing) = self.tagged_variant_backing(module, &variant)?
        {
            let narrowed = answer!(self.narrow_element(origin, backing, target, is_positive)?);
            let narrowed = if matches!(self.ty(narrowed)?, dir::Type::Never) {
                narrowed
            } else if narrowed == backing {
                source
            } else {
                self.normalized_intersection_type(module, [source, narrowed])?
            };

            return Ok(Answer::Ready(narrowed));
        }

        // select the target when it is narrower, otherwise preserve both constraints
        let is_top_like =
            answer!(self.decide_relation(origin, Relation::Subtype, target, source)?);
        let narrowed = if is_positive && is_top_like {
            target
        } else if is_positive {
            self.normalized_intersection_type(module, [source, target])?
        } else {
            source
        };

        Ok(Answer::Ready(narrowed))
    }
}
