use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{CheckState, Origin};

/// One associated type projected from a tried value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum TryProjection {
    /// The value produced when evaluation continues.
    Output,
    /// The value propagated when evaluation stops.
    Residual,
    /// The error a `catch` binds when evaluation stops.
    Failure,
}

impl CheckState<'_> {
    /// Return whether one type holds only nullish members.
    pub(in crate::sema) fn is_nullish_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let root = self.normalize(origin, ty)?;
        let elements = match self.union_leaves(origin, root)? {
            Some(leaves) => leaves,
            None => SmallVec::from_slice(&[root]),
        };
        for element in elements {
            let resolved = self.normalize(origin, element)?;
            if !matches!(
                self.ty(resolved)?,
                dir::Type::Null
                    | dir::Type::Undefined
                    | dir::Type::Never
                    | dir::Type::Literal(dir::Literal::Null | dir::Literal::Undefined)
            ) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Project the success or residual type of one tried value.
    pub(super) fn reduce_try_projection(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        projection: TryProjection,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // resolve the solved operand before splitting its arms
        let value = self.shallow_resolve(value)?;

        // split nullish members from the remaining value arms, exposing aliased unions
        let mut nullish = Vec::new();
        let mut values = Vec::new();
        let split = self.normalize(origin, value)?;
        if matches!(
            self.ty(split)?,
            dir::Type::Variable(_) | dir::Type::Parameter(_)
        ) {
            return Ok(None);
        }
        let elements = match self.union_leaves(origin, split)? {
            Some(leaves) => leaves,
            None => SmallVec::from_slice(&[value]),
        };
        for element in elements {
            let resolved = self.normalize(origin, element)?;
            match self.ty(resolved)? {
                dir::Type::Null
                | dir::Type::Undefined
                | dir::Type::Literal(dir::Literal::Null | dir::Literal::Undefined) => {
                    nullish.push(element)
                }
                _ => values.push(element),
            }
        }

        // select the requested member from each Try implementation
        let name = match projection {
            TryProjection::Output => "Output",
            TryProjection::Residual => "Residual",
            TryProjection::Failure => "Failure",
        };
        let key = dir::StaticKey::Name(self.strings().intern(name));

        // project each value through its Try implementation
        let mut projected = Vec::new();
        for value in values {
            let selected = self.select_language_protocol_member(
                origin,
                value,
                value,
                dir::MemberSpace::Static,
                key,
                dir::LanguageItem::Try,
                &[],
                &[],
            )?;
            match selected {
                Some((_, member)) => projected.push(member.ty),
                None if projection == TryProjection::Output => projected.push(value),
                None => {}
            }
        }

        // join the projected type with propagated nullish values
        let mut elements = Vec::new();
        if projection != TryProjection::Output {
            elements.extend(nullish);
        }
        elements.extend(projected);
        let joined = match elements.as_slice() {
            [] => self.intern_type(dir::Type::Never)?,
            [single] => *single,
            _ => self.normalized_union_type(elements)?,
        };

        Ok(Some(joined))
    }
}
