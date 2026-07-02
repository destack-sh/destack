use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Dependency, Origin};

/// Working accumulator for merging shape elements of an intersection.
/// Payload lists stay as plain vectors until the merged shape is interned.
#[derive(Default)]
struct ShapeMerge {
    /// The merged fields.
    fields: Vec<dir::TypeField>,
    /// The merged call signatures.
    call_signatures: Vec<dir::GlobalTypeId>,
    /// The merged construct signatures.
    construct_signatures: Vec<dir::GlobalTypeId>,
    /// The merged index signatures.
    index_signatures: Vec<dir::TypeIndexSignature>,
}

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
        let mut merged: Option<ShapeMerge> = None;
        let mut others = Vec::new();
        let mut shape_count = 0usize;
        for element in closed {
            let dir::Type::Shape(shape) = self.ty(element)? else {
                others.push(element);
                continue;
            };

            shape_count += 1;
            self.merge_intersection_shape(origin, &mut merged, element.module_id, shape)?;
        }

        // keep intersections symbolic unless two or more shapes contributed
        let (Some(merged), 2..) = (merged, shape_count) else {
            return Ok(Answer::Ready(id));
        };
        let module = origin.module();
        let fields = self.intern_fields(module, &merged.fields)?;
        let call_signatures = self.intern_type_ids(module, &merged.call_signatures)?;
        let construct_signatures = self.intern_type_ids(module, &merged.construct_signatures)?;
        let index_signatures = self.intern_index_signatures(module, &merged.index_signatures)?;
        let shape = self.intern_type(
            module,
            dir::Type::Shape(dir::ShapeType {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            }),
        )?;
        if others.is_empty() {
            return Ok(Answer::Ready(shape));
        }

        let mut elements = vec![shape];
        elements.extend(others);
        let elements = self.intern_type_ids(module, &elements)?;
        let rebuilt = self.intern_type(
            module,
            dir::Type::Intersection(dir::IntersectionType { elements }),
        )?;

        Ok(Answer::Ready(rebuilt))
    }

    /// Merge one shape into an intersection shape accumulator.
    fn merge_intersection_shape(
        &mut self,
        origin: Origin,
        merged: &mut Option<ShapeMerge>,
        module: destack_source::ModuleId,
        shape: dir::ShapeType,
    ) -> CompilerResult<()> {
        let fields = self.shape_fields(module, shape.fields)?.to_vec();
        let call_signatures = self.type_ids(module, shape.call_signatures)?.to_vec();
        let construct_signatures = self.type_ids(module, shape.construct_signatures)?.to_vec();
        let index_signatures = self
            .shape_index_signatures(module, shape.index_signatures)?
            .to_vec();

        let Some(merged) = merged.as_mut() else {
            *merged = Some(ShapeMerge {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            });

            return Ok(());
        };

        for field in fields {
            let Some(index) = merged
                .fields
                .iter()
                .position(|merged| merged.key == field.key)
            else {
                merged.fields.push(field);
                continue;
            };

            // intersect shared keys and keep stricter field attributes
            let shared = merged.fields[index];
            if shared.ty != field.ty {
                let elements = self.intern_type_ids(origin.module(), &[shared.ty, field.ty])?;
                merged.fields[index].ty = self.intern_type(
                    origin.module(),
                    dir::Type::Intersection(dir::IntersectionType { elements }),
                )?;
            }
            merged.fields[index].is_optional &= field.is_optional;
            merged.fields[index].is_readonly |= field.is_readonly;
        }

        merged.call_signatures.extend(call_signatures);
        merged.construct_signatures.extend(construct_signatures);
        merged.index_signatures.extend(index_signatures);

        Ok(())
    }
}
