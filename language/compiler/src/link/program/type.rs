use std::collections::HashMap;
use std::sync::Arc;

use tspp_core::Optional;
use tspp_heap::DropId;
use tspp_mir as mir;
use tspp_program::{
    DropEntry, DynamicLayout, ElementLayout, FunctionId, FunctionLayout, LayoutField,
    LayoutShapeBuilder, NewtypeLayout, Object, ObjectLayoutBuilder, PointerLayout, ReferenceLayout,
    ScalarFormat, SignatureId, SliceLayout, TypeDescriptorBuilder, TypeFingerprint, TypeId,
    TypeTableBuilder, VariantCaseLayout, VariantLayoutBuilder,
};
use tspp_source::{ModuleId, PackageId};

use crate::{LinkError, LinkResult};

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
    object: &'a Object,
    /// Program linker owning final type identities.
    program: &'a ProgramLinker<'a>,
}

impl<'a> TypeLinker<'a> {
    /// Create one type linker.
    pub(crate) fn new(program: &'a ProgramLinker<'a>) -> Self {
        Self { program }
    }

    /// Link the program type table.
    pub(crate) fn link(&self) -> LinkResult<TypeTableBuilder> {
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

        // pair each descriptor with its stable structural identity
        let fingerprints = self
            .program
            .types_by_id()
            .iter()
            .map(|(module, ty)| {
                let ty = self.program.object(*module).ty(*ty).ok_or_else(|| {
                    self.program
                        .invalid_input("missing canonical type fingerprint")
                })?;

                Ok(TypeFingerprint::from_raw(ty.fingerprint.raw()))
            })
            .collect::<LinkResult<Vec<_>>>()?;

        Ok(TypeTableBuilder::new().types(fingerprints.into_iter().zip(entries)))
    }
}

impl<'a> ObjectTypes<'a> {
    /// Create one object type projection.
    pub(crate) fn new(
        module: ModuleId,
        object: &'a Object,
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
    pub(crate) fn descriptor(&self, type_id: mir::TypeId) -> TypeDescriptorBuilder {
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

    /// Return the scalar format for one MIR type id.
    pub(crate) fn scalar_format(&self, ty: mir::TypeId) -> Option<ScalarFormat> {
        let ty = self.storage_type(ty);

        self.scalar_layout_node(self.get(ty))
    }

    /// Project one MIR layout shape into a program layout shape.
    pub(crate) fn layout_shape(
        &self,
        ty: mir::TypeId,
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
            mir::LayoutShape::Variant(layout) => {
                let cases = layout.cases.iter().map(|variant| VariantCaseLayout {
                    discriminant: variant.discriminant,
                    ty: self.type_id(variant.ty),
                    payload_offset: variant.payload_offset,
                });
                LayoutShapeBuilder::Variant(
                    VariantLayoutBuilder::new(self.type_id(layout.discriminant), layout.encoding)
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
            mir::LayoutShape::Dynamic => self.dynamic_layout_shape(type_shape)?,
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

    /// Project one MIR dynamic type into a program layout shape.
    fn dynamic_layout_shape(&self, type_shape: &mir::Type) -> Option<LayoutShapeBuilder> {
        let mir::Type::Dynamic { constraint, .. } = type_shape else {
            return None;
        };

        Some(LayoutShapeBuilder::Dynamic(DynamicLayout {
            constraint: self.type_id(*constraint),
        }))
    }

    /// Return the program type id for one MIR type.
    pub(crate) fn type_id(&self, ty: mir::TypeId) -> TypeId {
        self.program.type_id(self.module, ty)
    }

    /// Return flattened program supertypes for one MIR type.
    fn supertypes(&self, ty: mir::TypeId) -> Vec<TypeId> {
        let mut supertypes = Vec::new();
        self.append_supertypes(ty, ty, &mut supertypes);

        supertypes
    }

    /// Append transitive parents and interfaces for one MIR type.
    fn append_supertypes(&self, root: mir::TypeId, ty: mir::TypeId, supertypes: &mut Vec<TypeId>) {
        let Some(heritage) = self.object.ty(ty).and_then(|ty| ty.heritage.as_ref()) else {
            return;
        };

        // append inherited hierarchies before implemented interfaces
        for &base in &heritage.extends {
            self.append_supertype(root, base, supertypes);
        }

        // append each interface and its own inherited interfaces
        for &interface in &heritage.implements {
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
    pub(crate) fn storage_type(&self, mut ty: mir::TypeId) -> mir::TypeId {
        loop {
            match self.get(ty) {
                mir::Type::Application { base, .. }
                | mir::Type::Uninit { value: base }
                | mir::Type::ManuallyDrop { value: base } => {
                    ty = *base;
                }
                _ => return ty,
            }
        }
    }

    /// Return the scalar format for one MIR type shape.
    fn scalar_layout_node(&self, ty: &mir::Type) -> Option<ScalarFormat> {
        match ty {
            mir::Type::Int { width, is_signed } => Some(ScalarFormat::int(*width, *is_signed)),
            mir::Type::Isize => Some(ScalarFormat::int(u16::from(self.pointer_bytes()) * 8, true)),
            mir::Type::Usize => Some(ScalarFormat::int(
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
        ty: mir::TypeId,
        type_shape: &mir::Type,
    ) -> Option<LayoutShapeBuilder> {
        match type_shape {
            mir::Type::Reference {
                kind,
                access,
                pointee,
                ..
            } => Some(LayoutShapeBuilder::Reference(ReferenceLayout::new(
                self.type_id(*pointee),
                *kind,
                *access,
            ))),
            mir::Type::Pointer { pointee, access } => Some(LayoutShapeBuilder::Pointer(
                PointerLayout::new(self.type_id(*pointee), *access),
            )),
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
            access,
            element,
            ..
        } = type_shape
        else {
            return None;
        };

        // build the slice's element reference
        let reference = ReferenceLayout::new(self.type_id(*element), *kind, *access);

        Some(LayoutShapeBuilder::Slice(SliceLayout { reference }))
    }

    /// Project one MIR function type into a program layout shape.
    fn function_layout_shape(&self, type_shape: &mir::Type) -> Option<LayoutShapeBuilder> {
        let mir::Type::Function { signature, .. } = type_shape else {
            return None;
        };

        Some(LayoutShapeBuilder::Function(FunctionLayout {
            signature: self.signature(*signature)?,
        }))
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
        // pair each projected field with its declaration index
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

        // restore source declaration order
        fields.sort_by_key(|(source_index, _)| *source_index);

        fields.into_iter().map(|(_, field)| field).collect()
    }

    /// Return one program function signature.
    pub(crate) fn signature(&self, ty: mir::TypeId) -> Option<SignatureId> {
        let ty = self.storage_type(ty);

        self.program.type_signature_id(self.module, ty)
    }
}

impl TypeLinker<'_> {
    /// Build dense type ids from nominal identity and anonymous structure.
    pub(crate) fn index(
        objects: &[(ModuleId, Arc<Object>)],
    ) -> (
        HashMap<(ModuleId, mir::TypeId), TypeId>,
        Vec<(ModuleId, mir::TypeId)>,
    ) {
        let mut ids = HashMap::new();
        let mut types = Vec::new();
        let mut fingerprints = HashMap::new();

        // canonicalize nominal identities and anonymous structural types
        for (module, object) in objects {
            for ty in object.types() {
                if object.layouts().layout_id(ty.id).is_none() {
                    continue;
                }

                let id = *fingerprints.entry(ty.fingerprint).or_insert_with(|| {
                    let id = TypeId::from(types.len() as u32);
                    types.push((*module, ty.id));
                    id
                });

                ids.insert((*module, ty.id), id);
            }
        }

        (ids, types)
    }

    /// Return whether two module-local types resolve to the same program type.
    pub(crate) fn same(
        left_module: ModuleId,
        left: mir::TypeId,
        right_module: ModuleId,
        right: mir::TypeId,
        type_ids: &HashMap<(ModuleId, mir::TypeId), TypeId>,
    ) -> bool {
        let left_id = type_ids[&(left_module, left)];
        let right_id = type_ids[&(right_module, right)];

        left_id == right_id
    }

    /// Build dense drop identities and destructor functions in program type order.
    pub(crate) fn drops(
        package: PackageId,
        objects: &[(ModuleId, Arc<Object>)],
        type_ids: &HashMap<(ModuleId, mir::TypeId), TypeId>,
        type_count: usize,
        function_ids: &HashMap<(ModuleId, mir::FunctionId), FunctionId>,
    ) -> LinkResult<(HashMap<TypeId, DropId>, Vec<DropEntry>)> {
        let mut functions = HashMap::new();

        // resolve every specialized destructor into canonical program identity
        for (module, object) in objects {
            for (ty, storage, function) in object.drops().destructors() {
                if matches!(
                    storage,
                    mir::Storage::Static(mir::Space::Constant)
                        | mir::Storage::Static(mir::Space::Local)
                        | mir::Storage::Static(mir::Space::Shared)
                ) {
                    return Err(LinkError::invalid_input(
                        package,
                        format!("type {ty:?} defines a destructor for global storage"),
                    ));
                }

                let ty = type_ids[&(*module, ty)];
                let function_id = function_ids[&(*module, function)];
                let key = (ty, storage);

                if let Some((previous_module, previous_function)) = functions.get(&key) {
                    let previous_id = function_ids[&(*previous_module, *previous_function)];
                    if previous_id != function_id {
                        return Err(LinkError::invalid_input(
                            package,
                            format!("type {ty:?} has multiple {storage:?} destructors"),
                        ));
                    }
                } else {
                    functions.insert(key, (*module, function));
                }
            }
        }

        // assign drop ids in canonical program type order
        let mut ids = HashMap::new();
        let mut drops = Vec::new();
        for index in 0..type_count {
            let ty = TypeId::from(index as u32);
            let frame = functions
                .remove(&(ty, mir::Storage::Frame))
                .map(|(module, function)| function_ids[&(module, function)]);
            let local = functions
                .remove(&(ty, mir::Storage::Heap(mir::Space::Local)))
                .map(|(module, function)| function_ids[&(module, function)]);
            let shared = functions
                .remove(&(ty, mir::Storage::Heap(mir::Space::Shared)))
                .map(|(module, function)| function_ids[&(module, function)]);

            // skip types without a destructor in any storage
            if frame.is_none() && local.is_none() && shared.is_none() {
                continue;
            }

            ids.insert(ty, DropId::from_index(drops.len() as u32));
            drops.push(DropEntry {
                frame: Optional::from(frame),
                local: Optional::from(local),
                shared: Optional::from(shared),
            });
        }

        Ok((ids, drops))
    }
}
