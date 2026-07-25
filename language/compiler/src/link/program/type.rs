use destack_artifact as artifact;
use destack_mir as mir;
use destack_program::{
    ElementLayout, FunctionLayoutBuilder, LayoutField, LayoutShapeBuilder, NewtypeLayout,
    ObjectLayoutBuilder, ReferenceFlags, ReferenceLayout, ScalarFormat, SignatureId, SliceLayout,
    TensorDimension, TensorLayoutBuilder, TensorShardingAxis, TensorShardingBuilder,
    TensorViewLayoutBuilder, TypeDescriptorBuilder, TypeId, VariantCaseLayout,
    VariantLayoutBuilder,
};
use destack_source::ModuleId;

use crate::LinkResult;

use super::ProgramLinker;

/// Link MIR types into the program type table.
#[derive(Debug)]
pub(crate) struct TypeLinker<'a> {
    /// Program linker owning final type identities.
    program: &'a ProgramLinker<'a>,
}

/// Program type projection for one emitted object.
#[derive(Debug)]
pub(crate) struct ObjectTypes<'a> {
    /// Module that owns the MIR type identities.
    module: ModuleId,
    /// The object containing module-local type declarations.
    object: &'a artifact::Object,
    /// Program linker owning final type identities.
    program: &'a ProgramLinker<'a>,
}

impl<'a> TypeLinker<'a> {
    /// Create one type linker.
    pub(crate) fn new(program: &'a ProgramLinker<'a>) -> Self {
        Self { program }
    }

    /// Link the program type table.
    pub(crate) fn link(&self) -> LinkResult<Vec<TypeDescriptorBuilder>> {
        let mut descriptors = vec![None; self.program.types_by_id().len()];

        // merge every module-local descriptor and reject conflicts
        for (module, object) in self.program.objects() {
            let types = ObjectTypes::new(*module, object, self.program);
            for object_type in object.types() {
                if !self.program.has_type(*module, object_type.id) {
                    continue;
                }

                let descriptor = types.descriptor(object_type.id);
                let ty = self.program.type_id(*module, object_type.id);
                let entry = &mut descriptors[ty.index()];
                if let Some(previous) = entry {
                    if *previous != descriptor {
                        return Err(self
                            .program
                            .invalid_input(format!("type {ty:?} has conflicting descriptors")));
                    }
                } else {
                    *entry = Some(descriptor);
                }
            }
        }

        // preserve dense type order in the packed table
        let mut entries = Vec::with_capacity(descriptors.len());
        for (index, descriptor) in descriptors.into_iter().enumerate() {
            let ty = TypeId::from(index as u32);
            let descriptor = descriptor.ok_or_else(|| {
                self.program
                    .invalid_input(format!("type {ty:?} has no descriptor"))
            })?;
            entries.push(descriptor);
        }

        Ok(entries)
    }
}

impl<'a> ObjectTypes<'a> {
    /// Create one object type projection.
    pub(crate) fn new(
        module: ModuleId,
        object: &'a artifact::Object,
        program: &'a ProgramLinker<'a>,
    ) -> Self {
        Self {
            module,
            object,
            program,
        }
    }

    /// Return the program pointer byte width.
    pub(crate) const fn pointer_bytes(&self) -> u8 {
        self.object.target().pointer_bytes()
    }

    /// Return one module-local MIR type.
    pub(crate) fn get(&self, ty: mir::TypeId) -> &mir::Type {
        &self
            .object
            .ty(ty)
            .unwrap_or_else(|| unreachable!("missing emitted type {ty:?}"))
            .definition
    }

    /// Project one MIR type into a program type descriptor.
    pub(crate) fn descriptor(&self, type_id: mir::LocalNodeId<mir::Type>) -> TypeDescriptorBuilder {
        let storage_type = self.storage_type(type_id);
        let layout = self.program.layout_id(self.module, storage_type);
        let supertypes = self.supertypes(type_id);
        let drop = self.program.drop_id(self.module, type_id);

        let descriptor = TypeDescriptorBuilder::new(layout).supertypes(supertypes);
        match drop {
            Some(drop) => descriptor.drop(drop),
            None => descriptor,
        }
    }

    /// Return the scalar layout for one MIR type id.
    pub(crate) fn scalar_format(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<ScalarFormat> {
        let ty = self.storage_type(ty);

        self.scalar_layout_node(self.get(ty))
    }

    /// Project one MIR layout shape into a program layout shape.
    pub(crate) fn layout_shape(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        shape: &mir::LayoutShape,
    ) -> Option<LayoutShapeBuilder> {
        let repr = self.storage_type(ty);
        let type_shape = self.get(repr);

        Some(match shape {
            mir::LayoutShape::None => LayoutShapeBuilder::None,
            mir::LayoutShape::Scalar => self.scalar_layout_shape(repr, type_shape)?,
            mir::LayoutShape::Struct(layout) => {
                LayoutShapeBuilder::Struct(self.layout_fields(&layout.fields))
            }
            mir::LayoutShape::Tuple(layout) => {
                LayoutShapeBuilder::Tuple(self.layout_fields(&layout.elements))
            }
            mir::LayoutShape::Slice => self.slice_layout_shape(type_shape)?,
            mir::LayoutShape::Array(layout) => {
                LayoutShapeBuilder::Array(self.element_layout(layout))
            }
            mir::LayoutShape::Vector(layout) => {
                LayoutShapeBuilder::Vector(self.element_layout(layout))
            }
            mir::LayoutShape::Tensor(layout) => {
                let mir::Type::Tensor { space, shape, .. } = type_shape else {
                    return None;
                };
                LayoutShapeBuilder::Tensor(
                    TensorLayoutBuilder::new(
                        *space,
                        self.type_id(layout.element),
                        layout.format,
                        shape.iter().map(Self::tensor_dimension),
                    )
                    .sharding(self.tensor_sharding(&layout.sharding)),
                )
            }
            mir::LayoutShape::TensorView(layout) => {
                self.tensor_view_layout_shape(type_shape, layout)?
            }
            mir::LayoutShape::Variant(layout) => {
                let cases = layout.cases.iter().map(|variant| VariantCaseLayout {
                    discriminant: variant.discriminant,
                    ty: self.type_id(variant.ty),
                    payload_offset: variant.payload_offset,
                });
                LayoutShapeBuilder::Variant(
                    VariantLayoutBuilder::new(
                        self.type_id(layout.discriminant),
                        self.type_id(layout.storage),
                        layout.encoding,
                    )
                    .cases(cases),
                )
            }
            mir::LayoutShape::Object(layout) => {
                let mut object = ObjectLayoutBuilder::new(self.layout_fields(&layout.fields));
                if let Some(dispatch_offset) = layout.dispatch_offset {
                    object = object.dispatch_offset(dispatch_offset);
                }

                LayoutShapeBuilder::Object(object)
            }
            mir::LayoutShape::Dynamic => LayoutShapeBuilder::Dynamic,
            mir::LayoutShape::Function => self.function_layout_shape(type_shape)?,
            mir::LayoutShape::Newtype(layout) => {
                let backing_type = self.type_id(layout.backing_type);
                let backing_layout = self.program.layout_id(self.module, layout.backing_type);

                LayoutShapeBuilder::Newtype(NewtypeLayout {
                    backing_type,
                    backing_layout,
                })
            }
        })
    }

    /// Return the program type id for one MIR type.
    pub(crate) fn type_id(&self, ty: mir::LocalNodeId<mir::Type>) -> TypeId {
        self.program.type_id(self.module, ty)
    }

    /// Return flattened program supertypes for one MIR type.
    fn supertypes(&self, ty: mir::LocalNodeId<mir::Type>) -> Vec<TypeId> {
        let mut supertypes = Vec::new();
        self.append_supertypes(ty, ty, &mut supertypes);

        supertypes
    }

    /// Append transitive parents and interfaces for one MIR type.
    fn append_supertypes(&self, root: mir::TypeId, ty: mir::TypeId, supertypes: &mut Vec<TypeId>) {
        let Some(lineage) = self.object.ty(ty).and_then(|ty| ty.lineage.as_ref()) else {
            return;
        };

        // append the parent hierarchy before implemented interfaces
        if let Some(parent) = lineage.parent {
            self.append_supertype(root, parent, supertypes);
        }

        // append each interface and its own inherited interfaces
        for &interface in &lineage.interfaces {
            self.append_supertype(root, interface, supertypes);
        }
    }

    /// Append one supertype once, followed by its ancestry.
    fn append_supertype(&self, root: mir::TypeId, ty: mir::TypeId, supertypes: &mut Vec<TypeId>) {
        let id = self.type_id(ty);
        if ty == root || supertypes.contains(&id) {
            return;
        }

        supertypes.push(id);
        self.append_supertypes(root, ty, supertypes);
    }

    /// Return the program storage type for one MIR type.
    pub(crate) fn storage_type(
        &self,
        mut ty: mir::LocalNodeId<mir::Type>,
    ) -> mir::LocalNodeId<mir::Type> {
        loop {
            match self.get(ty) {
                mir::Type::WithLifetimes { base, .. }
                | mir::Type::Uninit { value: base }
                | mir::Type::Atomic { value: base }
                | mir::Type::ManuallyDrop { value: base } => {
                    ty = *base;
                }
                _ => return ty,
            }
        }
    }

    /// Return the scalar layout for one MIR type.
    fn scalar_layout_node(&self, ty: &mir::Type) -> Option<ScalarFormat> {
        match ty {
            mir::Type::Int { width, is_signed } => Some(ScalarFormat::int(*width, *is_signed)),
            mir::Type::Isize => Some(ScalarFormat::int(u16::from(self.pointer_bytes()) * 8, true)),
            mir::Type::Usize | mir::Type::TypeDescriptor => Some(ScalarFormat::int(
                u16::from(self.pointer_bytes()) * 8,
                false,
            )),
            mir::Type::TypeId => Some(ScalarFormat::int(u32::BITS as u16, false)),
            mir::Type::Float(float_type) => Some(ScalarFormat::Float {
                format: *float_type,
            }),
            mir::Type::Boolean => Some(ScalarFormat::Boolean),
            mir::Type::Character => Some(ScalarFormat::Character),
            _ => None,
        }
    }

    /// Project one scalar-like MIR type into a program layout shape.
    fn scalar_layout_shape(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        type_shape: &mir::Type,
    ) -> Option<LayoutShapeBuilder> {
        match type_shape {
            mir::Type::Reference {
                kind,
                space,
                access,
                pointee,
                nullability,
                ..
            } => Some(LayoutShapeBuilder::Reference(ReferenceLayout {
                pointee: self.type_id(*pointee),
                flags: ReferenceFlags::new(*kind, *space, *access, *nullability),
            })),
            mir::Type::FunctionPointer { signature } => Some(LayoutShapeBuilder::FunctionPointer(
                self.signature(*signature)?,
            )),
            _ => Some(LayoutShapeBuilder::Scalar(self.scalar_format(ty)?)),
        }
    }

    /// Project one MIR slice type into a program layout shape.
    fn slice_layout_shape(&self, type_shape: &mir::Type) -> Option<LayoutShapeBuilder> {
        let mir::Type::Slice {
            kind,
            space,
            access,
            element,
            nullability,
            ..
        } = type_shape
        else {
            return None;
        };

        Some(LayoutShapeBuilder::Slice(SliceLayout {
            reference: ReferenceLayout {
                pointee: self.type_id(*element),
                flags: ReferenceFlags::new(*kind, *space, *access, *nullability),
            },
        }))
    }

    /// Project one MIR tensor view into its executable descriptor layout.
    fn tensor_view_layout_shape(
        &self,
        type_shape: &mir::Type,
        layout: &mir::TensorViewLayout,
    ) -> Option<LayoutShapeBuilder> {
        let mir::Type::TensorView {
            kind,
            space,
            access,
            element,
            shape,
            nullability,
            ..
        } = type_shape
        else {
            return None;
        };
        let reference = ReferenceLayout {
            pointee: self.type_id(*element),
            flags: ReferenceFlags::new(*kind, *space, *access, *nullability),
        };
        let layout = TensorViewLayoutBuilder::new(
            reference,
            self.type_id(layout.element),
            layout.format,
            shape.iter().map(Self::tensor_dimension),
        )
        .sharding(self.tensor_sharding(&layout.sharding));

        Some(LayoutShapeBuilder::TensorView(layout))
    }

    /// Project one MIR tensor dimension into executable shape metadata.
    fn tensor_dimension(dimension: &mir::TensorDimension) -> TensorDimension {
        match dimension {
            mir::TensorDimension::Static(size) => TensorDimension::fixed(*size),
            mir::TensorDimension::Symbol(_) | mir::TensorDimension::Dynamic => {
                TensorDimension::dynamic()
            }
        }
    }

    /// Project one MIR function type into a program layout shape.
    fn function_layout_shape(&self, type_shape: &mir::Type) -> Option<LayoutShapeBuilder> {
        let mir::Type::Function {
            signature,
            environment,
        } = type_shape
        else {
            return None;
        };

        Some(LayoutShapeBuilder::Function(FunctionLayoutBuilder::new(
            self.signature(*signature)?,
            self.type_id(*environment),
        )))
    }

    /// Project one MIR element layout.
    fn element_layout(&self, layout: &mir::ElementLayout) -> ElementLayout {
        ElementLayout {
            element: self.type_id(layout.element),
            stride: layout.stride,
            count: layout.count,
        }
    }

    /// Project MIR layout fields.
    fn layout_fields(&self, fields: &[mir::LayoutField]) -> Vec<LayoutField> {
        let mut fields = fields
            .iter()
            .map(|field| {
                let layout = LayoutField {
                    name: field.name.into(),
                    ty: self.type_id(field.ty),
                    offset: field.offset,
                    size: field.size,
                    alignment: field.alignment,
                };

                (field.source_index, layout)
            })
            .collect::<Vec<_>>();
        fields.sort_by_key(|(source_index, _)| *source_index);

        fields.into_iter().map(|(_, field)| field).collect()
    }

    /// Project one MIR tensor sharding descriptor.
    fn tensor_sharding(&self, sharding: &mir::TensorSharding) -> TensorShardingBuilder {
        match sharding {
            mir::TensorSharding::Unsharded => TensorShardingBuilder::Unsharded,
            mir::TensorSharding::Sharding { axes } => TensorShardingBuilder::Sharded {
                axes: axes
                    .iter()
                    .map(|axis| self.tensor_sharding_axis(axis))
                    .collect(),
            },
        }
    }

    /// Project one MIR tensor sharding axis.
    fn tensor_sharding_axis(&self, axis: &mir::TensorShardingAxis) -> TensorShardingAxis {
        match axis {
            mir::TensorShardingAxis::Shard { axis } => TensorShardingAxis::Shard { axis: *axis },
            mir::TensorShardingAxis::Replicate => TensorShardingAxis::Replicate,
            mir::TensorShardingAxis::Partial { reduction } => TensorShardingAxis::Partial {
                reduction: *reduction,
            },
        }
    }

    /// Return one program function signature.
    pub(crate) fn signature(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<SignatureId> {
        let ty = self.storage_type(ty);

        self.program.type_signature_id(self.module, ty)
    }
}
