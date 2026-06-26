use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckError, CheckState, Origin, answer};

use super::aggregate::AggregateLayout;

impl CheckState<'_> {
    /// Check that one type at a representation slot has a layout.
    pub(in crate::check) fn check_layout(
        &mut self,
        source: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<destack_artifact::DiagnosticBuilder<CheckError>>>> {
        let origin = Origin::Node(source);

        if answer!(self.layout_of(origin, ty)?).is_some() {
            return Ok(Answer::Ready(None));
        }

        let ty = self.format_type(ty);
        let (module, anchor) = self.source_anchor(source);
        let error = CheckError::LayoutNotConcrete { anchor, module, ty };

        Ok(Answer::Ready(Some(error.into())))
    }

    /// Compute and memoize the layout of one type.
    /// Returns ready none when the type has no concrete representation.
    pub(in crate::check) fn layout_of(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::LocalLayoutId>>> {
        LayoutQuery::new(self, origin).layout_id(ty)
    }
}

/// One recursive layout query.
pub(super) struct LayoutQuery<'state, 'check> {
    /// The check state being queried.
    pub(super) check: &'state mut CheckState<'check>,
    /// The source origin for diagnostics and substitutions.
    pub(super) origin: Origin,
    /// The active recursion chain.
    pub(super) active: IndexSet<dir::GlobalTypeId>,
}

impl<'state, 'check> LayoutQuery<'state, 'check> {
    /// Create one layout query.
    fn new(check: &'state mut CheckState<'check>, origin: Origin) -> Self {
        Self {
            check,
            origin,
            active: IndexSet::new(),
        }
    }

    /// Compute one layout with the active recursion chain tracked.
    ///
    /// Inline recursion has no finite representation and reads as no layout (and errors).
    pub(super) fn layout_id(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::LocalLayoutId>>> {
        // close the represented type first
        let origin = self.origin;
        let ty = answer!(self.check.reduce_type_root(origin, ty)?);
        let ty = self.represented_type(ty)?;
        let segment = ty.module_id;

        // reuse memoized layouts
        if let Some(working) = self.check.layouts.get(&segment)
            && let Some(id) = working.layout_id_for_type(ty)
        {
            return Ok(Answer::Ready(Some(id)));
        }

        // inline recursion has no finite representation
        if !self.active.insert(ty) {
            return Ok(Answer::Ready(None));
        }
        let layout = self.compute_layout(ty, ty);
        self.active.swap_remove(&ty);

        let Some(layout) = answer!(layout?) else {
            return Ok(Answer::Ready(None));
        };

        let working = self.layout_segment(segment);
        let id = working.insert_layout(layout);
        working.set_type_layout(ty, id);

        Ok(Answer::Ready(Some(id)))
    }

    /// Return the layout segment for one module.
    pub(super) fn layout_segment(&mut self, module: ModuleId) -> &mut dir::LayoutSegment {
        self.check
            .layouts
            .entry(module)
            .or_insert_with(|| dir::LayoutSegment::new(module))
    }

    /// Compute the layout of one reduced type.
    ///
    /// The qualified root keeps the outer memory forms so `this` binds
    /// the instantiated form inside the laid out declaration.
    fn compute_layout(
        &mut self,
        ty: dir::GlobalTypeId,
        qualified: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::Layout>>> {
        let pointer_bytes = self.target_pointer_bytes()?;

        match self.check.ty(ty)? {
            // single-valued types store nothing standalone
            dir::Type::Never | dir::Type::Void | dir::Type::Undefined | dir::Type::Null => {
                Ok(Answer::Ready(Some(dir::Layout::unit())))
            }
            // erased values carry a dispatch header
            dir::Type::Any | dir::Type::Unknown | dir::Type::Object | dir::Type::Dynamic(_) => {
                Ok(Answer::Ready(Some(dir::Layout::dynamic(pointer_bytes))))
            }
            dir::Type::Primitive(primitive) => Ok(Answer::Ready(primitive.layout(pointer_bytes))),
            dir::Type::Literal(literal) => Ok(Answer::Ready(literal.layout())),
            dir::Type::Range(_) => Ok(Answer::Ready(Some(dir::Layout::scalar(16, 8, None)))),
            dir::Type::Function(_) => Ok(Answer::Ready(Some(dir::Layout::function(pointer_bytes)))),
            dir::Type::FunctionPointer(_) => Ok(Answer::Ready(Some(dir::Layout::pointer(
                pointer_bytes,
                true,
            )))),
            // indirect forms are pointers, direct forms keep their payload
            dir::Type::Form(form) => {
                let (form, value) = (form.form, form.value);

                match form {
                    dir::Form::Managed | dir::Form::Borrowed { .. } => Ok(Answer::Ready(Some(
                        dir::Layout::pointer_slot(value, pointer_bytes, true),
                    ))),
                    dir::Form::Raw => Ok(Answer::Ready(Some(dir::Layout::pointer_slot(
                        value,
                        pointer_bytes,
                        false,
                    )))),
                    // direct forms keep the qualified root for `this`
                    dir::Form::Owned | dir::Form::Placed { .. } | dir::Form::Readonly => {
                        let value = self.represented_type(value)?;

                        self.compute_layout(value, qualified)
                    }
                }
            }
            dir::Type::FixedArray(array) => {
                let (element, count) = (array.element, array.count);

                self.fixed_array_layout(element, count)
            }
            dir::Type::Slice(_) => Ok(Answer::Ready(Some(dir::Layout {
                shape: dir::LayoutShape::Slice,
                size: pointer_bytes * 2,
                alignment: pointer_bytes,
                niche: Some(dir::Niche {
                    offset: 0,
                    width: pointer_bytes,
                    start: 1,
                    end: dir::Niche::scalar_max(pointer_bytes),
                }),
            }))),
            dir::Type::Tuple(tuple) => {
                let fields = tuple
                    .elements
                    .iter()
                    .map(|element| (None, element.ty))
                    .collect::<SmallVec<[_; 4]>>();

                self.aggregate_layout(&fields, AggregateLayout::Tuple)
            }
            dir::Type::Shape(shape) => {
                let fields = shape
                    .fields
                    .iter()
                    .map(|field| (Some(field.key), field.ty))
                    .collect::<SmallVec<[_; 4]>>();

                self.aggregate_layout(&fields, AggregateLayout::Struct)
            }
            dir::Type::EnumMember(member) => self.compute_layout(member.owner, qualified),
            dir::Type::Union(union) => {
                let elements = union.elements.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.union_layout(&elements)
            }
            dir::Type::Instance(instance) => {
                let instance = instance.clone();

                self.reference_layout(ty, qualified, &instance)
            }
            // open or symbolic types have no representation yet
            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Return the type that owns representation for one reduced type.
    pub(super) fn represented_type(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let dir::Type::Primitive(primitive) = self.check.ty(ty)? else {
            return Ok(ty);
        };

        // primitive aliases defer representation to their language item
        if let Some(item) = primitive.representation_item() {
            let symbol = self.check.language_symbol(item);
            let reference = dir::Type::Instance(dir::GenericInstance {
                symbol,
                arguments: Vec::new(),
            });
            let origin = self.origin;
            let source = self.check.origin_source_node(origin)?;

            return self.check.push_type(origin.module(), reference, source);
        }

        Ok(ty)
    }
}
