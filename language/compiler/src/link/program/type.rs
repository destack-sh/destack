use std::collections::HashMap;

use destack_mir as mir;
use destack_program::{
    AddressSpace, CellLayout, ElementLayout, FunctionLayout, LayoutField, LayoutId, LayoutShape,
    NewtypeLayout, ObjectLayout, ReferenceFlags, ReferenceLayout, ScalarFormat, Signature,
    SliceLayout, StructLayout, TensorLayout, TensorViewLayout, TupleLayout, TypeDescriptor, TypeId,
    TypeTable, VariantCaseLayout, VariantLayout, VariantTagLayout,
};

use crate::LinkResult;

use super::ProgramLinker;

/// Link MIR types into executable type descriptors.
#[derive(Debug)]
pub(crate) struct TypeLinker<'a> {
    /// The MIR tree being linked.
    tree: &'a mir::Tree,
    /// Target ABI layout.
    target_layout: &'a mir::TargetLayout,
    /// Canonical MIR type table.
    types: &'a mir::TypeTable,
    /// Dense executable id projection for this program.
    program: &'a ProgramLinker,
}

impl<'a> TypeLinker<'a> {
    /// Create one type linker.
    pub(crate) fn new(
        tree: &'a mir::Tree,
        target_layout: &'a mir::TargetLayout,
        types: &'a mir::TypeTable,
        program: &'a ProgramLinker,
    ) -> Self {
        Self {
            tree,
            target_layout,
            types,
            program,
        }
    }

    /// Return the program pointer byte width.
    pub(crate) const fn pointer_bytes(&self) -> u8 {
        self.target_layout.pointer_bytes()
    }

    /// Link the executable type table.
    pub(crate) fn link(
        &self,
        layout_ids: &HashMap<mir::TypeId, LayoutId>,
    ) -> LinkResult<TypeTable> {
        let mut descriptors = Vec::new();

        // project MIR type descriptors into dense program ids
        for (type_id, ty) in self.tree.iter_nodes::<mir::Type>() {
            let program_type = self.type_id(type_id);
            let descriptor = self.descriptor(program_type, type_id, ty, layout_ids)?;

            debug_assert_eq!(program_type.index(), descriptors.len());
            descriptors.push(descriptor);
        }

        Ok(TypeTable::new(descriptors))
    }

    /// Project one MIR type into an executable type descriptor.
    pub(crate) fn descriptor(
        &self,
        program_type: TypeId,
        type_id: mir::LocalNodeId<mir::Type>,
        ty: &mir::Type,
        layout_ids: &HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
    ) -> LinkResult<TypeDescriptor> {
        let repr = self.repr_type(program_type, ty);
        let layout = self.executable_layout_id(repr, layout_ids)?;
        let supertypes = self.supertypes(type_id);

        Ok(TypeDescriptor {
            repr,
            layout,
            supertypes,
        })
    }

    /// Return the executable cell layout for one MIR type id.
    pub(crate) fn cell_layout(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<CellLayout> {
        let ty = self.storage_type(ty);

        self.cell_layout_node(self.tree.get(ty))
    }

    /// Return the scalar layout for one MIR type id.
    pub(crate) fn scalar_format(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<ScalarFormat> {
        let ty = self.storage_type(ty);

        self.scalar_layout_node(self.tree.get(ty))
    }

    /// Lower one MIR layout shape into an executable layout shape.
    pub(crate) fn layout_shape(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        shape: &mir::LayoutShape,
    ) -> Option<LayoutShape> {
        let repr = self.storage_type(ty);
        let type_shape = self.tree.get(repr);

        Some(match shape {
            mir::LayoutShape::None => LayoutShape::None,
            mir::LayoutShape::Scalar => self.scalar_layout_shape(repr, type_shape)?,
            mir::LayoutShape::Struct(layout) => LayoutShape::Struct(StructLayout {
                fields: self.layout_fields(&layout.fields),
            }),
            mir::LayoutShape::Tuple(layout) => LayoutShape::Tuple(TupleLayout {
                elements: self.layout_fields(&layout.elements),
            }),
            mir::LayoutShape::Slice => self.slice_layout_shape(type_shape)?,
            mir::LayoutShape::Array(layout) => LayoutShape::Array(self.element_layout(layout)),
            mir::LayoutShape::Vector(layout) => LayoutShape::Vector(self.element_layout(layout)),
            mir::LayoutShape::Tensor(layout) => LayoutShape::Tensor(TensorLayout {
                element: self.type_id(layout.element),
                format: layout.format,
                sharding: layout.sharding.clone(),
                rank: layout.rank,
            }),
            mir::LayoutShape::TensorView(layout) => LayoutShape::TensorView(TensorViewLayout {
                element: self.type_id(layout.element),
                format: layout.format,
                sharding: layout.sharding.clone(),
                rank: layout.rank,
            }),
            mir::LayoutShape::Variant(layout) => LayoutShape::Variant(VariantLayout {
                tag: VariantTagLayout {
                    ty: layout.tag.ty.map(|ty| self.type_id(ty)),
                    size: layout.tag.size,
                    alignment: layout.tag.alignment,
                },
                payload_offset: layout.payload_offset,
                variants: layout
                    .variants
                    .iter()
                    .map(|variant| VariantCaseLayout {
                        ty: self.type_id(variant.ty),
                        layout: LayoutId::new(variant.layout.raw()),
                    })
                    .collect(),
            }),
            mir::LayoutShape::Object(layout) => LayoutShape::Object(ObjectLayout {
                dispatch_offset: None,
                fields: self.layout_fields(&layout.fields),
            }),
            mir::LayoutShape::Dynamic => LayoutShape::Dynamic,
            mir::LayoutShape::Function => self.function_layout_shape(type_shape)?,
            mir::LayoutShape::Newtype(layout) => LayoutShape::Newtype(NewtypeLayout {
                backing_type: self.type_id(layout.backing_type),
                backing_layout: LayoutId::new(layout.backing_layout.raw()),
            }),
        })
    }

    /// Return the program type id for one MIR type.
    pub(crate) fn type_id(&self, ty: mir::LocalNodeId<mir::Type>) -> TypeId {
        self.program.type_id(ty)
    }

    /// Return flattened executable supertypes for one MIR type.
    fn supertypes(&self, ty: mir::LocalNodeId<mir::Type>) -> Vec<TypeId> {
        let mut supertypes = Vec::new();
        let Some(lineage) = self.types.lineage(ty) else {
            return supertypes;
        };

        // record class inheritance chain
        let mut parent = lineage.parent;
        while let Some(parent_type) = parent {
            let parent_id = self.type_id(parent_type);
            if !supertypes.contains(&parent_id) {
                supertypes.push(parent_id);
            }

            let parent_lineage = self.types.lineage(parent_type);
            if let Some(parent_lineage) = parent_lineage {
                for interface in &parent_lineage.interfaces {
                    let interface_id = self.type_id(*interface);
                    if !supertypes.contains(&interface_id) {
                        supertypes.push(interface_id);
                    }
                }
            }

            parent = parent_lineage.and_then(|lineage| lineage.parent);
        }

        // record implemented interface constraints
        for interface in &lineage.interfaces {
            let interface_id = self.type_id(*interface);
            if !supertypes.contains(&interface_id) {
                supertypes.push(interface_id);
            }
        }

        supertypes
    }

    /// Map a MIR reference kind to one runtime address space.
    pub(crate) fn address_space(
        &self,
        space: mir::Space,
        kind: mir::ReferenceKind,
    ) -> AddressSpace {
        match space {
            mir::Space::Local => match kind {
                mir::ReferenceKind::Managed | mir::ReferenceKind::Unique => AddressSpace::Local,
                mir::ReferenceKind::Borrowed => AddressSpace::Local,
                mir::ReferenceKind::Raw => AddressSpace::Raw,
            },
            mir::Space::Shared => match kind {
                mir::ReferenceKind::Managed | mir::ReferenceKind::Unique => AddressSpace::Shared,
                mir::ReferenceKind::Borrowed => AddressSpace::Shared,
                mir::ReferenceKind::Raw => AddressSpace::Raw,
            },
            mir::Space::Frame => AddressSpace::Frame,
            mir::Space::Static => AddressSpace::Static,
        }
    }

    /// Return the direct representation type for one MIR type.
    fn repr_type(&self, ty: TypeId, type_shape: &mir::Type) -> TypeId {
        let mut repr = ty;
        let mut shape = type_shape;

        loop {
            match shape {
                mir::Type::WithLifetimes { base, .. }
                | mir::Type::Uninit { value: base }
                | mir::Type::Atomic { value: base }
                | mir::Type::Newtype { inner: base, .. } => {
                    repr = self.program.type_id(*base);
                    shape = self.tree.get(*base);
                }
                _ => return repr,
            }
        }
    }

    /// Return the executable storage type for one MIR type.
    pub(crate) fn storage_type(
        &self,
        mut ty: mir::LocalNodeId<mir::Type>,
    ) -> mir::LocalNodeId<mir::Type> {
        loop {
            match self.tree.get(ty) {
                mir::Type::WithLifetimes { base, .. }
                | mir::Type::Uninit { value: base }
                | mir::Type::Atomic { value: base }
                | mir::Type::Newtype { inner: base, .. } => {
                    ty = *base;
                }
                _ => return ty,
            }
        }
    }

    /// Return the executable layout id for one program type when one exists.
    fn executable_layout_id(
        &self,
        ty: TypeId,
        layout_ids: &HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
    ) -> LinkResult<LayoutId> {
        let type_id = self
            .program
            .type_by_id(ty)
            .ok_or_else(|| self.program.invalid_input(format!("type id {ty:?}")))?;
        let layout = layout_ids.get(&type_id).ok_or_else(|| {
            self.program
                .invalid_input(format!("layout for {type_id:?}"))
        })?;

        Ok(*layout)
    }

    /// Return the cell layout for one MIR type.
    fn cell_layout_node(&self, ty: &mir::Type) -> Option<CellLayout> {
        match ty {
            mir::Type::Void => Some(CellLayout::Void),
            mir::Type::Boolean => Some(CellLayout::Boolean),
            mir::Type::Int { width, is_signed } => {
                let width = u8::try_from(*width).ok()?;

                if *is_signed {
                    Some(CellLayout::Int { width })
                } else {
                    Some(CellLayout::Uint { width })
                }
            }
            mir::Type::Isize => Some(CellLayout::Int {
                width: self.pointer_bytes() * 8,
            }),
            mir::Type::Usize | mir::Type::TypeDescriptor | mir::Type::TypeId => {
                Some(CellLayout::Uint {
                    width: self.pointer_bytes() * 8,
                })
            }
            mir::Type::Float(mir::FloatType::Float16) => Some(CellLayout::Float16),
            mir::Type::Float(mir::FloatType::Bfloat16) => Some(CellLayout::Bfloat16),
            mir::Type::Float(mir::FloatType::Float32) => Some(CellLayout::Float32),
            mir::Type::Float(mir::FloatType::Float64) => Some(CellLayout::Float64),
            mir::Type::Reference { kind, space, .. } => {
                let address_space = self.address_space(space.clone(), *kind);

                Some(address_space.cell_layout())
            }
            mir::Type::FunctionPointer { .. } => Some(CellLayout::FunctionPointer),
            mir::Type::Tensor { .. } => Some(CellLayout::HeapReference),
            mir::Type::FunctionSignature { .. } | mir::Type::Function { .. } => None,
            _ => None,
        }
    }

    /// Return the scalar layout for one MIR type.
    fn scalar_layout_node(&self, ty: &mir::Type) -> Option<ScalarFormat> {
        match ty {
            mir::Type::Int { width, is_signed } => Some(ScalarFormat::Int {
                width: *width,
                is_signed: *is_signed,
            }),
            mir::Type::Isize => Some(ScalarFormat::Int {
                width: u16::from(self.pointer_bytes()) * 8,
                is_signed: true,
            }),
            mir::Type::Usize | mir::Type::TypeDescriptor | mir::Type::TypeId => {
                Some(ScalarFormat::Int {
                    width: u16::from(self.pointer_bytes()) * 8,
                    is_signed: false,
                })
            }
            mir::Type::Float(float_type) => Some(ScalarFormat::Float {
                format: *float_type,
            }),
            mir::Type::Boolean => Some(ScalarFormat::Boolean),
            _ => None,
        }
    }

    /// Lower one scalar-like MIR type into an executable layout shape.
    fn scalar_layout_shape(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        type_shape: &mir::Type,
    ) -> Option<LayoutShape> {
        match type_shape {
            mir::Type::Reference {
                kind,
                space,
                access,
                pointee,
                nullability,
                ..
            } => Some(LayoutShape::Reference(ReferenceLayout {
                pointee: self.type_id(*pointee),
                flags: ReferenceFlags::new(*kind, space.clone(), *access, *nullability),
            })),
            mir::Type::FunctionPointer { signature } => {
                Some(LayoutShape::FunctionPointer(self.signature(*signature)?))
            }
            _ => Some(LayoutShape::Scalar(self.scalar_format(ty)?)),
        }
    }

    /// Lower one MIR slice type into an executable layout shape.
    fn slice_layout_shape(&self, type_shape: &mir::Type) -> Option<LayoutShape> {
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

        Some(LayoutShape::Slice(SliceLayout {
            data: ReferenceLayout {
                pointee: self.type_id(*element),
                flags: ReferenceFlags::new(*kind, space.clone(), *access, *nullability),
            },
        }))
    }

    /// Lower one MIR function type into an executable layout shape.
    fn function_layout_shape(&self, type_shape: &mir::Type) -> Option<LayoutShape> {
        let mir::Type::Function {
            signature,
            environment,
        } = type_shape
        else {
            return None;
        };

        Some(LayoutShape::Function(FunctionLayout {
            signature: self.signature(*signature)?,
            environment: self.type_id(*environment),
        }))
    }

    /// Lower one MIR element layout.
    fn element_layout(&self, layout: &mir::ElementLayout) -> ElementLayout {
        ElementLayout {
            element: self.type_id(layout.element),
            stride: layout.stride,
            count: layout.count,
        }
    }

    /// Lower MIR layout fields.
    fn layout_fields(&self, fields: &[mir::LayoutField]) -> Vec<LayoutField> {
        fields
            .iter()
            .map(|field| LayoutField {
                name: field.name,
                ty: self.type_id(field.ty),
                offset: field.offset,
                size: field.size,
                alignment: field.alignment,
                source_index: field.source_index,
            })
            .collect()
    }

    /// Return one executable function signature.
    pub(crate) fn signature(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<Signature> {
        let ty = self.storage_type(ty);
        let ty = self.tree.get(ty);
        let mir::Type::FunctionSignature {
            parameters, result, ..
        } = ty
        else {
            return None;
        };

        Some(Signature {
            parameters: parameters
                .iter()
                .map(|parameter| self.program.type_id(parameter.ty))
                .collect(),
            result: self.program.type_id(*result),
        })
    }
}
