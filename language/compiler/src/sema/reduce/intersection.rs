use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{CheckState, Origin, Relation};
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
        // collect the elements this intersection merges, each once
        let mut closed = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for element in elements {
            let element = self.shallow_resolve(*element)?;
            let head = self.structurally_normalize(origin, element)?;
            if let dir::Type::Intersection(nested) = self.ty(head)? {
                let nested = self.type_ids(head.module_id, nested.elements)?.to_vec();
                for nested in nested {
                    let nested = self.shallow_resolve(nested)?;
                    if !closed.contains(&nested) {
                        closed.push(nested);
                    }
                }

                continue;
            }
            if !closed.contains(&element) {
                closed.push(element);
            }
        }

        // distribute the intersection over one union element at a time
        for (index, element) in closed.iter().enumerate() {
            let head = self.structurally_normalize(origin, *element)?;
            let dir::Type::Union(union) = self.ty(head)? else {
                continue;
            };
            let arms = self.type_ids(head.module_id, union.elements)?.to_vec();
            let mut distributed = Vec::with_capacity(arms.len());
            for arm in arms {
                let mut arm_elements = closed.clone();
                arm_elements[index] = arm;
                let list = self.intern_type_ids(&arm_elements)?;
                let sub = self.intern_type(dir::Type::Intersection(dir::IntersectionType {
                    elements: list,
                }))?;
                distributed.push(self.reduce_intersection(origin, sub, &arm_elements)?);
            }

            return self.normalized_union_type(distributed);
        }

        // annihilate disjoint scalars and absorb literals into their primitive
        let mut scalar: Option<(dir::GlobalTypeId, bool)> = None;
        for element in &closed {
            let family = match self.ty(*element)? {
                dir::Type::Primitive(_) => Some(false),
                dir::Type::Literal(_) => Some(true),
                _ => None,
            };
            let Some(is_literal) = family else {
                continue;
            };
            let Some((kept, kept_is_literal)) = scalar else {
                scalar = Some((*element, is_literal));
                continue;
            };

            // keep the scalar that the other one covers
            let (narrow, wide) = match (kept_is_literal, is_literal) {
                (true, false) => (kept, *element),
                (false, true) => (*element, kept),
                _ if kept == *element => continue,
                _ => {
                    return self.intern_type(dir::Type::Never);
                }
            };
            let verdict = self.evaluate_relation(origin, Relation::Assignable, narrow, wide)?;
            if !verdict.holds() {
                return self.intern_type(dir::Type::Never);
            }
            scalar = Some((narrow, true));
        }
        if let Some((kept, _)) = scalar {
            let mut narrowed = SmallVec::<[dir::GlobalTypeId; 4]>::new();
            for element in closed {
                let is_scalar = matches!(
                    self.ty(element)?,
                    dir::Type::Primitive(_) | dir::Type::Literal(_)
                );
                if !is_scalar || element == kept {
                    narrowed.push(element);
                }
            }
            closed = narrowed;

            // annihilate a scalar met by a nominal class, struct or enum
            for element in &closed {
                let head = self.structurally_normalize(origin, *element)?;
                let dir::Type::Application(application) = self.ty(head)? else {
                    continue;
                };
                if matches!(
                    self.definition(application.symbol)?,
                    Some(
                        dir::Definition::Class(_)
                            | dir::Definition::Struct(_)
                            | dir::Definition::Enum(_)
                    )
                ) {
                    return self.intern_type(dir::Type::Never);
                }
            }
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
            // resolve each element to the shape it names
            let head = self.structurally_normalize(origin, element)?;
            let dir::Type::Object(shape) = self.ty(head)? else {
                others.push(element);
                continue;
            };

            shape_count += 1;
            self.merge_intersection_shape(origin, &mut merged, head.module_id, shape)?;
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
        origin: Origin,
        left: Option<dir::GlobalTypeId>,
        right: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        Ok(match (left, right) {
            (Some(left), Some(right)) if left != right => {
                let interned = self.normalized_intersection_type([left, right])?;
                let reduced = match self.ty(interned)? {
                    dir::Type::Intersection(intersection) => {
                        let elements = self
                            .type_ids(interned.module_id, intersection.elements)?
                            .to_vec();

                        self.reduce_intersection(origin, interned, &elements)?
                    }
                    _ => interned,
                };

                Some(reduced)
            }
            (left, right) => left.or(right),
        })
    }
}
