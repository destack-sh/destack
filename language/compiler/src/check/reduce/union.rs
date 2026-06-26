use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::CheckState;

impl CheckState<'_> {
    /// Return a normalized union type.
    pub(in crate::check) fn normalized_union_type(
        &mut self,
        module: ModuleId,
        elements: impl IntoIterator<Item = dir::GlobalTypeId>,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let elements = self.union_elements(elements)?;

        match elements.as_slice() {
            [single] => Ok(*single),
            _ => self.push_type(
                module,
                dir::Type::Union(dir::UnionType {
                    elements: elements.into_iter().collect(),
                }),
                source,
            ),
        }
    }

    /// Return flattened and deduplicated union elements.
    fn union_elements(
        &mut self,
        elements: impl IntoIterator<Item = dir::GlobalTypeId>,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 4]>> {
        let mut kept = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for element in elements {
            let element = self.settled_root(element)?;

            // flatten nested unions into one element list
            let elements = match self.ty(element)? {
                dir::Type::Union(union) => {
                    union.elements.iter().copied().collect::<SmallVec<[_; 4]>>()
                }
                _ => SmallVec::from_slice(&[element]),
            };

            // keep one representative for each equal type
            for element in elements {
                if !self.union_contains(&kept, element)? {
                    kept.push(element);
                }
            }
        }

        Ok(kept)
    }

    /// Return whether a union element list already contains one type.
    fn union_contains(
        &self,
        kept: &[dir::GlobalTypeId],
        element: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        for candidate in kept {
            if self.ty(*candidate)? == self.ty(element)? {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
