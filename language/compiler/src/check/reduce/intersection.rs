use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Dependency, Origin};

impl CheckState<'_> {
    /// Merge one intersection's structural shape elements.
    ///
    /// Shared field keys intersect their types, required fields and readonly forms win, and
    /// non-shape elements stay intersected.
    /// Returns the unchanged root while fewer than two elements are shapes.
    pub(in crate::check) fn reduce_intersection(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        elements: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let mut closed = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for element in elements {
            match self.reduce_type_head(origin, *element)? {
                Answer::Ready(element) => closed.push(element),
                Answer::Pending(dependencies) => blockers.extend(dependencies),
            }
        }
        if !blockers.is_empty() {
            return Ok(Answer::pending(blockers));
        }

        // reduce one element intersection to that element
        if let [single] = closed.as_slice() {
            return Ok(Answer::Ready(*single));
        }

        // merge structural shapes and keep every other element symbolic
        let mut merged: Option<dir::ShapeType> = None;
        let mut others = Vec::new();
        let mut shape_count = 0usize;
        for element in closed {
            let dir::Type::Shape(shape) = self.ty(element)? else {
                others.push(element);
                continue;
            };

            shape_count += 1;
            self.merge_intersection_shape(origin, &mut merged, shape.clone())?;
        }

        // keep intersections symbolic unless two or more shapes contributed
        let (Some(merged), 2..) = (merged, shape_count) else {
            return Ok(Answer::Ready(id));
        };
        let source = self.origin_source_node(origin)?;
        let shape = self.push_type(origin.module(), dir::Type::Shape(merged), source)?;
        if others.is_empty() {
            return Ok(Answer::Ready(shape));
        }

        let mut elements = vec![shape];
        elements.extend(others);
        let rebuilt = self.push_type(
            origin.module(),
            dir::Type::Intersection(dir::IntersectionType { elements }),
            source,
        )?;

        Ok(Answer::Ready(rebuilt))
    }

    /// Merge one shape into an intersection shape accumulator.
    fn merge_intersection_shape(
        &mut self,
        origin: Origin,
        merged: &mut Option<dir::ShapeType>,
        shape: dir::ShapeType,
    ) -> CompilerResult<()> {
        let Some(merged) = merged.as_mut() else {
            *merged = Some(shape);

            return Ok(());
        };

        for field in shape.fields {
            let Some(shared) = merged
                .fields
                .iter_mut()
                .find(|merged| merged.key == field.key)
            else {
                merged.fields.push(field);
                continue;
            };

            // intersect shared keys and keep stricter field attributes
            if shared.ty != field.ty {
                shared.ty = self.push_type(
                    origin.module(),
                    dir::Type::Intersection(dir::IntersectionType {
                        elements: vec![shared.ty, field.ty],
                    }),
                    self.origin_source_node(origin)?,
                )?;
            }
            shared.is_optional &= field.is_optional;
            shared.is_readonly |= field.is_readonly;
        }

        merged.call_signatures.extend(shape.call_signatures);
        merged
            .construct_signatures
            .extend(shape.construct_signatures);
        merged.index_signatures.extend(shape.index_signatures);

        Ok(())
    }
}
