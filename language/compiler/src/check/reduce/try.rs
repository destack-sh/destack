use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, answer};

/// One associated type projected from a tried value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum TryProjection {
    /// The value produced when evaluation continues.
    Output,
    /// The value propagated when evaluation stops.
    Residual,
}

impl CheckState<'_> {
    /// Project the success or residual type of one tried value.
    pub(super) fn reduce_try_projection(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        projection: TryProjection,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let value = answer!(self.reduce_type_head(origin, value)?);

        // split nullish members from the remaining value arms
        let mut nullish = Vec::new();
        let mut values = Vec::new();
        let elements = match self.ty(value)? {
            dir::Type::Union(union) => {
                SmallVec::<[_; 4]>::from_slice(self.type_ids(value.module_id, union.elements)?)
            }
            dir::Type::Variable(_) | dir::Type::Parameter(_) => return Ok(Answer::Ready(None)),
            _ => SmallVec::from_slice(&[value]),
        };
        for element in elements {
            match self.ty(element)? {
                dir::Type::Null
                | dir::Type::Undefined
                | dir::Type::Literal(dir::ScalarLiteral::Null | dir::ScalarLiteral::Undefined) => {
                    nullish.push(element)
                }
                _ => values.push(element),
            }
        }

        // select the requested member from each Try implementation
        let name = match projection {
            TryProjection::Output => "Output",
            TryProjection::Residual => "Residual",
        };
        let key = dir::StaticKey::Name(self.strings().intern(name));

        // project each value through its Try implementation
        let mut projected = Vec::new();
        for value in values {
            let selected = answer!(self.body().select_language_protocol_member(
                origin,
                value,
                value,
                dir::MemberSpace::Static,
                key,
                dir::LanguageItem::Try,
                &[],
                &[],
            )?);
            match selected {
                Some((_, member)) => projected.push(member.ty),
                None if projection == TryProjection::Output => projected.push(value),
                None => {}
            }
        }

        // join the projected type with propagated nullish values
        let mut elements = Vec::new();
        if projection == TryProjection::Residual {
            elements.extend(nullish);
        }
        elements.extend(projected);
        let joined = match elements.as_slice() {
            [] => self.intern_type(dir::Type::Never)?,
            [single] => *single,
            _ => self.normalized_union_type(elements)?,
        };

        Ok(Answer::Ready(Some(joined)))
    }
}
