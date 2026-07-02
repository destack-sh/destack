use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Relation, answer};

impl CheckState<'_> {
    /// Evaluate one runtime guard narrowing.
    pub(super) fn reduce_narrow(
        &mut self,
        origin: Origin,
        narrow: dir::NarrowType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let source = answer!(self.reduce_type_head(origin, narrow.source)?);
        let target = answer!(self.reduce_type_head(origin, narrow.target)?);

        // distribute over union-valued sources
        let elements = match self.ty(source)? {
            dir::Type::Union(union) => {
                SmallVec::<[_; 4]>::from_slice(self.type_ids(source.module_id, union.elements)?)
            }
            dir::Type::Variable(_) | dir::Type::Parameter(_) => return Ok(Answer::Ready(None)),
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
        let narrowed = if is_positive && is_top_like {
            target
        } else if is_positive {
            self.intern_type(module, dir::Type::Never)?
        } else {
            source
        };

        Ok(Answer::Ready(narrowed))
    }
}
