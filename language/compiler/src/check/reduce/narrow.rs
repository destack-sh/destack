use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Dependency, Origin, Relation, answer};

impl CheckState<'_> {
    /// Evaluate one runtime guard narrowing.
    pub(super) fn reduce_narrow(
        &mut self,
        origin: Origin,
        narrow: dir::NarrowType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let source = answer!(self.reduce_type_head(origin, narrow.source)?);
        let target = answer!(self.reduce_type_head(origin, narrow.target)?);

        // narrow newtypes through their backing representation
        let mut source = source;
        if matches!(self.ty(source)?, dir::Type::Instance(_))
            && let Some(backing) = answer!(self.newtype_backing(origin, source)?)
        {
            source = answer!(self.reduce_type_head(origin, backing)?);
        }

        // narrow the payload beneath memory forms, then rebuild the forms
        if matches!(self.ty(source)?, dir::Type::Form(_)) {
            let value = answer!(self.value_beneath_forms(origin, source)?);
            let target_value = answer!(self.value_beneath_forms(origin, target)?);
            let operation = self.intern_type(
                origin.module(),
                dir::Type::Operation(dir::TypeOperation::Narrow(dir::NarrowType {
                    source: value,
                    target: target_value,
                    is_positive: narrow.is_positive,
                })),
            )?;
            let narrowed = answer!(self.reduce_type_head(origin, operation)?);
            if matches!(self.ty(narrowed)?, dir::Type::Operation(_)) {
                return Ok(Answer::Ready(None));
            }
            if matches!(self.ty(narrowed)?, dir::Type::Never) {
                return Ok(Answer::Ready(Some(narrowed)));
            }
            let rebuilt = answer!(self.replace_beneath_forms(origin, source, narrowed)?);

            return Ok(Answer::Ready(Some(rebuilt)));
        }

        // distribute over union-valued sources
        let elements = match self.ty(source)? {
            dir::Type::Union(union) => {
                SmallVec::<[_; 4]>::from_slice(self.type_ids(source.module_id, union.elements)?)
            }
            dir::Type::Variable(_) | dir::Type::Parameter(_) => SmallVec::from_slice(&[source]),
            // stuck operations wait for their blocking variables
            dir::Type::Operation(_) => {
                let variables = self.type_variables(source)?;
                if variables.is_empty() {
                    return Ok(Answer::Ready(None));
                }

                return Ok(Answer::pending(
                    variables.into_iter().map(Dependency::Variable),
                ));
            }
            _ => SmallVec::from_slice(&[source]),
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

        // erased values expose the checked target on matching branches
        if matches!(self.ty(source)?, dir::Type::Dynamic(_)) {
            let narrowed = if is_positive { target } else { source };

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

        // exact matches keep or remove the source arm
        if answer!(self.decide_relation(origin, Relation::Assignable, source, target)?) {
            let narrowed = if is_positive {
                source
            } else {
                self.intern_type(module, dir::Type::Never)?
            };

            return Ok(Answer::Ready(narrowed));
        }

        // top-like source arms take the target on matching branches
        let is_top_like =
            answer!(self.decide_relation(origin, Relation::Assignable, target, source)?);
        let can_preserve_intersection = self.can_preserve_intersection_narrowing(source, target)?;
        let narrowed = if is_positive && is_top_like {
            target
        } else if is_positive && can_preserve_intersection {
            self.normalized_intersection_type(module, [source, target])?
        } else if is_positive {
            self.intern_type(module, dir::Type::Never)?
        } else {
            source
        };

        Ok(Answer::Ready(narrowed))
    }

    /// Return whether positive narrowing may need to keep both relation sides.
    fn can_preserve_intersection_narrowing(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let source_type = self.ty(source)?;
        let target_type = self.ty(target)?;

        Ok(matches!(
            source_type,
            dir::Type::Parameter(_) | dir::Type::Erased(_) | dir::Type::Variable(_)
        ) || matches!(
            target_type,
            dir::Type::Parameter(_) | dir::Type::Erased(_) | dir::Type::Variable(_)
        ))
    }
}
