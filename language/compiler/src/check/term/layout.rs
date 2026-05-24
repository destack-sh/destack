use destack_artifact::TargetArch;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckComponentState, FormTerm, GenericSubstitution, ShapeMemberTerm, StaticTerm, TypeTerm,
    VariableId,
};

/// Compile-time query over a concrete type layout.
///
/// ```ts
/// sizeOf<T>()
/// alignOf<T>()
/// strideOf<T>()
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct LayoutTerm {
    /// The source layout query expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The queried type.
    pub(in crate::check) target: VariableId,
    /// The requested layout property.
    pub(in crate::check) query: LayoutQuery,
}

/// Requested layout property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum LayoutQuery {
    /// Total storage size in bytes.
    Size,
    /// Required alignment in bytes.
    Alignment,
    /// Repeated element stride in bytes.
    Stride,
}

/// Solved layout before commit allocates DIR layout ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct ConcreteLayout {
    /// The layout shape.
    pub(in crate::check) shape: ConcreteLayoutShape,
    /// The size in bytes.
    pub(in crate::check) size: Option<u32>,
    /// The alignment in bytes.
    pub(in crate::check) alignment: Option<u32>,
}

/// Solved layout shape before commit allocates DIR layout ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) enum ConcreteLayoutShape {
    /// No runtime storage.
    None,
    /// Builtin scalar storage.
    Scalar,
    /// Pointer-sized erased value storage.
    Dynamic,
    /// Struct or object storage.
    Struct {
        /// The fields in layout order.
        fields: Vec<ConcreteLayoutField>,
    },
    /// Tuple storage.
    Tuple {
        /// The tuple elements in layout order.
        elements: Vec<ConcreteLayoutField>,
    },
    /// Variant value storage.
    Variant {
        /// The variant cases.
        variants: Vec<ConcreteVariantLayout>,
    },
    /// Transparent nominal storage.
    Newtype {
        /// The backing type layout.
        backing: Box<ConcreteLayout>,
    },
    /// Runtime function or closure storage.
    Function,
}

/// Solved field or tuple element layout before commit allocates DIR layout ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct ConcreteLayoutField {
    /// The field key.
    pub(in crate::check) key: Option<dir::StaticKey>,
    /// The field type.
    pub(in crate::check) ty: LayoutType,
    /// The field layout.
    pub(in crate::check) layout: Box<ConcreteLayout>,
    /// The offset in bytes.
    pub(in crate::check) offset: Option<u32>,
    /// The size in bytes.
    pub(in crate::check) size: Option<u32>,
    /// The alignment in bytes.
    pub(in crate::check) alignment: Option<u32>,
}

/// Solved variant case layout before commit allocates DIR layout ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct ConcreteVariantLayout {
    /// The logical case type.
    pub(in crate::check) ty: LayoutType,
    /// The case layout.
    pub(in crate::check) layout: Box<ConcreteLayout>,
}

/// Type identity attached to one solved layout node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum LayoutType {
    /// Check type variable.
    Variable(VariableId),
    /// Committed DIR type id.
    TypeId(dir::LocalTypeId),
}

impl LayoutTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> smallvec::SmallVec<[VariableId; 4]> {
        smallvec::smallvec![self.target]
    }

    /// Substitute generic arguments through this layout term.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        state: &mut CheckComponentState<'_>,
    ) -> CompilerResult<Self> {
        Ok(Self {
            source: self.source,
            target: state.substitute_type_variable(module, substitution, self.target)?,
            query: self.query,
        })
    }
}

impl CheckComponentState<'_> {
    /// Reduce one layout query when its target type is solved.
    pub(in crate::check) fn reduce_layout_static(
        &mut self,
        module: ModuleId,
        term: &LayoutTerm,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let pointer_bytes = self.target_pointer_bytes()?;
        let Some(layout) = self.type_layout(module, term.target, pointer_bytes)? else {
            return Ok(None);
        };
        let Some(value) = term.query.value(&layout) else {
            self.record_layout_rejection(term)?;

            return Ok(None);
        };

        self.record_layout_resolution(term, layout)?;

        Ok(Some(dir::StaticTerm::ScalarLiteral {
            value: dir::ScalarLiteral::Integer(i64::from(value)),
        }))
    }

    /// Return a concrete layout for one solved type variable.
    fn type_layout(
        &mut self,
        module: ModuleId,
        variable: VariableId,
        pointer_bytes: u32,
    ) -> CompilerResult<Option<ConcreteLayout>> {
        let Some(term) = self.solved_type_term(variable)? else {
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
    ) -> CompilerResult<Option<ConcreteLayout>> {
        let layout = match term {
            TypeTerm::Literal(atom) => {
                let ty = atom.to_type();

                self.dir_type_layout(module, &ty, pointer_bytes)?
            }
            TypeTerm::Form { form, payload } => {
                self.form_term_layout(module, form, LayoutType::Variable(*payload), pointer_bytes)?
            }
            TypeTerm::FixedArray {
                element,
                length,
                is_readonly: _,
            } => self.fixed_array_term_layout(module, *element, *length, pointer_bytes)?,
            TypeTerm::Slice {
                element: _,
                is_readonly: _,
            } => Some(scalar_layout(pointer_bytes * 2, pointer_bytes)),
            TypeTerm::Tuple {
                elements,
                is_readonly: _,
                form: _,
            } => {
                let fields = elements.iter().map(|element| LayoutFieldInput {
                    key: None,
                    ty: LayoutType::Variable(element.ty),
                });

                self.aggregate_layout(module, fields, AggregateLayoutShape::Tuple, pointer_bytes)?
            }
            TypeTerm::Shape { members } => self.shape_layout(module, members, pointer_bytes)?,
            TypeTerm::Union { elements } => {
                let variants = elements.iter().copied().map(LayoutType::Variable);

                self.variant_layout(module, variants, pointer_bytes)?
            }
            TypeTerm::Range { .. } => Some(scalar_layout(16, 8)),
            TypeTerm::Function(_) => Some(function_layout(pointer_bytes)),
            TypeTerm::Closure { .. } => Some(function_layout(pointer_bytes)),
            TypeTerm::Dynamic { .. } => None,
            TypeTerm::Variable(variable) => self.type_layout(module, *variable, pointer_bytes)?,
            _ => None,
        };

        Ok(layout)
    }

    /// Return a concrete layout for one committed DIR type.
    fn dir_type_layout(
        &mut self,
        module: ModuleId,
        ty: &dir::Type,
        pointer_bytes: u32,
    ) -> CompilerResult<Option<ConcreteLayout>> {
        let layout = match ty {
            dir::Type::Never | dir::Type::Void | dir::Type::Undefined => Some(none_layout()),
            dir::Type::Any | dir::Type::Unknown | dir::Type::Object => {
                Some(any_layout(pointer_bytes))
            }
            dir::Type::Null => Some(none_layout()),
            dir::Type::Primitive(primitive) => primitive_layout(*primitive, pointer_bytes),
            dir::Type::Literal(literal) => literal_layout(literal),
            dir::Type::Form(form) => self.form_layout(module, form, pointer_bytes)?,
            dir::Type::FixedArray(array) => {
                self.fixed_array_layout(module, array, pointer_bytes)?
            }
            dir::Type::Range(_) => Some(scalar_layout(16, 8)),
            dir::Type::Slice(_) => Some(scalar_layout(pointer_bytes * 2, pointer_bytes)),
            dir::Type::Tuple(tuple) => {
                let fields = tuple.elements.iter().map(|element| LayoutFieldInput {
                    key: None,
                    ty: LayoutType::TypeId(element.ty),
                });

                self.aggregate_layout(module, fields, AggregateLayoutShape::Tuple, pointer_bytes)?
            }
            dir::Type::Shape(shape) => self.dir_shape_layout(module, shape, pointer_bytes)?,
            dir::Type::Function(_) | dir::Type::Closure(_) => Some(function_layout(pointer_bytes)),
            dir::Type::Union(union) => {
                let variants = union.elements.iter().copied().map(LayoutType::TypeId);

                self.variant_layout(module, variants, pointer_bytes)?
            }
            dir::Type::Named(_)
            | dir::Type::Parameter(_)
            | dir::Type::This
            | dir::Type::Dynamic(_)
            | dir::Type::Predicate(_)
            | dir::Type::Operation(_)
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
    ) -> CompilerResult<Option<ConcreteLayout>> {
        let layout = match form.form {
            dir::Form::Managed | dir::Form::Borrowed { .. } | dir::Form::Raw => {
                Some(pointer_layout(pointer_bytes))
            }
            dir::Form::Owned | dir::Form::Placed { .. } | dir::Form::Readonly => {
                let value = self.module(module)?.get_type(form.value);

                self.dir_type_layout(module, &value, pointer_bytes)?
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
    ) -> CompilerResult<Option<ConcreteLayout>> {
        let layout = match form {
            FormTerm::Managed | FormTerm::Borrowed { .. } | FormTerm::Raw => {
                Some(pointer_layout(pointer_bytes))
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
    ) -> CompilerResult<Option<ConcreteLayout>> {
        let element_type = self.module(module)?.get_type(array.element);
        let element = self.dir_type_layout(module, &element_type, pointer_bytes)?;
        let Some(element) = element else {
            return Ok(None);
        };
        let Some(length) = self.static_usize(module, array.count)? else {
            return Ok(None);
        };
        let Some(size) = element.size else {
            return Ok(None);
        };
        let Some(alignment) = element.alignment else {
            return Ok(None);
        };

        Ok(Some(ConcreteLayout {
            shape: ConcreteLayoutShape::Tuple {
                elements: Vec::new(),
            },
            size: Some(size.saturating_mul(length)),
            alignment: Some(alignment),
        }))
    }

    /// Return a concrete layout for one fixed array term.
    fn fixed_array_term_layout(
        &mut self,
        module: ModuleId,
        element: VariableId,
        length: VariableId,
        pointer_bytes: u32,
    ) -> CompilerResult<Option<ConcreteLayout>> {
        let Some(element) = self.type_layout(module, element, pointer_bytes)? else {
            return Ok(None);
        };
        let Some(length) = self.static_usize_variable(length)? else {
            return Ok(None);
        };
        let Some(size) = element.size else {
            return Ok(None);
        };
        let Some(alignment) = element.alignment else {
            return Ok(None);
        };

        Ok(Some(ConcreteLayout {
            shape: ConcreteLayoutShape::Tuple {
                elements: Vec::new(),
            },
            size: Some(size.saturating_mul(length)),
            alignment: Some(alignment),
        }))
    }

    /// Return a concrete layout for one DIR shape.
    fn dir_shape_layout(
        &mut self,
        module: ModuleId,
        shape: &dir::ShapeType,
        pointer_bytes: u32,
    ) -> CompilerResult<Option<ConcreteLayout>> {
        let fields = shape.fields.iter().map(|field| LayoutFieldInput {
            key: Some(field.key),
            ty: LayoutType::TypeId(field.ty),
        });

        self.aggregate_layout(module, fields, AggregateLayoutShape::Struct, pointer_bytes)
    }

    /// Return a concrete layout for one check shape.
    fn shape_layout(
        &mut self,
        module: ModuleId,
        members: &[ShapeMemberTerm],
        pointer_bytes: u32,
    ) -> CompilerResult<Option<ConcreteLayout>> {
        let fields = members.iter().filter_map(|member| match member {
            ShapeMemberTerm::Field { key, ty, .. } => Some(LayoutFieldInput {
                key: Some(*key),
                ty: LayoutType::Variable(*ty),
            }),
            ShapeMemberTerm::CallSignature { .. }
            | ShapeMemberTerm::ConstructSignature { .. }
            | ShapeMemberTerm::IndexSignature { .. } => None,
        });

        self.aggregate_layout(module, fields, AggregateLayoutShape::Struct, pointer_bytes)
    }

    /// Return an aggregate layout for ordered fields.
    fn aggregate_layout(
        &mut self,
        module: ModuleId,
        fields: impl IntoIterator<Item = LayoutFieldInput>,
        shape: AggregateLayoutShape,
        pointer_bytes: u32,
    ) -> CompilerResult<Option<ConcreteLayout>> {
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
            let field_alignment = field_alignment.max(1);
            let field_offset = align_to(offset, field_alignment);

            offset = field_offset.saturating_add(field_size);
            alignment = alignment.max(field_alignment);
            layout_fields.push(ConcreteLayoutField {
                key: field.key,
                ty: field.ty,
                layout: Box::new(layout),
                offset: Some(field_offset),
                size: Some(field_size),
                alignment: Some(field_alignment),
            });
        }
        let shape = match shape {
            AggregateLayoutShape::Struct => ConcreteLayoutShape::Struct {
                fields: layout_fields,
            },
            AggregateLayoutShape::Tuple => ConcreteLayoutShape::Tuple {
                elements: layout_fields,
            },
        };

        Ok(Some(ConcreteLayout {
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
    ) -> CompilerResult<Option<ConcreteLayout>> {
        let mut size = 1;
        let mut alignment = 1;
        let mut variant_layouts = Vec::new();

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

            size = size.max(variant_size);
            alignment = alignment.max(variant_alignment.max(1));
            variant_layouts.push(ConcreteVariantLayout {
                ty,
                layout: Box::new(layout),
            });
        }

        Ok(Some(ConcreteLayout {
            shape: ConcreteLayoutShape::Variant {
                variants: variant_layouts,
            },
            size: Some(align_to(size.saturating_add(1), alignment)),
            alignment: Some(alignment),
        }))
    }

    /// Return one committed static value as a `usize`.
    fn static_usize(
        &self,
        module: ModuleId,
        value: dir::LocalStaticId,
    ) -> CompilerResult<Option<u32>> {
        let value = self.module(module)?.get_static(value);
        let dir::StaticTerm::ScalarLiteral {
            value: dir::ScalarLiteral::Integer(value),
        } = value
        else {
            return Ok(None);
        };

        Ok(u32::try_from(value).ok())
    }

    /// Return one solved static variable as a `usize`.
    fn static_usize_variable(&self, value: VariableId) -> CompilerResult<Option<u32>> {
        let Some(value) = self.solved_static_term(value)? else {
            return Ok(None);
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
    fn target_pointer_bytes(&self) -> CompilerResult<u32> {
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

    /// Record a successful layout query.
    fn record_layout_resolution(
        &mut self,
        term: &LayoutTerm,
        layout: ConcreteLayout,
    ) -> CompilerResult<()> {
        let outcome = crate::check::LayoutOutcome::Resolved(crate::check::LayoutResolution {
            source: term.source,
            target: term.target,
            query: term.query,
            layout,
        });

        self.module_mut(term.source.module_id)?
            .record_layout_outcome(term.source, outcome);

        Ok(())
    }

    /// Record a failed layout query.
    fn record_layout_rejection(&mut self, term: &LayoutTerm) -> CompilerResult<()> {
        let outcome = crate::check::LayoutOutcome::Rejected(crate::check::LayoutFailure {
            source: term.source,
            target: term.target,
            query: term.query,
        });

        self.module_mut(term.source.module_id)?
            .record_layout_outcome(term.source, outcome);

        Ok(())
    }
}

impl LayoutType {
    /// Return the solved layout for this type input.
    fn layout(
        self,
        module: ModuleId,
        state: &mut CheckComponentState<'_>,
        pointer_bytes: u32,
    ) -> CompilerResult<Option<ConcreteLayout>> {
        match self {
            Self::Variable(variable) => state.type_layout(module, variable, pointer_bytes),
            Self::TypeId(ty) => {
                let ty = state.module(module)?.get_type(ty);

                state.dir_type_layout(module, &ty, pointer_bytes)
            }
        }
    }
}

impl LayoutQuery {
    /// Return this query's integer value from a layout.
    fn value(self, layout: &ConcreteLayout) -> Option<u32> {
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

/// Return a storage-free layout.
fn none_layout() -> ConcreteLayout {
    ConcreteLayout {
        shape: ConcreteLayoutShape::None,
        size: Some(0),
        alignment: Some(1),
    }
}

/// Return a pointer layout.
fn pointer_layout(pointer_bytes: u32) -> ConcreteLayout {
    scalar_layout(pointer_bytes, pointer_bytes)
}

/// Return an erased value layout.
fn any_layout(pointer_bytes: u32) -> ConcreteLayout {
    ConcreteLayout {
        shape: ConcreteLayoutShape::Dynamic,
        size: Some(pointer_bytes),
        alignment: Some(pointer_bytes),
    }
}

/// Return a runtime function layout.
fn function_layout(pointer_bytes: u32) -> ConcreteLayout {
    ConcreteLayout {
        shape: ConcreteLayoutShape::Function,
        size: Some(pointer_bytes * 2),
        alignment: Some(pointer_bytes),
    }
}

/// Return a scalar layout.
fn scalar_layout(size: u32, alignment: u32) -> ConcreteLayout {
    ConcreteLayout {
        shape: ConcreteLayoutShape::Scalar,
        size: Some(size),
        alignment: Some(alignment),
    }
}

/// Return a primitive layout when it has a concrete representation.
fn primitive_layout(primitive: dir::PrimitiveType, pointer_bytes: u32) -> Option<ConcreteLayout> {
    let layout = match primitive {
        dir::PrimitiveType::Boolean => scalar_layout(1, 1),
        dir::PrimitiveType::Character => scalar_layout(4, 4),
        dir::PrimitiveType::String | dir::PrimitiveType::Bigint => return None,
        dir::PrimitiveType::Integer(integer) => integer_layout(integer, pointer_bytes),
        dir::PrimitiveType::Float(float) => float_layout(float),
        dir::PrimitiveType::Symbol | dir::PrimitiveType::UniqueSymbol => {
            pointer_layout(pointer_bytes)
        }
    };

    Some(layout)
}

/// Return a literal layout.
fn literal_layout(literal: &dir::ScalarLiteral) -> Option<ConcreteLayout> {
    let layout = match literal {
        dir::ScalarLiteral::Null => none_layout(),
        dir::ScalarLiteral::Boolean(_) => scalar_layout(1, 1),
        dir::ScalarLiteral::Integer(_) => scalar_layout(8, 8),
        dir::ScalarLiteral::Bigint(_) => return None,
        dir::ScalarLiteral::Float(_) => scalar_layout(8, 8),
        dir::ScalarLiteral::Character(_) => scalar_layout(4, 4),
        dir::ScalarLiteral::String(_) | dir::ScalarLiteral::RegexString { .. } => return None,
    };

    Some(layout)
}

/// Return an integer layout.
fn integer_layout(integer: dir::IntegerType, pointer_bytes: u32) -> ConcreteLayout {
    match integer {
        dir::IntegerType::Integer { .. } => scalar_layout(pointer_bytes, pointer_bytes),
        dir::IntegerType::Fixed { width, .. } => {
            let bytes = u32::from(width).div_ceil(8).max(1);

            scalar_layout(bytes, bytes)
        }
        dir::IntegerType::Pointer { .. } => scalar_layout(pointer_bytes, pointer_bytes),
    }
}

/// Return a float layout.
fn float_layout(float: dir::FloatType) -> ConcreteLayout {
    match float {
        dir::FloatType::Float => scalar_layout(8, 8),
        dir::FloatType::Float32 => scalar_layout(4, 4),
        dir::FloatType::Float64 => scalar_layout(8, 8),
    }
}

/// Align a byte offset to an alignment.
fn align_to(value: u32, alignment: u32) -> u32 {
    if alignment <= 1 {
        return value;
    }

    value.div_ceil(alignment) * alignment
}
