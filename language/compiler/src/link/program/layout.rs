use tspp_mir::{TraceMap, TraceTable};
use tspp_program::{LayoutBuilder, LayoutShapeBuilder, TypeId};

use crate::LinkResult;

use super::{ObjectTypes, ProgramLinker};

/// Linked program layouts and trace maps.
#[derive(Debug)]
pub(crate) struct ProgramLayouts {
    /// Program layout table.
    pub(crate) layouts: Vec<LayoutBuilder>,
    /// Trace maps keyed by program trace id.
    pub(crate) traces: TraceTable,
}

/// One canonical program layout before trace ids are assigned.
#[derive(Debug)]
struct CanonicalLayout {
    /// Runtime layout shape.
    shape: LayoutShapeBuilder,
    /// Total byte width.
    size: u32,
    /// Required byte alignment.
    alignment: u32,
    /// Reference offsets traced by the collector.
    trace_map: TraceMap,
}

/// Link complete MIR layouts into program layout and trace tables.
#[derive(Debug)]
pub(crate) struct LayoutLinker<'a> {
    /// Dense program id projection.
    program: &'a ProgramLinker<'a>,
}

impl<'a> LayoutLinker<'a> {
    /// Create one layout linker.
    pub(crate) fn new(program: &'a ProgramLinker<'a>) -> Self {
        Self { program }
    }

    /// Link program layouts and trace maps.
    pub(crate) fn link(&self) -> LinkResult<ProgramLayouts> {
        let mut canonical: Vec<Option<CanonicalLayout>> =
            Vec::with_capacity(self.program.types_by_id().len());
        canonical.resize_with(self.program.types_by_id().len(), || None);

        // merge every module-local representation and reject conflicts
        for (module, object) in self.program.objects() {
            let types = ObjectTypes::new(*module, object, self.program);
            for object_type in object.types() {
                let type_id = object_type.id;
                if !self.program.has_type(*module, type_id) {
                    continue;
                }

                let source = object.layouts().type_layout(type_id).ok_or_else(|| {
                    self.program
                        .invalid_input(format!("missing layout for {type_id:?}"))
                })?;
                let shape = types.layout_shape(type_id, &source.shape).ok_or_else(|| {
                    self.program
                        .invalid_input(format!("invalid layout shape for {type_id:?}"))
                })?;
                let ty = self.program.type_id(*module, type_id);
                let entry = &mut canonical[ty.index()];
                if let Some(previous) = entry {
                    if previous.shape != shape
                        || previous.size != source.size
                        || previous.alignment != source.alignment
                        || previous.trace_map != source.trace_map
                    {
                        return Err(self
                            .program
                            .invalid_input(format!("type {ty:?} has conflicting layouts")));
                    }
                } else {
                    *entry = Some(CanonicalLayout {
                        shape,
                        size: source.size,
                        alignment: source.alignment,
                        trace_map: source.trace_map.clone(),
                    });
                }
            }
        }

        // assign trace ids in dense program type order
        let mut entries = Vec::with_capacity(canonical.len());
        let mut traces = TraceTable::new();
        for (index, layout) in canonical.into_iter().enumerate() {
            let ty = TypeId::from(index as u32);
            let layout = layout.ok_or_else(|| {
                self.program
                    .invalid_input(format!("type {ty:?} has no layout"))
            })?;
            let CanonicalLayout {
                shape,
                size,
                alignment,
                trace_map,
            } = layout;
            let trace = traces.insert(trace_map);
            entries.push(LayoutBuilder::new(shape, size, alignment, trace));
        }

        Ok(ProgramLayouts {
            layouts: entries,
            traces,
        })
    }
}
