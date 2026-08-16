use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{CheckState, Origin};
use crate::{CompilerError, CompilerResult};

/// Working accumulator for merging shape elements of an intersection.
#[derive(Default)]
struct ShapeMerge {
    /// The merged fields.
    fields: Vec<dir::TypeProperty>,
    /// The merged call signatures.
    call_signatures: Vec<dir::GlobalTypeId>,
    /// The merged construct signatures.
    construct_signatures: Vec<dir::GlobalTypeId>,
    /// The merged index signatures.
    index_signatures: Vec<dir::TypeIndexSignature>,
}

impl CheckState<'_> {
    /// Return a flattened intersection type without redundant `unknown` elements.
    pub(in crate::sema) fn normalized_intersection_type(
        &mut self,
        elements: impl IntoIterator<Item = dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut kept = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for element in elements {
            let element = self.shallow_resolve(element)?;

            // flatten nested intersections into one element list
            let elements = match self.ty(element)? {
                dir::Type::Intersection(intersection) => SmallVec::<[_; 4]>::from_slice(
                    self.type_ids(element.module_id, intersection.elements)?,
                ),
                _ => SmallVec::from_slice(&[element]),
            };

            // keep each element once, skipping unknown
            for element in elements {
                let is_unknown = matches!(self.ty(element)?, dir::Type::Unknown);
                if !is_unknown && !kept.contains(&element) {
                    kept.push(element);
                }
            }
        }

        match kept.as_slice() {
            [] => self.intern_type(dir::Type::Unknown),
            [single] => Ok(*single),
            _ => {
                let elements = self.intern_type_ids(&kept)?;

                self.intern_type(dir::Type::Intersection(dir::IntersectionType { elements }))
            }
        }
    }

    /// Merge one intersection's structural shape elements.
    pub(in crate::sema) fn reduce_intersection(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        elements: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // collect the elements this intersection merges, resolving solved spellings
        let mut closed = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for element in elements {
            closed.push(self.shallow_resolve(*element)?);
        }

        // exact key members absorb the string primitive
        let has_exact_key = closed.iter().any(|element| {
            matches!(
                self.ty(*element),
                Ok(dir::Type::Key(_) | dir::Type::Literal(dir::ScalarLiteral::String(_)))
            )
        });
        if has_exact_key {
            closed.retain(|element| {
                !matches!(
                    self.ty(*element),
                    Ok(dir::Type::Primitive(dir::PrimitiveType::String))
                )
            });
        }

        // reduce one element intersection to that element
        if let [single] = closed.as_slice() {
            return Ok(*single);
        }

        // merge structural shapes and keep every other element symbolic
        let mut merged: Option<ShapeMerge> = None;
        let mut others = Vec::new();
        let mut shape_count = 0usize;
        for element in closed {
            let dir::Type::Object(shape) = self.ty(element)? else {
                others.push(element);
                continue;
            };

            shape_count += 1;
            self.merge_intersection_shape(origin, &mut merged, element.module_id, shape)?;
        }

        // keep intersections symbolic unless two or more shapes contributed
        let (Some(merged), 2..) = (merged, shape_count) else {
            return Ok(id);
        };
        let fields = self.intern_properties(&merged.fields)?;
        let call_signatures = self.intern_type_ids(&merged.call_signatures)?;
        let construct_signatures = self.intern_type_ids(&merged.construct_signatures)?;
        let index_signatures = self.intern_index_signatures(&merged.index_signatures)?;
        let shape = self.intern_type(dir::Type::Object(dir::ShapeType {
            properties: fields,
            call_signatures,
            construct_signatures,
            index_signatures,
        }))?;
        if others.is_empty() {
            return Ok(shape);
        }

        let mut elements = vec![shape];
        elements.extend(others);
        let elements = self.intern_type_ids(&elements)?;
        let rebuilt =
            self.intern_type(dir::Type::Intersection(dir::IntersectionType { elements }))?;

        Ok(rebuilt)
    }

    /// Merge one shape into an intersection shape accumulator.
    fn merge_intersection_shape(
        &mut self,
        origin: Origin,
        merged: &mut Option<ShapeMerge>,
        module: ModuleId,
        shape: dir::ShapeType,
    ) -> CompilerResult<()> {
        let fields = self.shape_properties(module, shape.properties)?.to_vec();
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

            // intersect shared keys and keep stricter property attributes
            let shared = merged.fields[index];
            let read =
                self.intersect_property_slot(origin, shared.access.read(), field.access.read())?;
            let write = match shared.access.is_writable() && field.access.is_writable() {
                true => self.intersect_property_slot(
                    origin,
                    shared.access.write(),
                    field.access.write(),
                )?,
                false => None,
            };
            merged.fields[index].access = match (read, write) {
                (Some(read), Some(write)) => dir::PropertyAccess::ReadWrite { read, write },
                (Some(read), None) => dir::PropertyAccess::Read(read),
                (None, Some(write)) => dir::PropertyAccess::Write(write),
                (None, None) => {
                    return Err(CompilerError::Internal {
                        message: format!("intersected property {:?} exposes no access", shared.key),
                    });
                }
            };
            merged.fields[index].is_optional &= field.is_optional;
        }

        merged.call_signatures.extend(call_signatures);
        merged.construct_signatures.extend(construct_signatures);
        merged.index_signatures.extend(index_signatures);

        Ok(())
    }
    /// Intersect one shared property slot pair.
    fn intersect_property_slot(
        &mut self,
        _origin: Origin,
        left: Option<dir::GlobalTypeId>,
        right: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        Ok(match (left, right) {
            (Some(left), Some(right)) if left != right => {
                Some(self.normalized_intersection_type([left, right])?)
            }
            (left, right) => left.or(right),
        })
    }
}
