use destack_artifact as artifact;
use destack_core::Optional;
use destack_mir as mir;
use destack_program::{
    AllocationSite, CallDispatch, CallMode, CallSite, ContinuationSite, CounterSite, EdgeSite,
    MemoryAccess, MemorySite, ProgramPoint, SampleSite, SiteTableBuilder, Suspension,
    SuspensionSite,
};
use destack_source::ModuleId;

use super::{FrameLinker, ProgramLinker};
use crate::LinkResult;

/// Link object-local sites into one Program site table.
#[derive(Debug)]
pub(crate) struct SiteLinker<'a> {
    /// Dense Program identity projection.
    program: &'a ProgramLinker<'a>,
    /// Canonical frame state projection.
    frames: &'a FrameLinker<'a>,
}

impl<'a> SiteLinker<'a> {
    /// Create one site linker.
    pub(crate) fn new(program: &'a ProgramLinker<'a>, frames: &'a FrameLinker<'a>) -> Self {
        Self { program, frames }
    }

    /// Link every emitted site row.
    pub(crate) fn link(&self) -> LinkResult<SiteTableBuilder> {
        let mut allocations = Vec::new();
        let mut memory = Vec::new();
        let mut calls = Vec::new();
        let mut continuations = Vec::new();
        let mut edges = Vec::new();
        let mut suspensions = Vec::new();
        let mut counters = Vec::new();
        let mut samples = Vec::new();

        // project every object-local site into dense Program identities
        for (module, object) in self.program.objects() {
            // link allocation and memory operations
            allocations.extend(
                object
                    .allocations()
                    .iter()
                    .map(|site| self.allocation(*module, site))
                    .collect::<LinkResult<Vec<_>>>()?,
            );
            memory.extend(
                object
                    .memory()
                    .iter()
                    .map(|site| self.memory(*module, site)),
            );

            // link calls and continuation transfers
            calls.extend(
                object
                    .calls()
                    .iter()
                    .map(|site| self.call(*module, site))
                    .collect::<LinkResult<Vec<_>>>()?,
            );
            continuations.extend(
                object
                    .continuations()
                    .iter()
                    .map(|site| self.continuation(*module, site)),
            );

            // link edges and coroutine suspensions
            edges.extend(object.edges().iter().map(|site| self.edge(*module, site)));
            suspensions.extend(
                object
                    .suspensions()
                    .iter()
                    .map(|site| self.suspension(*module, site))
                    .collect::<LinkResult<Vec<_>>>()?,
            );

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
            .continuations(continuations)
            .edges(edges)
            .suspensions(suspensions)
            .counters(counters)
            .samples(samples))
    }

    /// Link one allocation site.
    fn allocation(
        &self,
        module: ModuleId,
        site: &artifact::AllocationSite,
    ) -> LinkResult<AllocationSite> {
        let object = self.program.object(module);
        let result_type = object
            .storage_type(site.result_type)
            .and_then(|ty| object.ty(ty))
            .ok_or_else(|| self.program.invalid_input("allocation has no result type"))?;
        let virtual_table = match result_type.definition {
            mir::Type::Reference { .. } => self.program.virtual_table_id(module, site.storage_type),
            _ => None,
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
    fn memory(&self, module: ModuleId, site: &artifact::MemorySite) -> MemorySite {
        MemorySite {
            point: self.point(module, site.point),
            access: match site.access {
                mir::MemoryOperation::Read => MemoryAccess::Read,
                mir::MemoryOperation::Write => MemoryAccess::Write,
                mir::MemoryOperation::ReadWrite => MemoryAccess::ReadWrite,
            },
            space: site.space,
            value_type: self.program.type_id(module, site.value_type),
        }
    }

    /// Link one call site.
    fn call(&self, module: ModuleId, site: &artifact::CallSite) -> LinkResult<CallSite> {
        let (dispatch, slot) = match site.dispatch {
            mir::CallDispatch::Direct => (CallDispatch::Direct, None),
            mir::CallDispatch::Indirect => (CallDispatch::Indirect, None),
            mir::CallDispatch::Virtual { slot } => (CallDispatch::Virtual, Some(slot.0)),
            mir::CallDispatch::Dynamic { slot } => (CallDispatch::Dynamic, Some(slot.0)),
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
            .ok_or_else(|| self.program.invalid_input("missing call signature"))?;

        Ok(CallSite {
            point: self.point(module, site.point),
            resume: Optional::from(site.resume.map(|point| self.point(module, point))),
            unwind: Optional::from(site.unwind.map(|point| self.point(module, point))),
            mode: match site.mode {
                artifact::CallMode::Return => CallMode::Return,
                artifact::CallMode::Tail => CallMode::Tail,
            },
            dispatch,
            space: Optional::from(site.space),
            target: Optional::from(target),
            dispatch_type: Optional::from(dispatch_type),
            signature,
            slot: Optional::from(slot),
        })
    }

    /// Link one continuation control site.
    fn continuation(
        &self,
        module: ModuleId,
        site: &artifact::ContinuationSite,
    ) -> ContinuationSite {
        ContinuationSite {
            point: self.point(module, site.point),
            yielded: self.point(module, site.yielded),
            returned: self.point(module, site.returned),
            unwind: Optional::from(site.unwind.map(|point| self.point(module, point))),
        }
    }

    /// Link one control-flow edge.
    fn edge(&self, module: ModuleId, site: &artifact::EdgeSite) -> EdgeSite {
        EdgeSite {
            source: self.point(module, site.source),
            target: self.point(module, site.target),
        }
    }

    /// Link one coroutine suspension site.
    fn suspension(
        &self,
        module: ModuleId,
        site: &artifact::SuspensionSite,
    ) -> LinkResult<SuspensionSite> {
        let frame_state = self
            .frames
            .state(module, artifact::FramePoint::operation(site.point))
            .ok_or_else(|| self.program.invalid_input("missing suspension frame state"))?;

        Ok(SuspensionSite {
            point: self.point(module, site.point),
            resume: self.point(module, site.resume),
            cancel: Optional::from(site.cancel.map(|point| self.point(module, point))),
            complete: Optional::from(site.complete.map(|point| self.point(module, point))),
            unwind: Optional::from(site.unwind.map(|point| self.point(module, point))),
            frame_state,
            operation: match site.operation {
                artifact::Suspension::Await => Suspension::Await,
                artifact::Suspension::Yield => Suspension::Yield,
            },
            value_type: self.program.type_id(module, site.value_type),
            resume_type: self.program.type_id(module, site.resume_type),
            complete_type: Optional::from(
                site.complete_type
                    .map(|ty| self.program.type_id(module, ty)),
            ),
        })
    }

    /// Link one explicit counter site.
    fn counter(&self, module: ModuleId, site: &artifact::CounterSite) -> CounterSite {
        CounterSite {
            point: self.point(module, site.point),
            counter: self
                .program
                .counter_id(module, site.point.function, site.counter),
        }
    }

    /// Link one explicit sample site.
    fn sample(&self, module: ModuleId, site: &artifact::SampleSite) -> SampleSite {
        SampleSite {
            point: self.point(module, site.point),
            sampler: self
                .program
                .sampler_id(module, site.point.function, site.sampler),
            value_type: self.program.type_id(module, site.value_type),
        }
    }

    /// Link one object-local point.
    fn point(&self, module: ModuleId, point: artifact::Point) -> ProgramPoint {
        ProgramPoint::new(
            self.program.function_id(module, point.function),
            point.operation,
        )
    }
}
