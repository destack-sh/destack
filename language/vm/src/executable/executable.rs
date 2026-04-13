use std::collections::{HashMap, HashSet};
use std::fmt;

use destack_core::ImmutableStringPool;
use {destack_engine as engine, destack_mir as mir};

use super::layout::{Layout, build_layouts};
use super::lower::{LoweredValueSlot, analyze_lowered_value_slots, lower_function};
use super::value::frame_slot_value_class_from_type;
use super::{CallTarget, Function, FunctionTable};
use crate::{Error, Result};

/// Immutable runnable lowering and metadata shared across isolates.
pub struct Executable {
    /// The MIR tree executed by this executable.
    pub(crate) tree: mir::NodeTree,
    /// The immutable string pool for this executable.
    pub(crate) strings: ImmutableStringPool,
    /// Lowered function bodies for the current interpreter backend.
    pub(crate) functions: FunctionTable,
    /// Lookup table for function ids by name.
    pub(crate) function_id_by_name: HashMap<String, mir::LocalNodeId<mir::Function>>,
    /// Compiled layouts keyed by MIR type id.
    pub(crate) layouts: HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    /// Lookup table for vtables keyed by vtable globals.
    pub(crate) vtable_id_by_global: HashMap<mir::LocalNodeId<mir::Global>, mir::VtableId>,

    /// Logical frame layouts by dense layout id.
    pub(crate) frame_layouts: Vec<engine::FrameLayout>,
    /// Logical frame layout id by owning function.
    pub(crate) frame_layout_id_by_function:
        HashMap<mir::LocalNodeId<mir::Function>, engine::FrameLayoutId>,

    /// Resume points by dense resume point id.
    pub(crate) resume_points: Vec<engine::ResumePoint>,
    /// Resume transfers by dense transfer id.
    pub(crate) resume_transfers: Vec<engine::ResumeTransfer>,
    /// Generic resume point ids keyed by function, block, and instruction offset.
    pub(crate) resume_point_id_by_position: HashMap<
        (
            mir::LocalNodeId<mir::Function>,
            mir::LocalNodeId<mir::Block>,
            u32,
        ),
        engine::ResumePointId,
    >,

    /// Safepoints by dense safepoint id.
    pub(crate) safepoints: Vec<engine::Safepoint>,
    /// Safepoint id keyed by semantic resume point.
    pub(crate) safepoint_id_by_resume_point: HashMap<engine::ResumePointId, engine::SafepointId>,
    /// Materialization maps by dense map id.
    pub(crate) materialization_maps: Vec<engine::MaterializationMap>,
}

impl Executable {
    /// Build one executable from one MIR tree and immutable string pool.
    pub fn new(tree: mir::NodeTree, strings: ImmutableStringPool) -> Result<Self> {
        ExecutableBuilder::new(tree, strings).build()
    }

    /// Return the frame layout for one function when present.
    pub(crate) fn frame_layout(
        &self,
        function: mir::LocalNodeId<mir::Function>,
    ) -> Option<&engine::FrameLayout> {
        let layout_id = self.frame_layout_id_by_function.get(&function)?;
        self.frame_layouts.get(layout_id.0 as usize)
    }

    /// Return the frame layout for one layout id when present.
    pub(crate) fn frame_layout_by_id(
        &self,
        frame_layout: engine::FrameLayoutId,
    ) -> Option<&engine::FrameLayout> {
        self.frame_layouts.get(frame_layout.0 as usize)
    }

    /// Return one resume point by id.
    pub(crate) fn resume_point(
        &self,
        resume_point: engine::ResumePointId,
    ) -> Option<&engine::ResumePoint> {
        self.resume_points.get(resume_point.0 as usize)
    }

    /// Return one resume transfer by id.
    pub(crate) fn resume_transfer(
        &self,
        resume_transfer: engine::ResumeTransferId,
    ) -> Option<&engine::ResumeTransfer> {
        self.resume_transfers.get(resume_transfer.0 as usize)
    }

    /// Return one safepoint by id.
    pub(crate) fn safepoint(&self, safepoint: engine::SafepointId) -> Option<&engine::Safepoint> {
        self.safepoints.get(safepoint.0 as usize)
    }

    /// Return one safepoint id for one semantic resume point.
    pub(crate) fn safepoint_for_resume_point(
        &self,
        resume_point: engine::ResumePointId,
    ) -> Option<engine::SafepointId> {
        self.safepoint_id_by_resume_point
            .get(&resume_point)
            .copied()
    }

    /// Return one materialization map by id.
    pub(crate) fn materialization_map(
        &self,
        materialization_map: engine::MaterializationMapId,
    ) -> Option<&engine::MaterializationMap> {
        self.materialization_maps
            .get(materialization_map.0 as usize)
    }

    /// Return the compiled layout for one MIR type.
    pub(crate) fn layout(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<&Layout> {
        self.layouts.get(&ty)
    }

    /// Return one generic resume point id for one execution position.
    pub(crate) fn resume_point_for_position(
        &self,
        function: mir::LocalNodeId<mir::Function>,
        block: mir::LocalNodeId<mir::Block>,
        instruction_offset: u32,
    ) -> Option<engine::ResumePointId> {
        self.resume_point_id_by_position
            .get(&(function, block, instruction_offset))
            .copied()
    }

    /// Return the caller return destination implied by one resume point.
    pub(crate) fn return_destination_for_resume_point(
        &self,
        resume_point: engine::ResumePointId,
    ) -> Result<Option<mir::Value>> {
        let Some(resume_point) = self.resume_point(resume_point) else {
            return Ok(None);
        };

        // a resume at the start of a block has no preceding call
        if resume_point.mir_instruction_offset == 0 {
            return Ok(None);
        }

        // resolve the preceding MIR instruction in the resumed block
        let block = self.tree.get(resume_point.block);
        let instruction_index = resume_point.mir_instruction_offset as usize - 1;
        let Some(instruction_id) = block.instructions.get(instruction_index).copied() else {
            return Ok(None);
        };
        let instruction = self.tree.get(instruction_id);

        // recover the call destination when the resumed instruction follows a call
        match instruction {
            mir::Instruction::Call { destination, .. }
            | mir::Instruction::CallVirtual { destination, .. }
            | mir::Instruction::CallInterface { destination, .. }
            | mir::Instruction::CallIndirect { destination, .. } => (*destination)
                .map(|value| {
                    value.value().ok_or_else(|| Error::ConcreteMirRequired {
                        context: "call destination".to_string(),
                    })
                })
                .transpose(),
            _ => Ok(None),
        }
    }

    /// Return the caller return destination implied by one execution position.
    pub(crate) fn return_destination_for_position(
        &self,
        function: mir::LocalNodeId<mir::Function>,
        block: mir::LocalNodeId<mir::Block>,
        instruction_offset: u32,
    ) -> Result<Option<mir::Value>> {
        let Some(resume_point) =
            self.resume_point_for_position(function, block, instruction_offset)
        else {
            return Ok(None);
        };
        self.return_destination_for_resume_point(resume_point)
    }
}

impl fmt::Debug for Executable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Executable")
            .field(
                "functions",
                &format!("<{} functions>", self.function_id_by_name.len()),
            )
            .field("frame_layouts", &self.frame_layouts.len())
            .field("resume_points", &self.resume_points.len())
            .field("resume_transfers", &self.resume_transfers.len())
            .field("safepoints", &self.safepoints.len())
            .field("materialization_maps", &self.materialization_maps.len())
            .finish_non_exhaustive()
    }
}

/// Build one executable from one MIR tree and immutable string pool.
struct ExecutableBuilder {
    tree: mir::NodeTree,
    strings: ImmutableStringPool,
    frame_layouts: Vec<engine::FrameLayout>,
    frame_layout_id_by_function: HashMap<mir::LocalNodeId<mir::Function>, engine::FrameLayoutId>,
    resume_points: Vec<engine::ResumePoint>,
    resume_transfers: Vec<engine::ResumeTransfer>,
    safepoints: Vec<engine::Safepoint>,
    safepoint_id_by_resume_point: HashMap<engine::ResumePointId, engine::SafepointId>,
    materialization_maps: Vec<engine::MaterializationMap>,
    resume_point_id_by_position: HashMap<
        (
            mir::LocalNodeId<mir::Function>,
            mir::LocalNodeId<mir::Block>,
            u32,
        ),
        engine::ResumePointId,
    >,
}

impl ExecutableBuilder {
    /// Create one executable builder.
    fn new(tree: mir::NodeTree, strings: ImmutableStringPool) -> Self {
        Self {
            tree,
            strings,
            frame_layouts: Vec::new(),
            frame_layout_id_by_function: HashMap::new(),
            resume_points: Vec::new(),
            resume_transfers: Vec::new(),
            safepoints: Vec::new(),
            safepoint_id_by_resume_point: HashMap::new(),
            materialization_maps: Vec::new(),
            resume_point_id_by_position: HashMap::new(),
        }
    }

    /// Build the executable.
    fn build(mut self) -> Result<Executable> {
        let function_id_by_name = self.build_function_id_by_name();
        let vtable_id_by_global = self.build_vtable_id_by_global();
        let (lowered_function_ids, target_by_id) = self.build_function_targets();
        let layouts = build_layouts(&self.tree)?;
        let functions = self.build_functions(&lowered_function_ids, &target_by_id, &layouts)?;
        let functions = FunctionTable::new(functions, target_by_id);

        Ok(Executable {
            tree: self.tree,
            strings: self.strings,
            function_id_by_name,
            vtable_id_by_global,
            frame_layouts: self.frame_layouts,
            frame_layout_id_by_function: self.frame_layout_id_by_function,
            resume_points: self.resume_points,
            resume_transfers: self.resume_transfers,
            safepoints: self.safepoints,
            safepoint_id_by_resume_point: self.safepoint_id_by_resume_point,
            materialization_maps: self.materialization_maps,
            layouts,
            resume_point_id_by_position: self.resume_point_id_by_position,
            functions,
        })
    }

    /// Build the function name lookup table.
    fn build_function_id_by_name(&self) -> HashMap<String, mir::LocalNodeId<mir::Function>> {
        let mut map = HashMap::new();

        for (function_id, function) in self.tree.iter_nodes::<mir::Function>() {
            let name = self.strings.get(function.name).to_string();
            map.entry(name).or_insert(function_id);
        }

        map
    }

    /// Build a lookup table from vtable globals to vtable ids.
    fn build_vtable_id_by_global(&self) -> HashMap<mir::LocalNodeId<mir::Global>, mir::VtableId> {
        let mut map = HashMap::new();

        for (table_id, table) in self.tree.metadata.dispatch.iter_vtables() {
            let mir::VtableStorage::Global(global) = table.storage;
            map.insert(global, table_id);
        }

        map
    }

    /// Build the lowered function order and callable target map.
    fn build_function_targets(
        &self,
    ) -> (
        Vec<mir::LocalNodeId<mir::Function>>,
        HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
    ) {
        let mut lowered_function_ids = Vec::new();
        let mut target_by_id = HashMap::new();

        // collect imported and lowerable callables
        for (function_id, function) in self.tree.iter_nodes::<mir::Function>() {
            // imported functions stay as import targets
            if function.is_import() {
                target_by_id.insert(function_id, CallTarget::Import);
                continue;
            }

            // declarations without bodies are not executable call targets
            if function.entry.is_none() {
                continue;
            }

            lowered_function_ids.push(function_id);
        }

        // assign stable lowered indices in build order
        for (index, function_id) in lowered_function_ids.iter().enumerate() {
            target_by_id.insert(*function_id, CallTarget::Lowered(index as u32));
        }

        (lowered_function_ids, target_by_id)
    }

    /// Build the lowered functions and append their execution metadata.
    fn build_functions(
        &mut self,
        lowered_function_ids: &[mir::LocalNodeId<mir::Function>],
        target_by_id: &HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
        layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ) -> Result<Vec<Function>> {
        let call_targets = target_by_id.clone();

        let mut functions = Vec::with_capacity(lowered_function_ids.len());

        // build one executable function at a time
        for function_id in lowered_function_ids {
            let function = self.build_function(*function_id, &call_targets, layouts)?;
            functions.push(function);
        }

        Ok(functions)
    }

    /// Build one lowered function and append its execution metadata.
    fn build_function(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        call_targets: &HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
        layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ) -> Result<Function> {
        let function = self.tree.get(function_id);
        let (value_slots, deferred_block_params) =
            analyze_lowered_value_slots(&self.tree, function)?;

        // derive the logical frame shape before lowering
        let frame_layout = self.build_frame_layout(function_id, function, &value_slots)?;
        let liveness = { mir::FunctionLiveness::build(function, &self.tree) };
        let (yield_resume_points, exceptional_call_resume_points) =
            self.build_resume_points(function_id, &frame_layout, &liveness)?;

        // lower the function with the preassigned yield resume ids
        let lowered_function = lower_function(
            &self.tree,
            function_id,
            frame_layout.id,
            &yield_resume_points,
            &exceptional_call_resume_points,
            call_targets,
            layouts,
            &value_slots,
            &deferred_block_params,
        )?
        .ok_or_else(|| Error::ConcreteMirRequired {
            context: format!("executable function {function_id:?}"),
        })?;

        // append the frame shape and yield resume transfers first
        self.frame_layout_id_by_function
            .insert(function_id, frame_layout.id);
        self.frame_layouts.push(frame_layout.clone());

        // append the generic lowered pc resume points
        for block in &lowered_function.blocks {
            for (instruction_offset, mir_instruction_offset) in
                block.mir_instruction_offsets.iter().copied().enumerate()
            {
                let resume_point_id = engine::ResumePointId(self.resume_points.len() as u32);
                let resume_point = engine::ResumePoint {
                    id: resume_point_id,
                    function: function_id,
                    frame_layout: frame_layout.id,
                    block: block.mir_block,
                    instruction_offset: instruction_offset as u32,
                    mir_instruction_offset,
                    transfer: None,
                };

                self.resume_point_id_by_position.insert(
                    (function_id, block.mir_block, instruction_offset as u32),
                    resume_point_id,
                );
                self.append_resume_point(&frame_layout, &liveness, &resume_point)?;
            }
        }

        Ok(lowered_function)
    }

    /// Build one logical frame layout for one function.
    fn build_frame_layout(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
        function: &mir::Function,
        value_slots: &[LoweredValueSlot],
    ) -> Result<engine::FrameLayout> {
        let mut slots = Vec::new();

        // value slots
        let value_start = slots.len() as u32;
        for slot in value_slots {
            slots.push(engine::FrameSlot {
                kind: engine::FrameSlotKind::Value,
                source: slot.source,
                ty: slot.ty,
                value_class: frame_slot_value_class_from_type(&self.tree, slot.ty),
            });
        }
        let value_slots = value_start..slots.len() as u32;

        // local slots
        let local_start = slots.len() as u32;
        for local_id in &function.locals {
            let local = self.tree.get(*local_id);
            let local_type = (local.ty).ty().ok_or_else(|| Error::ConcreteMirRequired {
                context: "frame local type".to_string(),
            })?;
            slots.push(engine::FrameSlot {
                kind: engine::FrameSlotKind::Local,
                source: engine::FrameSlotSource::Local(*local_id),
                ty: local_type,
                value_class: frame_slot_value_class_from_type(&self.tree, local_type),
            });
        }
        let local_slots = local_start..slots.len() as u32;

        // function environment slot
        let environment_slot = (function.environment)
            .map(|ty| {
                ty.ty().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "environment".to_string(),
                })
            })
            .transpose()?
            .map(|environment| {
                let slot = slots.len() as u32;
                slots.push(engine::FrameSlot {
                    kind: engine::FrameSlotKind::Environment,
                    source: engine::FrameSlotSource::Environment,
                    ty: environment,
                    value_class: frame_slot_value_class_from_type(&self.tree, environment),
                });
                slot
            });

        Ok(engine::FrameLayout {
            id: engine::FrameLayoutId(self.frame_layouts.len() as u32),
            function: function_id,
            slots,
            value_slots,
            local_slots,
            environment_slot,
        })
    }

    /// Build the semantic resume lookups for one function.
    fn build_resume_points(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        frame_layout: &engine::FrameLayout,
        liveness: &mir::FunctionLiveness,
    ) -> Result<(
        HashMap<mir::LocalNodeId<mir::Block>, engine::ResumePointId>,
        HashMap<mir::LocalNodeId<mir::Block>, (engine::ResumePointId, engine::ResumePointId)>,
    )> {
        let block_ids = self.tree.get(function_id).blocks.clone();
        let mut yield_resume_points = HashMap::new();
        let mut exceptional_call_resume_points = HashMap::new();

        // assign semantic resume points to suspension and exceptional call edges
        for block_id in block_ids {
            let yield_edge = {
                let block = self.tree.get(block_id);
                let terminator = self.tree.get(block.terminator);

                match terminator {
                    mir::Terminator::Yield { resume, .. } => Some((
                        (resume.block)
                            .block()
                            .ok_or_else(|| Error::ConcreteMirRequired {
                                context: "yield resume target".to_string(),
                            })?,
                        resume
                            .arguments
                            .iter()
                            .map(|argument| {
                                (*argument)
                                    .value()
                                    .ok_or_else(|| Error::ConcreteMirRequired {
                                        context: "yield resume argument".to_string(),
                                    })
                            })
                            .collect::<Result<Vec<_>>>()?,
                    )),
                    _ => None,
                }
            };

            // yield resumes into one single continuation block
            if let Some((resume, resume_arguments)) = yield_edge {
                // yield resumes may bind one trailing resume value
                let resume_value = self.infer_resume_value(resume, resume_arguments.len())?;
                let resume_point_id = self.append_resume_entry(
                    function_id,
                    frame_layout,
                    liveness,
                    resume,
                    &resume_arguments,
                    resume_value,
                )?;

                yield_resume_points.insert(block_id, resume_point_id);
                continue;
            }

            let exceptional_call_edge = {
                let block = self.tree.get(block_id);
                let terminator = self.tree.get(block.terminator);
                match terminator {
                    mir::Terminator::Invoke {
                        normal_target,
                        unwind_target,
                        ..
                    }
                    | mir::Terminator::InvokeIndirect {
                        normal_target,
                        unwind_target,
                        ..
                    }
                    | mir::Terminator::InvokeVirtual {
                        normal_target,
                        unwind_target,
                        ..
                    }
                    | mir::Terminator::InvokeInterface {
                        normal_target,
                        unwind_target,
                        ..
                    } => Some((
                        (normal_target.block).block().ok_or_else(|| {
                            Error::ConcreteMirRequired {
                                context: "invoke normal target".to_string(),
                            }
                        })?,
                        normal_target
                            .arguments
                            .iter()
                            .map(|argument| {
                                (*argument)
                                    .value()
                                    .ok_or_else(|| Error::ConcreteMirRequired {
                                        context: "invoke normal argument".to_string(),
                                    })
                            })
                            .collect::<Result<Vec<_>>>()?,
                        (unwind_target.block).block().ok_or_else(|| {
                            Error::ConcreteMirRequired {
                                context: "invoke unwind target".to_string(),
                            }
                        })?,
                        unwind_target
                            .arguments
                            .iter()
                            .map(|argument| {
                                (*argument)
                                    .value()
                                    .ok_or_else(|| Error::ConcreteMirRequired {
                                        context: "invoke unwind argument".to_string(),
                                    })
                            })
                            .collect::<Result<Vec<_>>>()?,
                    )),
                    _ => None,
                }
            };

            // exceptional call continuations branch to one normal or unwind resume point
            if let Some((normal_target, normal_arguments, unwind_target, unwind_arguments)) =
                exceptional_call_edge
            {
                // normal and unwind edges may each bind one trailing implicit value
                let normal_resume_value =
                    self.infer_resume_value(normal_target, normal_arguments.len())?;
                let normal_resume_point_id = self.append_resume_entry(
                    function_id,
                    frame_layout,
                    liveness,
                    normal_target,
                    &normal_arguments,
                    normal_resume_value,
                )?;

                let unwind_resume_value =
                    self.infer_resume_value(unwind_target, unwind_arguments.len())?;
                let unwind_resume_point_id = self.append_resume_entry(
                    function_id,
                    frame_layout,
                    liveness,
                    unwind_target,
                    &unwind_arguments,
                    unwind_resume_value,
                )?;

                exceptional_call_resume_points
                    .insert(block_id, (normal_resume_point_id, unwind_resume_point_id));
            }
        }

        Ok((yield_resume_points, exceptional_call_resume_points))
    }

    /// Return the trailing implicit resume value for one edge when present.
    fn infer_resume_value(
        &self,
        block: mir::LocalNodeId<mir::Block>,
        explicit_argument_count: usize,
    ) -> Result<Option<mir::Value>> {
        let resume_block = self.tree.get(block);

        // block edges bind the implicit transferred value after explicit arguments
        if resume_block.parameters.len() == explicit_argument_count + 1 {
            return resume_block
                .parameters
                .last()
                .map(|parameter| {
                    (parameter.value)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "resume parameter".to_string(),
                        })
                })
                .transpose();
        }

        Ok(None)
    }

    /// Append one semantic resume point and transfer for one block entry.
    fn append_resume_entry(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        frame_layout: &engine::FrameLayout,
        liveness: &mir::FunctionLiveness,
        block: mir::LocalNodeId<mir::Block>,
        arguments: &[mir::Value],
        resume_value: Option<mir::Value>,
    ) -> Result<engine::ResumePointId> {
        let resume_block = self.tree.get(block);
        let copy_parameters = if resume_value.is_some() {
            &resume_block.parameters[..resume_block.parameters.len() - 1]
        } else {
            &resume_block.parameters[..]
        };
        let copies = copy_parameters
            .iter()
            .zip(arguments.iter())
            .map(|(parameter, argument)| {
                let destination =
                    (parameter.value)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "resume parameter".to_string(),
                        })?;

                Ok(engine::ResumeCopy {
                    source: argument.0,
                    destination: destination.0,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let transfer_id = engine::ResumeTransferId(self.resume_transfers.len() as u32);
        self.resume_transfers.push(engine::ResumeTransfer {
            id: transfer_id,
            copies,
            resume_value: resume_value.map(|value| value.0),
        });

        let resume_point_id = engine::ResumePointId(self.resume_points.len() as u32);
        let resume_point = engine::ResumePoint {
            id: resume_point_id,
            function: function_id,
            frame_layout: frame_layout.id,
            block,
            instruction_offset: 0,
            mir_instruction_offset: 0,
            transfer: Some(transfer_id),
        };

        self.append_resume_point(frame_layout, liveness, &resume_point)?;

        Ok(resume_point_id)
    }

    /// Append one semantic resume point and its attached metadata.
    fn append_resume_point(
        &mut self,
        frame_layout: &engine::FrameLayout,
        liveness: &mir::FunctionLiveness,
        resume_point: &engine::ResumePoint,
    ) -> Result<()> {
        let safepoint_id = engine::SafepointId(self.safepoints.len() as u32);
        let materialization_map_id =
            engine::MaterializationMapId(self.materialization_maps.len() as u32);
        let materialized_values = self.materialized_values(liveness, resume_point)?;
        let materialized_locals = self.materialized_locals(liveness, resume_point);
        let slots = frame_layout
            .slots
            .iter()
            .enumerate()
            .map(|(slot_index, _slot)| engine::MaterializationSlot {
                slot: slot_index as u32,
                value: if self.is_materialized_slot(
                    frame_layout,
                    slot_index as u32,
                    &materialized_values,
                    &materialized_locals,
                ) {
                    engine::MaterializationValue::FrameSlot(slot_index as u32)
                } else {
                    engine::MaterializationValue::Undefined
                },
            })
            .collect();

        self.materialization_maps.push(engine::MaterializationMap {
            id: materialization_map_id,
            safepoint: safepoint_id,
            frames: vec![engine::MaterializationFrame {
                frame_layout: frame_layout.id,
                resume_point: resume_point.id,
                slots,
            }],
        });

        self.safepoints.push(engine::Safepoint {
            id: safepoint_id,
            function: resume_point.function,
            frame_layout: frame_layout.id,
            resume_point: resume_point.id,
            stack_map: None,
            materialization_map: Some(materialization_map_id),
        });
        self.safepoint_id_by_resume_point
            .insert(resume_point.id, safepoint_id);
        self.resume_points.push(resume_point.clone());

        Ok(())
    }

    /// Return the values materialized at one resume point.
    fn materialized_values(
        &self,
        liveness: &mir::FunctionLiveness,
        resume_point: &engine::ResumePoint,
    ) -> Result<HashSet<mir::Value>> {
        let Some(resume_transfer) = resume_point
            .transfer
            .and_then(|resume_transfer| self.resume_transfers.get(resume_transfer.0 as usize))
        else {
            return Ok(liveness.value_live_before_instruction(
                &self.tree,
                resume_point.block,
                resume_point.mir_instruction_offset as usize,
            ));
        };

        // materialize live in values that survive the resume edge
        let live_in = liveness.value_live_in(resume_point.block);
        let resume_block = self.tree.get(resume_point.block);
        let parameter_values: HashSet<mir::Value> = resume_block
            .parameters
            .iter()
            .map(|parameter| {
                (parameter.value)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "resume block parameter".to_string(),
                    })
            })
            .collect::<Result<HashSet<_>>>()?;
        let mut values: HashSet<mir::Value> =
            live_in.difference(&parameter_values).copied().collect();

        for copy in &resume_transfer.copies {
            let destination = mir::Value::new(copy.destination);
            let source = mir::Value::new(copy.source);

            if live_in.contains(&destination) {
                values.insert(source);
            }
        }

        Ok(values)
    }

    /// Return the locals materialized at one resume point.
    fn materialized_locals(
        &self,
        liveness: &mir::FunctionLiveness,
        resume_point: &engine::ResumePoint,
    ) -> HashSet<mir::LocalNodeId<mir::Local>> {
        liveness.local_live_in(resume_point.block).clone()
    }

    /// Return whether one frame slot is materialized at this resume point.
    fn is_materialized_slot(
        &self,
        layout: &engine::FrameLayout,
        slot: u32,
        materialized_values: &HashSet<mir::Value>,
        materialized_locals: &HashSet<mir::LocalNodeId<mir::Local>>,
    ) -> bool {
        // value slots
        if layout.value_slots.contains(&slot) {
            let Some(value_slot) = layout.slot(slot) else {
                return false;
            };

            return match value_slot.source {
                engine::FrameSlotSource::Value(value)
                | engine::FrameSlotSource::DisaggregatedValue(value) => {
                    materialized_values.contains(&value)
                }
                _ => false,
            };
        }

        // local slots
        if layout.local_slots.contains(&slot) {
            let local = mir::LocalNodeId::new(slot - layout.local_slots.start);
            return materialized_locals.contains(&local);
        }

        // function environment slot
        layout.environment_slot == Some(slot)
    }
}
