use std::collections::HashMap;

use destack_core::SectionPacker;
use destack_mir as mir;
use destack_mir::{TraceMap, TraceTable};
use destack_program::{LayoutBuilder, LayoutId, LayoutShapeBuilder, LayoutTable};

use crate::LinkResult;

use super::vm::StorageLayout;
use super::{ProgramLinker, TypeLinker};

/// Linked program layouts and transient storage layouts.
#[derive(Debug)]
pub(crate) struct ProgramLayouts {
    /// Program layout ids keyed by MIR type id.
    pub(crate) ids: HashMap<mir::TypeId, LayoutId>,
    /// Lowered value storage layouts keyed by MIR type id.
    pub(crate) storage: HashMap<mir::TypeId, StorageLayout>,
    /// Program layout table.
    pub(crate) layouts: LayoutTable,
    /// Compiler trace maps keyed by program TraceId.
    pub(crate) trace_maps: TraceTable,
}

/// Link MIR layouts into program layout and trace tables.
#[derive(Debug)]
pub(crate) struct LayoutLinker<'a> {
    /// MIR tree being linked.
    tree: &'a mir::Tree,
    /// Target ABI layout for this program.
    target_layout: &'a mir::TargetLayout,
    /// MIR type table for names, lineage, and type attachments.
    types: &'a mir::TypeTable,
    /// MIR layout table produced by lower and optimization.
    source: &'a mir::LayoutTable,
    /// Dense program id projection.
    program: &'a ProgramLinker,
}

impl<'a> LayoutLinker<'a> {
    /// Create one layout linker.
    pub(crate) fn new(
        tree: &'a mir::Tree,
        target_layout: &'a mir::TargetLayout,
        types: &'a mir::TypeTable,
        source: &'a mir::LayoutTable,
        program: &'a ProgramLinker,
    ) -> Self {
        Self {
            tree,
            target_layout,
            types,
            source,
            program,
        }
    }

    /// Link program layout, trace, and transient storage layouts.
    pub(crate) fn link(&self, sections: &mut SectionPacker) -> LinkResult<ProgramLayouts> {
        let ids = self.layout_ids()?;
        let storage = StorageLayout::build_all(
            self.tree,
            self.target_layout,
            self.source,
            &ids,
            self.program,
        )?;
        let (layouts, traces) = self.layout_table(sections, &storage)?;

        Ok(ProgramLayouts {
            ids,
            storage,
            layouts,
            trace_maps: traces,
        })
    }

    /// Build the layout id map for all program MIR types.
    fn layout_ids(&self) -> LinkResult<HashMap<mir::TypeId, LayoutId>> {
        let mut next_layout_id = self
            .tree
            .iter_nodes::<mir::Type>()
            .filter_map(|(type_id, _)| self.source.layout_id(type_id))
            .map(|layout_id| layout_id.raw())
            .max()
            .map_or(Ok(1), |layout_id| {
                layout_id
                    .checked_add(1)
                    .ok_or_else(|| self.program.layout_overflow("layout id space exhausted"))
            })?;
        let mut ids = HashMap::new();

        // preserve existing MIR layout ids and assign dense ids to implicit layouts
        for (type_id, _) in self.tree.iter_nodes::<mir::Type>() {
            let layout_id = if let Some(layout_id) = self.source.layout_id(type_id) {
                LayoutId::new(layout_id.raw())
            } else {
                let layout_id = LayoutId::new(next_layout_id);
                next_layout_id = next_layout_id
                    .checked_add(1)
                    .ok_or_else(|| self.program.layout_overflow("layout id space exhausted"))?;

                layout_id
            };

            ids.insert(type_id, layout_id);
        }

        Ok(ids)
    }

    /// Build the program layout table from lowered value layouts.
    fn layout_table(
        &self,
        sections: &mut SectionPacker,
        storage: &HashMap<mir::TypeId, StorageLayout>,
    ) -> LinkResult<(LayoutTable, TraceTable)> {
        let max_layout_id = storage
            .values()
            .map(|layout| layout.layout_id.raw() as usize)
            .max()
            .unwrap_or_default();
        let mut traces = TraceTable::new();
        let empty_trace = traces.insert(TraceMap::empty());
        let empty_layout = LayoutBuilder {
            shape: LayoutShapeBuilder::None,
            size: 0,
            alignment: 1,
            trace: empty_trace,
        };
        let mut entries = vec![empty_layout; max_layout_id];

        let type_linker = TypeLinker::new(self.tree, self.target_layout, self.types, self.program);

        // project each lowered layout into program ids
        for (type_id, layout) in storage {
            let program_layout =
                self.program_layout(*type_id, layout, &type_linker, &mut traces)?;

            let Some(entry) = entries.get_mut(layout.layout_id.index()) else {
                return Err(self
                    .program
                    .layout_overflow(format!("layout id out of range: {:?}", layout.layout_id)));
            };
            *entry = program_layout;
        }

        let table = LayoutTable::pack(sections, entries);

        Ok((table, traces))
    }

    /// Build one program layout entry for one MIR type.
    fn program_layout(
        &self,
        type_id: mir::TypeId,
        layout: &StorageLayout,
        type_linker: &TypeLinker<'_>,
        traces: &mut TraceTable,
    ) -> LinkResult<LayoutBuilder> {
        let shape = if let Some(source) = self.source.type_layout(self.tree.repr_type(type_id)) {
            source.shape.clone()
        } else {
            self.layout_shape(type_id, layout)?
        };
        let shape = type_linker.layout_shape(type_id, &shape).ok_or_else(|| {
            self.program.invalid_input(format!(
                "program layout shape for {type_id:?} {:?}: {shape:?}",
                self.tree.get(type_id)
            ))
        })?;

        Ok(LayoutBuilder {
            shape,
            size: u32::try_from(layout.byte_len)
                .map_err(|_| self.program.layout_overflow("layout byte length"))?,
            alignment: u32::try_from(layout.alignment())
                .map_err(|_| self.program.layout_overflow("layout alignment"))?,
            trace: traces.insert(layout.trace_map.clone()),
        })
    }

    /// Build a layout shape for MIR that has no layout table entry.
    fn layout_shape(
        &self,
        type_id: mir::TypeId,
        layout: &StorageLayout,
    ) -> LinkResult<mir::LayoutShape> {
        match self.tree.get(self.tree.repr_type(type_id)) {
            mir::Type::Void | mir::Type::FunctionSignature { .. } => Ok(mir::LayoutShape::None),
            mir::Type::Boolean
            | mir::Type::Int { .. }
            | mir::Type::Isize
            | mir::Type::Usize
            | mir::Type::Float(_)
            | mir::Type::TypeDescriptor
            | mir::Type::TypeId
            | mir::Type::Reference { .. }
            | mir::Type::FunctionPointer { .. } => Ok(mir::LayoutShape::Scalar),
            mir::Type::Slice { .. } => Ok(mir::LayoutShape::Slice),
            mir::Type::Function { .. } => Ok(mir::LayoutShape::Function),
            mir::Type::FixedArray {
                element, length, ..
            } => {
                let count = u32::try_from(*length)
                    .map_err(|_| self.program.layout_overflow("array length"))?;
                let stride = layout
                    .element()
                    .map(|element| element.stride)
                    .ok_or_else(|| self.program.invalid_input("array element layout"))?;
                let stride = u32::try_from(stride)
                    .map_err(|_| self.program.layout_overflow("array stride"))?;

                Ok(mir::LayoutShape::Array(mir::ElementLayout {
                    element: *element,
                    stride,
                    count,
                }))
            }
            mir::Type::Struct { .. } => Ok(mir::LayoutShape::Struct(mir::StructLayout {
                fields: self.layout_fields(layout)?,
            })),
            mir::Type::Tuple { .. } => Ok(mir::LayoutShape::Tuple(mir::TupleLayout {
                elements: self.layout_fields(layout)?,
            })),
            mir::Type::Vector { element, lanes, .. } => {
                let stride = layout
                    .element()
                    .map(|element| element.stride)
                    .ok_or_else(|| self.program.invalid_input("vector element layout"))?;
                let stride = u32::try_from(stride)
                    .map_err(|_| self.program.layout_overflow("vector stride"))?;

                Ok(mir::LayoutShape::Vector(mir::ElementLayout {
                    element: *element,
                    stride,
                    count: *lanes,
                }))
            }
            mir::Type::Tensor {
                element,
                shape,
                format,
                sharding,
                ..
            } => Ok(mir::LayoutShape::Tensor(mir::TensorLayout {
                element: *element,
                format: *format,
                sharding: sharding.clone(),
                rank: u32::try_from(shape.len())
                    .map_err(|_| self.program.layout_overflow("tensor rank"))?,
            })),
            mir::Type::TensorView {
                element,
                shape,
                format,
                sharding,
                ..
            } => Ok(mir::LayoutShape::TensorView(mir::TensorViewLayout {
                element: *element,
                format: *format,
                sharding: sharding.clone(),
                rank: u32::try_from(shape.len())
                    .map_err(|_| self.program.layout_overflow("tensor view rank"))?,
            })),
            mir::Type::Atomic { value } | mir::Type::Uninit { value } => {
                self.layout_shape(*value, layout)
            }
            mir::Type::Dynamic { .. } => Ok(mir::LayoutShape::Dynamic),
            mir::Type::Variant { .. } => Err(self.program.invalid_input("variant layout")),
            mir::Type::Newtype { .. } | mir::Type::WithLifetimes { .. } | mir::Type::Error => {
                Err(self.program.invalid_input("type layout"))
            }
        }
    }

    /// Return layout fields from one lowered value layout.
    fn layout_fields(&self, layout: &StorageLayout) -> LinkResult<Vec<mir::LayoutField>> {
        let field_count = layout
            .field_count()
            .ok_or_else(|| self.program.invalid_input("field layout"))?;
        let mut fields = Vec::with_capacity(field_count);

        for index in 0..field_count {
            let field = layout
                .field(index as u32)
                .ok_or_else(|| self.program.invalid_field_access(index as u32, field_count))?;
            fields.push(mir::LayoutField {
                name: None,
                ty: field.ty,
                offset: u32::try_from(field.offset)
                    .map_err(|_| self.program.layout_overflow("field offset"))?,
                size: u32::try_from(field.byte_len)
                    .map_err(|_| self.program.layout_overflow("field byte length"))?,
                alignment: 1,
                source_index: Some(index as u32),
            });
        }

        Ok(fields)
    }
}
