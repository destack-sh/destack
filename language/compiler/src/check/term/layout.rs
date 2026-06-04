use destack_artifact::TargetArch;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckState, FormTerm, GenericArgument, LayoutDecision, LayoutFailure, LayoutResolution,
    RepresentationConstraint, ShapeMember, StaticOperand, StaticTerm, Substitution, TypeOperand,
    TypeTerm, VariableId,
};

/// Compile-time query over a concrete type layout.
///
/// ```ds
/// sizeOf<T>()
/// alignOf<T>()
/// strideOf<T>()
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct LayoutTerm {
    /// The source layout query expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The queried type.
    pub(in crate::check) target: TypeOperand,
    /// The requested layout property.
    pub(in crate::check) query: LayoutQuery,
}

/// Requested layout property.
///
/// Examples:
/// ```ds
/// sizeOf<T>()
/// alignOf<T>()
/// strideOf<T>()
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum LayoutQuery {
    /// Total storage size in bytes.
    ///
    /// Examples:
    /// ```ds
    /// sizeOf<T>()
    /// ```
    Size,
    /// Required alignment in bytes.
    ///
    /// Examples:
    /// ```ds
    /// alignOf<T>()
    /// ```
    Alignment,
    /// Repeated element stride in bytes.
    ///
    /// Examples:
    /// ```ds
    /// strideOf<T>()
    /// ```
    Stride,
}

/// Solved layout before commit allocates DIR layout ids.
///
/// Examples:
/// ```ds
/// struct Point { x: int32, y: int32 }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct Layout {
    /// The layout shape.
    pub(in crate::check) shape: LayoutShape,
    /// The size in bytes.
    pub(in crate::check) size: Option<u32>,
    /// The alignment in bytes.
    pub(in crate::check) alignment: Option<u32>,
}

impl Layout {
    /// Return a storage-free layout.
    fn none() -> Self {
        Self {
            shape: LayoutShape::None,
            size: Some(0),
            alignment: Some(1),
        }
    }

    /// Return a pointer layout.
    fn pointer(pointer_bytes: u32) -> Self {
        Self::scalar(pointer_bytes, pointer_bytes)
    }

    /// Return an erased value layout.
    fn dynamic(pointer_bytes: u32) -> Self {
        Self {
            shape: LayoutShape::Dynamic,
            size: Some(pointer_bytes * 2),
            alignment: Some(pointer_bytes),
        }
    }

    /// Return a runtime closure layout.
    fn closure(pointer_bytes: u32) -> Self {
        Self {
            shape: LayoutShape::Closure,
            size: Some(pointer_bytes * 2),
            alignment: Some(pointer_bytes),
        }
    }

    /// Return a scalar layout.
    fn scalar(size: u32, alignment: u32) -> Self {
        Self {
            shape: LayoutShape::Scalar,
            size: Some(size),
            alignment: Some(alignment),
        }
    }

    /// Return a primitive layout when it has a concrete representation.
    fn primitive(primitive: dir::PrimitiveType, pointer_bytes: u32) -> Option<Self> {
        let layout = match primitive {
            dir::PrimitiveType::Boolean => Self::scalar(1, 1),
            dir::PrimitiveType::Character => Self::scalar(4, 4),
            dir::PrimitiveType::String | dir::PrimitiveType::Bigint => return None,
            dir::PrimitiveType::Integer(integer) => Self::integer(integer, pointer_bytes),
            dir::PrimitiveType::Float(float) => Self::float(float),
            dir::PrimitiveType::Symbol | dir::PrimitiveType::UniqueSymbol => {
                Self::pointer(pointer_bytes)
            }
        };

        Some(layout)
    }

    /// Return a literal layout.
    fn literal(literal: &dir::ScalarLiteral) -> Option<Self> {
        let layout = match literal {
            dir::ScalarLiteral::Null => Self::none(),
            dir::ScalarLiteral::Undefined => Self::none(),
            dir::ScalarLiteral::Boolean(_) => Self::scalar(1, 1),
            dir::ScalarLiteral::Integer(_) => Self::scalar(8, 8),
            dir::ScalarLiteral::Bigint(_) => return None,
            dir::ScalarLiteral::Float(_) => Self::scalar(8, 8),
            dir::ScalarLiteral::Character(_) => Self::scalar(4, 4),
            dir::ScalarLiteral::String(_) | dir::ScalarLiteral::RegexString { .. } => {
                return None;
            }
        };

        Some(layout)
    }

    /// Return an integer layout.
    fn integer(integer: dir::IntegerType, pointer_bytes: u32) -> Self {
        match integer {
            dir::IntegerType::Integer { .. } => Self::scalar(pointer_bytes, pointer_bytes),
            dir::IntegerType::Fixed { width, .. } => {
                let bytes = u32::from(width).div_ceil(8).max(1);

                Self::scalar(bytes, bytes)
            }
            dir::IntegerType::Pointer { .. } => Self::scalar(pointer_bytes, pointer_bytes),
        }
    }

    /// Return a float layout.
    fn float(float: dir::FloatType) -> Self {
        match float {
            dir::FloatType::Float => Self::scalar(8, 8),
            dir::FloatType::Float16 | dir::FloatType::Bfloat16 => Self::scalar(2, 2),
            dir::FloatType::Float32 => Self::scalar(4, 4),
            dir::FloatType::Float64 => Self::scalar(8, 8),
        }
    }
}

/// Solved layout shape before commit allocates DIR layout ids.
///
/// Examples:
/// ```ds
/// void
/// int32
/// { x: int32, y: int32 }
/// [int32, int32]
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) enum LayoutShape {
    /// No runtime storage.
    ///
    /// Examples:
    /// ```ds
    /// void
    /// ```
    None,
    /// Builtin scalar storage.
    ///
    /// Examples:
    /// ```ds
    /// int32
    /// ```
    Scalar,
    /// Struct storage.
    ///
    /// Examples:
    /// ```ds
    /// { x: int32, y: int32 }
    /// ```
    Struct(StructLayout),
    /// Tuple storage.
    ///
    /// Examples:
    /// ```ds
    /// [int32, int32]
    /// ```
    Tuple(TupleLayout),
    /// Slice header storage.
    ///
    /// Examples:
    /// ```ds
    /// [int32]
    /// ```
    Slice(SliceLayout),
    /// Array storage.
    ///
    /// Examples:
    /// ```ds
    /// [int32; 4]
    /// ```
    Array(ArrayLayout),
    /// Variant value storage.
    ///
    /// Examples:
    /// ```ds
    /// Option<int32>
    /// ```
    Variant(VariantLayout),
    /// Object storage with a dispatch table header.
    ///
    /// Examples:
    /// ```ds
    /// class Service { value: int32 }
    /// ```
    Object(ObjectLayout),
    /// Pointer-sized erased value storage.
    ///
    /// Examples:
    /// ```ds
    /// Dynamic<T>
    /// ```
    Dynamic,
    /// Runtime closure storage.
    ///
    /// Examples:
    /// ```ds
    /// (value: int32) => int32
    /// ```
    Closure,
    /// Transparent nominal storage.
    ///
    /// Examples:
    /// ```ds
    /// newtype UserId = int64
    /// ```
    Newtype(NewtypeLayout),
}

/// Solved struct layout before commit allocates DIR layout ids.
///
/// Examples:
/// ```ds
/// { x: int32 }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct StructLayout {
    /// The fields in layout order.
    pub(in crate::check) fields: Vec<LayoutField>,
}

/// Solved tuple layout before commit allocates DIR layout ids.
///
/// Examples:
/// ```ds
/// [int32, int32]
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct TupleLayout {
    /// The tuple elements in layout order.
    pub(in crate::check) elements: Vec<LayoutField>,
}

/// Solved slice layout before commit allocates DIR layout ids.
///
/// Examples:
/// ```ds
/// [int32]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct SliceLayout {
    /// The slice element type.
    pub(in crate::check) element: LayoutType,
}

/// Solved array layout before commit allocates DIR layout ids.
///
/// Examples:
/// ```ds
/// [int32; 4]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct ArrayLayout {
    /// The array element type.
    pub(in crate::check) element: LayoutType,
    /// The byte stride between elements.
    pub(in crate::check) stride: Option<u32>,
    /// The fixed element count when known.
    pub(in crate::check) count: Option<u32>,
}

/// Solved variant layout before commit allocates DIR layout ids.
///
/// Examples:
/// ```ds
/// Option<int32>
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct VariantLayout {
    /// The tag layout.
    pub(in crate::check) tag: VariantTagLayout,
    /// The variant payload byte offset.
    pub(in crate::check) payload_offset: Option<u32>,
    /// The variant cases.
    pub(in crate::check) variants: Vec<VariantCaseLayout>,
}

/// Solved variant tag layout before commit allocates DIR layout ids.
///
/// Examples:
/// ```ds
/// @repr(uint8)
/// enum Status { Ready, Done }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct VariantTagLayout {
    /// The tag type when it has been materialized.
    pub(in crate::check) ty: Option<LayoutType>,
    /// The tag size in bytes.
    pub(in crate::check) size: Option<u32>,
    /// The tag alignment in bytes.
    pub(in crate::check) alignment: Option<u32>,
}

/// Solved object layout before commit allocates DIR layout ids.
///
/// Examples:
/// ```ds
/// class Service { value: int32 }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct ObjectLayout {
    /// The fields in layout order.
    pub(in crate::check) fields: Vec<LayoutField>,
}

/// Solved newtype layout before commit allocates DIR layout ids.
///
/// Examples:
/// ```ds
/// newtype UserId = int64
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct NewtypeLayout {
    /// The backing type.
    pub(in crate::check) backing_type: LayoutType,
    /// The backing layout.
    pub(in crate::check) backing_layout: Box<Layout>,
}

/// Solved field or tuple element layout before commit allocates DIR layout ids.
///
/// Examples:
/// ```ds
/// { x: int32 }
/// [int32]
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct LayoutField {
    /// The field key.
    pub(in crate::check) key: Option<dir::StaticKey>,
    /// The field type.
    pub(in crate::check) ty: LayoutType,
    /// The field layout.
    pub(in crate::check) layout: Box<Layout>,
    /// The offset in bytes.
    pub(in crate::check) offset: Option<u32>,
    /// The size in bytes.
    pub(in crate::check) size: Option<u32>,
    /// The alignment in bytes.
    pub(in crate::check) alignment: Option<u32>,
}

/// Solved variant case layout before commit allocates DIR layout ids.
///
/// Examples:
/// ```ds
/// Some(value)
/// None
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct VariantCaseLayout {
    /// The logical case type.
    pub(in crate::check) ty: LayoutType,
    /// The case layout.
    pub(in crate::check) layout: Box<Layout>,
}

/// Type identity attached to one solved layout node.
///
/// Examples:
/// ```ds
/// int32
/// Box<int32>
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum LayoutType {
    /// Check type operand.
    ///
    /// Examples:
    /// ```ds
    /// Box<T>
    /// ```
    Operand(TypeOperand),
    /// Committed type id.
    ///
    /// Examples:
    /// ```ds
    /// int32
    /// ```
    TypeId(dir::GlobalTypeId),
}

impl LayoutTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> smallvec::SmallVec<[VariableId; 2]> {
        self.target.referenced_variables(state)
    }

    /// Substitute generic arguments through this layout term.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: Substitution<'_>,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        Ok(Self {
            source: self.source,
            target: state.substitute_type_operand(module, substitution, self.target)?,
            query: self.query,
        })
    }
}

impl CheckState<'_> {
    /// Return whether one type operand has a concrete layout.
    pub(in crate::check) fn type_operand_concrete(
        &mut self,
        module: ModuleId,
        operand: TypeOperand,
    ) -> CompilerResult<Option<bool>> {
        let Some(term) = self.type_operand_term(operand)? else {
            return Ok(None);
        };
        let pointer_bytes = self.target_pointer_bytes()?;
        let layout = self.type_term_layout(module, &term, pointer_bytes)?;

        Ok(Some(layout.is_some()))
    }

    /// Reduce one layout query when its target type is solved.
    pub(in crate::check) fn reduce_layout_term(
        &mut self,
        module: ModuleId,
        term: &LayoutTerm,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let pointer_bytes = self.target_pointer_bytes()?;
        let Some(layout) = self.type_operand_layout(module, term.target, pointer_bytes)? else {
            return Ok(None);
        };
        let Some(value) = term.query.value(&layout) else {
            self.reject_layout(term)?;

            return Ok(None);
        };

        self.select_layout_resolution(term, layout)?;

        Ok(Some(dir::StaticTerm::ScalarLiteral {
            value: dir::ScalarLiteral::Integer(i64::from(value)),
        }))
    }

    /// Return a concrete layout for one solved type operand.
    pub(in crate::check) fn type_operand_layout(
        &mut self,
        module: ModuleId,
        operand: TypeOperand,
        pointer_bytes: u32,
    ) -> CompilerResult<Option<Layout>> {
        let Some(term) = self.type_operand_term(operand)? else {
            return Ok(None);
        };

        self.type_term_layout(module, &term, pointer_bytes)
    }

    /// Return a concrete layout for one solved type term.
    fn type_term_layout(
        &mut self,
        module: ModuleId,
        term: &TypeTerm,
        pointer_bytes: u32,
    ) -> CompilerResult<Option<Layout>> {
        let layout = match term {
            TypeTerm::Literal(atom) => {
                let ty = atom.to_type();

                self.committed_type_layout(module, &ty, pointer_bytes)?
            }
            TypeTerm::Form { form, payload } => {
                let form = self.inference.term(*form).clone();

                self.form_term_layout(module, &form, LayoutType::Operand(*payload), pointer_bytes)?
            }
            TypeTerm::FixedArray { element, length } => {
                self.fixed_array_term_layout(module, *element, *length, pointer_bytes)?
            }
            TypeTerm::Slice { element } => {
                self.slice_layout(module, LayoutType::Operand(*element), pointer_bytes)?
            }
            TypeTerm::Tuple { elements, form: _ } => {
                let fields = elements
                    .iter()
                    .map(|element| LayoutFieldInput {
                        key: None,
                        ty: LayoutType::Operand(element.ty.into()),
                    })
                    .collect::<Vec<_>>();

                self.aggregate_layout(
                    module,
                    fields.into_iter(),
                    AggregateLayoutShape::Tuple,
                    RepresentationConstraint::default(),
                    pointer_bytes,
                )?
            }
            TypeTerm::Shape(shape) => {
                let members = &self.inference.term(*shape).members;
                let members = members.clone();

                self.shape_layout(module, &members, pointer_bytes)?
            }
            TypeTerm::Union { elements } => {
                let variants = elements.iter().copied().map(LayoutType::Operand);

                self.variant_layout(module, variants, pointer_bytes)?
            }
            TypeTerm::Intrinsic => None,
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } => self.nominal_layout(module, *symbol, arguments, pointer_bytes)?,
            TypeTerm::Range { .. } => Some(Layout::scalar(16, 8)),
            TypeTerm::Function(_) => Some(Layout::closure(pointer_bytes)),
            TypeTerm::Closure { .. } => Some(Layout::closure(pointer_bytes)),
            TypeTerm::Dynamic { .. } => Some(Layout::dynamic(pointer_bytes)),
            _ => None,
        };

        Ok(layout)
    }

    /// Return a concrete layout for one committed type.
    fn committed_type_layout(
        &mut self,
        module: ModuleId,
        ty: &dir::Type,
        pointer_bytes: u32,
    ) -> CompilerResult<Option<Layout>> {
        let layout = match ty {
            dir::Type::Never | dir::Type::Void | dir::Type::Undefined => Some(Layout::none()),
            dir::Type::Any | dir::Type::Unknown | dir::Type::Object => {
                Some(Layout::dynamic(pointer_bytes))
            }
            dir::Type::Null => Some(Layout::none()),
            dir::Type::Primitive(primitive) => Layout::primitive(*primitive, pointer_bytes),
            dir::Type::Literal(literal) => Layout::literal(literal),
            dir::Type::Intrinsic => None,
            dir::Type::Form(form) => self.form_layout(module, form, pointer_bytes)?,
            dir::Type::FixedArray(array) => {
                self.fixed_array_layout(module, array, pointer_bytes)?
            }
            dir::Type::Range(_) => Some(Layout::scalar(16, 8)),
            dir::Type::Slice(slice) => {
                self.slice_layout(module, LayoutType::TypeId(slice.element), pointer_bytes)?
            }
            dir::Type::Tuple(tuple) => {
                let fields = tuple.elements.iter().map(|element| LayoutFieldInput {
                    key: None,
                    ty: LayoutType::TypeId(element.ty),
                });

                self.aggregate_layout(
                    module,
                    fields,
                    AggregateLayoutShape::Tuple,
                    RepresentationConstraint::default(),
                    pointer_bytes,
                )?
            }
            dir::Type::Shape(shape) => {
                let fields = shape.fields.iter().map(|field| LayoutFieldInput {
                    key: Some(field.key),
                    ty: LayoutType::TypeId(field.ty),
                });

                self.aggregate_layout(
                    module,
                    fields,
                    AggregateLayoutShape::Struct,
                    RepresentationConstraint::default(),
                    pointer_bytes,
                )?
            }
            dir::Type::Function(_) | dir::Type::Closure(_) => Some(Layout::closure(pointer_bytes)),
            dir::Type::Union(union) => {
                let variants = union.elements.iter().copied().map(LayoutType::TypeId);

                self.variant_layout(module, variants, pointer_bytes)?
            }
            dir::Type::Dynamic(_) => Some(Layout::dynamic(pointer_bytes)),
            dir::Type::Reference(_)
            | dir::Type::Member(_)
            | dir::Type::Parameter(_)
            | dir::Type::This
            | dir::Type::Operation(_)
            | dir::Type::Array(_)
            | dir::Type::Intersection(_) => None,
            dir::Type::Error => None,
        };

        Ok(layout)
    }

    /// Return a concrete layout for one memory form.
    fn form_layout(
        &mut self,
        module: ModuleId,
        form: &dir::FormType,
        pointer_bytes: u32,
    ) -> CompilerResult<Option<Layout>> {
        let layout = match form.form {
            dir::Form::Managed | dir::Form::Borrowed { .. } | dir::Form::Raw => {
                Some(Layout::pointer(pointer_bytes))
            }
            dir::Form::Owned | dir::Form::Placed { .. } | dir::Form::Readonly => {
                let value = self.layout_type_value(form.value).clone();

                self.committed_type_layout(module, &value, pointer_bytes)?
            }
        };

        Ok(layout)
    }

    /// Return a concrete layout for one memory form term.
    fn form_term_layout(
        &mut self,
        module: ModuleId,
        form: &FormTerm,
        value: LayoutType,
        pointer_bytes: u32,
    ) -> CompilerResult<Option<Layout>> {
        let layout = match form {
            FormTerm::Managed | FormTerm::Borrowed { .. } | FormTerm::Raw => {
                Some(Layout::pointer(pointer_bytes))
            }
            FormTerm::Owned | FormTerm::Placed { .. } | FormTerm::Readonly => {
                value.layout(module, self, pointer_bytes)?
            }
        };

        Ok(layout)
    }

    /// Return a concrete layout for one fixed array.
    fn fixed_array_layout(
        &mut self,
        module: ModuleId,
        array: &dir::FixedArrayType,
        pointer_bytes: u32,
    ) -> CompilerResult<Option<Layout>> {
        let element_type = self.layout_type_value(array.element).clone();
        let element = self.committed_type_layout(module, &element_type, pointer_bytes)?;
        let Some(element) = element else {
            return Ok(None);
        };
        let Some(length) = self.static_usize(array.count)? else {
            return Ok(None);
        };
        let Some(size) = element.size else {
            return Ok(None);
        };
        let Some(alignment) = element.alignment else {
            return Ok(None);
        };
        let stride = align_to(size, alignment);

        Ok(Some(Layout {
            shape: LayoutShape::Array(ArrayLayout {
                element: LayoutType::TypeId(array.element),
                stride: Some(stride),
                count: Some(length),
            }),
            size: Some(stride.saturating_mul(length)),
            alignment: Some(alignment),
        }))
    }

    /// Return a concrete layout for one fixed array term.
    fn fixed_array_term_layout(
        &mut self,
        module: ModuleId,
        element: TypeOperand,
        length: StaticOperand,
        pointer_bytes: u32,
    ) -> CompilerResult<Option<Layout>> {
        let element_operand = element;
        let Some(element_layout) =
            self.type_operand_layout(module, element_operand, pointer_bytes)?
        else {
            return Ok(None);
        };
        let Some(length) = self.static_usize_operand(length)? else {
            return Ok(None);
        };
        let Some(size) = element_layout.size else {
            return Ok(None);
        };
        let Some(alignment) = element_layout.alignment else {
            return Ok(None);
        };
        let stride = align_to(size, alignment);

        Ok(Some(Layout {
            shape: LayoutShape::Array(ArrayLayout {
                element: LayoutType::Operand(element_operand),
                stride: Some(stride),
                count: Some(length),
            }),
            size: Some(stride.saturating_mul(length)),
            alignment: Some(alignment),
        }))
    }

    /// Return a concrete layout for one check shape.
    fn shape_layout(
        &mut self,
        module: ModuleId,
        members: &[ShapeMember],
        pointer_bytes: u32,
    ) -> CompilerResult<Option<Layout>> {
        let fields = members
            .iter()
            .filter_map(|member| match member {
                ShapeMember::Field { key, ty, .. } => Some(LayoutFieldInput {
                    key: Some(key.clone()),
                    ty: LayoutType::Operand(*ty),
                }),
                ShapeMember::Spread { .. }
                | ShapeMember::CallSignature { .. }
                | ShapeMember::ConstructSignature { .. }
                | ShapeMember::IndexSignature { .. } => None,
            })
            .collect::<Vec<_>>();

        self.aggregate_layout(
            module,
            fields.into_iter(),
            AggregateLayoutShape::Struct,
            RepresentationConstraint::default(),
            pointer_bytes,
        )
    }

    /// Return an aggregate layout for ordered fields.
    fn aggregate_layout(
        &mut self,
        module: ModuleId,
        fields: impl IntoIterator<Item = LayoutFieldInput>,
        shape: AggregateLayoutShape,
        constraint: RepresentationConstraint,
        pointer_bytes: u32,
    ) -> CompilerResult<Option<Layout>> {
        let mut offset = 0;
        let mut alignment = 1;
        let mut layout_fields = Vec::new();

        for field in fields {
            let Some(layout) = field.ty.layout(module, self, pointer_bytes)? else {
                return Ok(None);
            };
            let Some(field_size) = layout.size else {
                return Ok(None);
            };
            let Some(field_alignment) = layout.alignment else {
                return Ok(None);
            };
            let field_alignment = constraint.field_alignment(field_alignment);
            let field_offset = align_to(offset, field_alignment);

            offset = field_offset.saturating_add(field_size);
            alignment = alignment.max(field_alignment);
            layout_fields.push(LayoutField {
                key: field.key,
                ty: field.ty,
                layout: Box::new(layout),
                offset: Some(field_offset),
                size: Some(field_size),
                alignment: Some(field_alignment),
            });
        }

        // build output shape
        let shape = match shape {
            AggregateLayoutShape::Struct => LayoutShape::Struct(StructLayout {
                fields: layout_fields,
            }),
            AggregateLayoutShape::Tuple => LayoutShape::Tuple(TupleLayout {
                elements: layout_fields,
            }),
        };

        let alignment = constraint.aggregate_alignment(alignment);

        Ok(Some(Layout {
            shape,
            size: Some(align_to(offset, alignment)),
            alignment: Some(alignment),
        }))
    }

    /// Return a variant layout for union alternatives.
    fn variant_layout(
        &mut self,
        module: ModuleId,
        variants: impl IntoIterator<Item = LayoutType>,
        pointer_bytes: u32,
    ) -> CompilerResult<Option<Layout>> {
        let tag_size = 1;
        let tag_alignment = 1;
        let mut payload_size = 0;
        let mut payload_alignment = 1;
        let mut cases = Vec::<VariantCaseLayout>::new();

        for ty in variants {
            let Some(layout) = ty.layout(module, self, pointer_bytes)? else {
                return Ok(None);
            };
            let Some(variant_size) = layout.size else {
                return Ok(None);
            };
            let Some(variant_alignment) = layout.alignment else {
                return Ok(None);
            };

            payload_size = payload_size.max(variant_size);
            payload_alignment = payload_alignment.max(variant_alignment.max(1));
            cases.push(VariantCaseLayout {
                ty,
                layout: Box::new(layout),
            });
        }
        let payload_offset = align_to(tag_size, payload_alignment);
        let alignment = tag_alignment.max(payload_alignment);
        let size = align_to(payload_offset.saturating_add(payload_size), alignment);

        Ok(Some(Layout {
            shape: LayoutShape::Variant(VariantLayout {
                tag: VariantTagLayout {
                    ty: None,
                    size: Some(tag_size),
                    alignment: Some(tag_alignment),
                },
                payload_offset: Some(payload_offset),
                variants: cases,
            }),
            size: Some(size),
            alignment: Some(alignment),
        }))
    }

    /// Return a concrete layout for one slice header.
    fn slice_layout(
        &mut self,
        _module: ModuleId,
        element: LayoutType,
        pointer_bytes: u32,
    ) -> CompilerResult<Option<Layout>> {
        let size = pointer_bytes * 2;

        Ok(Some(Layout {
            shape: LayoutShape::Slice(SliceLayout { element }),
            size: Some(size),
            alignment: Some(pointer_bytes),
        }))
    }

    /// Return a concrete layout for one nominal reference.
    fn nominal_layout(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        pointer_bytes: u32,
    ) -> CompilerResult<Option<Layout>> {
        if let Some(layout) = self.newtype_layout(module, symbol, arguments, pointer_bytes)? {
            return Ok(Some(layout));
        }

        self.struct_layout(module, symbol, pointer_bytes)
    }

    /// Return a concrete layout for one newtype reference.
    fn newtype_layout(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        pointer_bytes: u32,
    ) -> CompilerResult<Option<Layout>> {
        let Some(value) = self.newtype_backing(module, symbol)? else {
            return Ok(None);
        };
        let substitution = self.generic_substitution(symbol, arguments)?;
        let backing = if substitution.is_empty() {
            value
        } else {
            self.substitute_type_operand(module, &substitution, value)?
        };
        let Some(backing_layout) = self.type_operand_layout(module, backing, pointer_bytes)? else {
            return Ok(None);
        };
        let backing_type = LayoutType::Operand(backing);

        Ok(Some(Layout {
            shape: LayoutShape::Newtype(NewtypeLayout {
                backing_type,
                backing_layout: Box::new(backing_layout.clone()),
            }),
            size: backing_layout.size,
            alignment: backing_layout.alignment,
        }))
    }

    /// Return a concrete layout for one struct reference.
    fn struct_layout(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        pointer_bytes: u32,
    ) -> CompilerResult<Option<Layout>> {
        let Some(fields) = self.nominal_fields(module, symbol)? else {
            return Ok(None);
        };
        let fields = fields.into_iter().map(|(key, ty)| LayoutFieldInput {
            key: Some(key),
            ty: LayoutType::Operand(ty),
        });

        self.aggregate_layout(
            module,
            fields,
            AggregateLayoutShape::Struct,
            RepresentationConstraint::default(),
            pointer_bytes,
        )
    }

    /// Return one committed static value as a `usize`.
    fn static_usize(&self, value: dir::GlobalStaticId) -> CompilerResult<Option<u32>> {
        let value = self.layout_static_value(value);
        let dir::StaticTerm::ScalarLiteral {
            value: dir::ScalarLiteral::Integer(value),
        } = value
        else {
            return Ok(None);
        };

        Ok(u32::try_from(*value).ok())
    }

    /// Return one committed type for layout reduction.
    fn layout_type_value(&self, ty: dir::GlobalTypeId) -> &dir::Type {
        self.r#type(ty)
    }

    /// Return one committed static value for layout reduction.
    fn layout_static_value(&self, value: dir::GlobalStaticId) -> &dir::StaticTerm {
        self.r#static(value)
    }

    /// Return one static operand as a `usize`.
    fn static_usize_operand(&self, value: StaticOperand) -> CompilerResult<Option<u32>> {
        let value = match value {
            StaticOperand::Variable(variable) => {
                let Some(value) = self.static_solution(variable)? else {
                    return Ok(None);
                };

                value
            }
            StaticOperand::Term(term) => self.inference.term(term).clone(),
            StaticOperand::Static(value) => {
                StaticTerm::Literal(self.layout_static_value(value).clone())
            }
        };
        let StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
            value: dir::ScalarLiteral::Integer(value),
        }) = value
        else {
            return Ok(None);
        };

        Ok(u32::try_from(value).ok())
    }

    /// Return the target pointer size.
    pub(in crate::check) fn target_pointer_bytes(&self) -> CompilerResult<u32> {
        let profile = self
            .compiler
            .profile(self.context.revision(), self.profile)?;
        let bytes = match profile.key.target_arch {
            Some(TargetArch::X86)
            | Some(TargetArch::Armv7)
            | Some(TargetArch::Armv6)
            | Some(TargetArch::Riscv32)
            | Some(TargetArch::Wasm32) => 4,
            Some(TargetArch::X86_64)
            | Some(TargetArch::Aarch64)
            | Some(TargetArch::Riscv64)
            | Some(TargetArch::PowerPc64)
            | Some(TargetArch::PowerPc64le)
            | Some(TargetArch::S390x)
            | Some(TargetArch::Mips64)
            | Some(TargetArch::Mips64el)
            | Some(TargetArch::LoongArch64)
            | Some(TargetArch::Wasm64)
            | Some(TargetArch::Other(_))
            | None => 8,
        };

        Ok(bytes)
    }

    /// Select a successful layout query.
    fn select_layout_resolution(
        &mut self,
        term: &LayoutTerm,
        layout: Layout,
    ) -> CompilerResult<()> {
        let decision = LayoutDecision::Resolved(LayoutResolution {
            source: term.source,
            target: term.target,
            query: term.query,
            layout,
        });

        self.select_layout(term.source, decision)?;

        Ok(())
    }

    /// Select a failed layout query.
    fn reject_layout(&mut self, term: &LayoutTerm) -> CompilerResult<()> {
        let decision = LayoutDecision::Rejected(LayoutFailure {
            source: term.source,
            target: term.target,
            query: term.query,
        });

        self.select_layout(term.source, decision)?;

        Ok(())
    }
}

impl LayoutType {
    /// Return the solved layout for this type input.
    fn layout(
        self,
        module: ModuleId,
        state: &mut CheckState<'_>,
        pointer_bytes: u32,
    ) -> CompilerResult<Option<Layout>> {
        match self {
            Self::Operand(operand) => state.type_operand_layout(module, operand, pointer_bytes),
            Self::TypeId(ty) => {
                let ty = state.layout_type_value(ty).clone();

                state.committed_type_layout(module, &ty, pointer_bytes)
            }
        }
    }
}

impl LayoutQuery {
    /// Return this query's integer value from a layout.
    fn value(self, layout: &Layout) -> Option<u32> {
        match self {
            Self::Size => layout.size,
            Self::Alignment => layout.alignment,
            Self::Stride => {
                let size = layout.size?;
                let alignment = layout.alignment?;

                Some(align_to(size, alignment))
            }
        }
    }
}

/// Field input used by aggregate layout construction.
struct LayoutFieldInput {
    /// The field key.
    key: Option<dir::StaticKey>,
    /// The field type.
    ty: LayoutType,
}

/// Aggregate layout shape selected by source type syntax.
enum AggregateLayoutShape {
    /// Struct or object storage.
    Struct,
    /// Tuple storage.
    Tuple,
}

/// Align a byte offset to an alignment.
fn align_to(value: u32, alignment: u32) -> u32 {
    if alignment <= 1 {
        return value;
    }

    value.div_ceil(alignment) * alignment
}
