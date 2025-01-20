from abc import ABC
from typing import TYPE_CHECKING, ClassVar, override

import structlog
from opentelemetry import trace

from bench.language import (
    BreakpointScope,
    BreakpointSite,
    CustomObject,
    HasContext,
    Pipe,
    PipeType,
    RunOptions,
    RunType,
    TypeBase,
)
from bench.runtime.core import RunIn, Runner, Runtime

if TYPE_CHECKING:
    from .flow import FlowRunner

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

#
# Pipes
#


class PipeRunner(Runner[Pipe], ABC):
    runner_type: ClassVar[RunType] = RunType.ACTION

    def __init__(
        self,
        *,
        runtime: Runtime,
        node: Pipe,
        options: RunOptions,
        context: HasContext,
        run: RunIn,
        flow: "FlowRunner | None" = None,
        parent: Runner | None = None,
        variables: CustomObject | None = None,
        inputs: CustomObject | None = None,
        outputs: TypeBase | CustomObject | None = None,
    ) -> None:
        super().__init__(
            runtime=runtime,
            node=node,
            options=options,
            context=context,
            parent=parent,
            variables=variables,
            inputs=inputs,
            outputs=outputs,
            run=run,
        )
        self.flow = flow

    @override
    def _has_breakpoint_set(self, *sites: BreakpointSite):
        if super()._has_breakpoint_set(*sites):
            return True
        if self.flow is not None:
            for bp in self.flow.breakpoints:
                if bp.scope == BreakpointScope.PIPE and bp.site in sites:
                    return True
        return False

    @override
    async def run(self) -> None:
        if self.node.delay is not None:
            await self.runtime.oracle.sleep(self.node.delay.total_seconds())


class ForwardPipeRunner(PipeRunner):
    pass


class SelectPipeRunner(PipeRunner):
    pass


class SelectAndBackPipeRunner(PipeRunner):
    pass


PIPE_RUNNER_BY_PIPE_TYPE: dict[PipeType, type[PipeRunner]] = {
    PipeType.FORWARD: ForwardPipeRunner,
    PipeType.SELECT: SelectPipeRunner,
    PipeType.SELECT_AND_BACK: SelectAndBackPipeRunner,
}
