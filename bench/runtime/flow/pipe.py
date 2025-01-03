from abc import ABC
from typing import TYPE_CHECKING, ClassVar, override

import structlog
from opentelemetry import trace

from bench.language import (
    BreakpointScope,
    BreakpointSite,
    Context,
    CustomObject,
    ObjectKind,
    Pipe,
    PipeType,
    Run,
    RunOptions,
    RunType,
    TypeBase,
)
from bench.runtime.core import Runner, Runtime

if TYPE_CHECKING:
    from .flow import FlowRunnerBase

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

#
# Pipes
#


class PipeRunnerBase(Runner[Pipe], ABC):
    kind: ClassVar[RunType] = RunType.PIPE

    def __init__(
        self,
        *,
        runtime: Runtime,
        node: Pipe,
        track: bool,
        options: RunOptions,
        context: Context,
        flow: "FlowRunnerBase | None" = None,
        parent: Runner | None = None,
        variables: CustomObject | None = None,
        inputs: CustomObject | None = None,
        output_type: TypeBase | None = None,
        run: Run | None = None,
    ) -> None:
        super().__init__(
            runtime=runtime,
            node=node,
            track=track,
            options=options,
            context=context,
            parent=parent,
            variables=variables,
            inputs=inputs,
            output_type=output_type,
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
        if self.output_type is not None:
            # assemble/map inputs from incoming Runs/Context :PipeMapping
            self.outputs = CustomObject.new(
                ObjectKind.INPUT, {}, self.output_type, supergraph=self.runtime.session._supergraph
            )
            if self.inputs is not None:
                for field in self.outputs._type._fields:
                    key = self.inputs._get_key(field.name)
                    if key is not None:
                        self.outputs[field] = self.inputs._do_get(key)


class ForwardPipeRunner(PipeRunnerBase):
    pass


class ForwardAndBackPipeRunner(PipeRunnerBase):
    pass


class SelectPipeRunner(PipeRunnerBase):
    pass


class SelectAndBackPipeRunner(PipeRunnerBase):
    pass


PIPE_RUNNER_BY_PIPE_TYPE: dict[PipeType, type[PipeRunnerBase]] = {
    PipeType.FORWARD: ForwardPipeRunner,
    PipeType.FORWARD_AND_BACK: ForwardAndBackPipeRunner,
    PipeType.SELECT: SelectPipeRunner,
    PipeType.SELECT_AND_BACK: SelectAndBackPipeRunner,
}
