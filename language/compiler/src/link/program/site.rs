use std::collections::HashMap;
use std::sync::Arc;

use tspp_core::{Optional, StringId};
use tspp_mir as mir;
use tspp_program::{
    AllocationSite, AllocationSiteId, CallDispatch, CallMode, CallSite, CounterId, CounterSite,
    EdgeSite, FunctionId, MemoryAccess, MemorySite, Object, ProgramPoint, SampleSite, SamplerId,
    SiteTableBuilder, object,
};
use tspp_source::ModuleId;

use super::ProgramLinker;
use crate::{CompilerResult, invalid_program_input};

/// Link object-local sites into one Program site table.
#[derive(Debug)]
pub(crate) struct SiteLinker<'a> {
    /// Dense Program identity projection.
    program: &'a ProgramLinker<'a>,
}

impl<'a> SiteLinker<'a> {
    /// Create one site linker.
    pub(crate) fn new(program: &'a ProgramLinker<'a>) -> Self {
        Self { program }
    }

    /// Link every emitted site entry.
    pub(crate) fn link(&self) -> CompilerResult<SiteTableBuilder> {
        let mut allocations = Vec::new();
        let mut memory = Vec::new();
        let mut calls = Vec::new();
        let mut edges = Vec::new();
        let mut counters = Vec::new();
        let mut samples = Vec::new();

        // link canonical allocation sites in dense order
        for &(module, index) in self.program.allocation_sites() {
            let site = &self.program.object(module).allocations()[index as usize];
            allocations.push(self.allocation(module, site)?);
        }

        // project every object-local site into dense Program identities
        for (module, object) in self.program.objects() {
            // link memory operations
            memory.extend(
                object
                    .memory()
                    .iter()
                    .map(|site| self.memory(*module, site)),
            );

            // link calls and control-flow edges
            calls.extend(
                object
                    .calls()
                    .iter()
                    .map(|site| self.call(*module, site))
                    .collect::<CompilerResult<Vec<_>>>()?,
            );
            edges.extend(object.edges().iter().map(|site| self.edge(*module, site)));

            // link profile counters and samples
            counters.extend(
                object
                    .counters()
                    .iter()
                    .map(|site| self.counter(*module, site)),
            );
            samples.extend(
                object
                    .samples()
                    .iter()
                    .map(|site| self.sample(*module, site)),
            );
        }

        Ok(SiteTableBuilder::new()
            .allocations(allocations)
            .memory(memory)
            .calls(calls)
            .edges(edges)
            .counters(counters)
            .samples(samples))
    }

    /// Link one allocation site.
    fn allocation(
        &self,
        module: ModuleId,
        site: &object::AllocationSite,
    ) -> CompilerResult<AllocationSite> {
        // require the virtual table of a new class object
        let layout = self
            .program
            .object(module)
            .layouts()
            .type_layout(site.storage_type)
            .ok_or_else(|| invalid_program_input("allocation has no storage layout"))?;
        let virtual_table = match matches!(layout.shape, mir::LayoutShape::Class(_)) {
            true => Some(
                self.program
                    .virtual_table_id(module, site.storage_type)
                    .ok_or_else(|| {
                        invalid_program_input("class allocation has no virtual table")
                    })?,
            ),
            false => None,
        };

        Ok(AllocationSite {
            point: self.point(module, site.point),
            space: site.space,
            result_type: self.program.type_id(module, site.result_type),
            storage_type: self.program.type_id(module, site.storage_type),
            layout: self.program.layout_id(module, site.storage_type),
            virtual_table: Optional::from(virtual_table),
        })
    }

    /// Link one memory site.
    fn memory(&self, module: ModuleId, site: &object::MemorySite) -> MemorySite {
        MemorySite {
            point: self.point(module, site.point),
            access: match site.access {
                mir::MemoryOperation::Read => MemoryAccess::Read,
                mir::MemoryOperation::Write => MemoryAccess::Write,
                mir::MemoryOperation::ReadWrite => MemoryAccess::ReadWrite,
            },
            storage: Optional::from(site.storage),
            value_type: self.program.type_id(module, site.value_type),
        }
    }

    /// Link one call site.
    fn call(&self, module: ModuleId, site: &object::CallSite) -> CompilerResult<CallSite> {
        let (dispatch, slot) = match site.dispatch {
            mir::CallDispatch::Direct => (CallDispatch::Direct, None),
            mir::CallDispatch::Indirect => (CallDispatch::Indirect, None),
            mir::CallDispatch::Virtual { slot } => (CallDispatch::Virtual, Some(slot.0)),
            mir::CallDispatch::Dynamic { slot } => (CallDispatch::Dynamic, Some(slot.0)),
            mir::CallDispatch::Witness => unreachable!("linked calls resolve every witness"),
        };
        let target = site
            .target
            .map(|function| self.program.function_id(module, function));
        let dispatch_type = site
            .dispatch_type
            .map(|ty| self.program.type_id(module, ty));
        let signature = self
            .program
            .type_signature_id(module, site.signature)
            .ok_or_else(|| invalid_program_input("missing call signature"))?;

        Ok(CallSite {
            point: self.point(module, site.point),
            resume: Optional::from(site.resume.map(|point| self.point(module, point))),
            unwind: Optional::from(site.unwind.map(|point| self.point(module, point))),
            mode: match site.mode {
                object::CallMode::Return => CallMode::Return,
                object::CallMode::Tail => CallMode::Tail,
            },
            dispatch,
            space: Optional::from(site.space),
            target: Optional::from(target),
            dispatch_type: Optional::from(dispatch_type),
            signature,
            slot: Optional::from(slot),
        })
    }

    /// Link one control-flow edge.
    fn edge(&self, module: ModuleId, site: &object::EdgeSite) -> EdgeSite {
        EdgeSite {
            source: self.point(module, site.source),
            target: self.point(module, site.target),
        }
    }

    /// Intern one instrument name into the program strings.
    fn instrument_name(&self, name: Option<StringId>) -> Optional<StringId> {
        match name {
            Some(name) => {
                let text = self.program.strings().get(name).to_string();

                Optional::some(self.program.intern_string(&text))
            }
            None => Optional::none(),
        }
    }

    /// Link one explicit counter site.
    fn counter(&self, module: ModuleId, site: &object::CounterSite) -> CounterSite {
        CounterSite {
            point: self.point(module, site.point),
            counter: self
                .program
                .counter_id(module, site.point.function, site.counter),
            name: self.instrument_name(site.name),
        }
    }

    /// Link one explicit sample site.
    fn sample(&self, module: ModuleId, site: &object::SampleSite) -> SampleSite {
        SampleSite {
            point: self.point(module, site.point),
            sampler: self
                .program
                .sampler_id(module, site.point.function, site.sampler),
            value_type: self.program.type_id(module, site.value_type),
            name: self.instrument_name(site.name),
        }
    }

    /// Link one object-local point.
    fn point(&self, module: ModuleId, point: object::Point) -> ProgramPoint {
        ProgramPoint::new(
            self.program.function_id(module, point.function),
            point.operation,
        )
    }
}

impl SiteLinker<'_> {
    /// Assign dense allocation site ids in program point order.
    pub(crate) fn allocation_ids(
        objects: &[(ModuleId, Arc<Object>)],
        function_ids: &HashMap<(ModuleId, mir::FunctionId), FunctionId>,
        functions: &[(ModuleId, mir::FunctionId)],
    ) -> CompilerResult<(
        HashMap<(ModuleId, u32), AllocationSiteId>,
        Vec<(ModuleId, u32)>,
    )> {
        // group each canonical function's sites with their operation offsets
        let mut canonical = HashMap::<(ModuleId, mir::FunctionId), Vec<(u32, u32)>>::new();
        for (module, object) in objects {
            for (index, site) in object.allocations().iter().enumerate() {
                let key = (*module, site.point.function);
                let function = function_ids
                    .get(&key)
                    .ok_or_else(|| invalid_program_input("allocation site function is absent"))?;
                if functions[function.index()] == key {
                    canonical
                        .entry(key)
                        .or_default()
                        .push((site.point.operation, index as u32));
                }
            }
        }

        // number the canonical sites in function order, then operation order
        let mut by_operation = HashMap::<(FunctionId, u32), AllocationSiteId>::new();
        let mut sites = Vec::new();
        for (position, key) in functions.iter().enumerate() {
            let Some(indices) = canonical.get_mut(key) else {
                continue;
            };
            indices.sort_unstable();
            for &(operation, index) in indices.iter() {
                let id = AllocationSiteId(sites.len() as u32);
                by_operation.insert((FunctionId(position as u32), operation), id);
                sites.push((key.0, index));
            }
        }

        // resolve every site through its function's canonical copy
        let mut ids = HashMap::new();
        for (module, object) in objects {
            for (index, site) in object.allocations().iter().enumerate() {
                let function = function_ids[&(*module, site.point.function)];
                let id = by_operation
                    .get(&(function, site.point.operation))
                    .copied()
                    .ok_or_else(|| {
                        invalid_program_input("allocation site absent from its canonical function")
                    })?;
                ids.insert((*module, index as u32), id);
            }
        }

        Ok((ids, sites))
    }

    /// Assign contiguous Program counter and sampler ranges in function order.
    pub(crate) fn profile_starts(
        objects: &[(ModuleId, Arc<Object>)],
        functions: &[(ModuleId, mir::FunctionId)],
    ) -> CompilerResult<(
        HashMap<(ModuleId, mir::FunctionId), CounterId>,
        HashMap<(ModuleId, mir::FunctionId), SamplerId>,
    )> {
        let mut counter_starts = HashMap::new();
        let mut sampler_starts = HashMap::new();
        let mut counts = HashMap::new();
        let mut counter_start = 0u32;
        let mut sampler_start = 0u32;

        // initialize every object-local function profile range
        for (module, object) in objects {
            for function in object.functions() {
                counts.insert((*module, function.id), (0, 0));
            }

            // derive local counter widths from their semantic sites
            for site in object.counters() {
                let count = counts
                    .get_mut(&(*module, site.point.function))
                    .ok_or_else(|| invalid_program_input("counter function is absent"))?;
                let end = site
                    .counter
                    .0
                    .checked_add(1)
                    .ok_or_else(|| invalid_program_input("counter id overflow"))?;
                count.0 = count.0.max(end);
            }

            // derive local sampler widths from their semantic sites
            for site in object.samples() {
                let count = counts
                    .get_mut(&(*module, site.point.function))
                    .ok_or_else(|| invalid_program_input("sampler function is absent"))?;
                let end = site
                    .sampler
                    .0
                    .checked_add(1)
                    .ok_or_else(|| invalid_program_input("sampler id overflow"))?;
                count.1 = count.1.max(end);
            }
        }

        // assign one contiguous profile range to each canonical function
        for &(module, function) in functions {
            let (counter_count, sampler_count) = counts
                .get(&(module, function))
                .copied()
                .ok_or_else(|| invalid_program_input("missing bytecode function"))?;

            counter_starts.insert((module, function), CounterId(counter_start));
            sampler_starts.insert((module, function), SamplerId(sampler_start));
            counter_start = counter_start
                .checked_add(counter_count)
                .ok_or_else(|| invalid_program_input("counter id overflow"))?;
            sampler_start = sampler_start
                .checked_add(sampler_count)
                .ok_or_else(|| invalid_program_input("sampler id overflow"))?;
        }

        Ok((counter_starts, sampler_starts))
    }
}
