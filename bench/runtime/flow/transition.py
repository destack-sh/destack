from abc import ABC
from typing import TYPE_CHECKING, ClassVar, override

import structlog
from opentelemetry import trace

from bench.language import Agent, CustomObject, FlowEdge, RunType, TypeBase
from bench.runtime.core import RunIn, Runner, Runtime

if TYPE_CHECKING:
    from .flow import FlowRunner

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class TransitionRunner(Runner[FlowEdge], ABC):
    runner_type: ClassVar[RunType] = RunType.TRANSITION

    def __init__(
        self,
        *,
        runtime: Runtime,
        node: FlowEdge,
        run: RunIn,
        flow: "FlowRunner | None" = None,
        parent: Runner | None = None,
        agent: Agent | None = None,
        inputs: CustomObject | None = None,
        outputs: TypeBase | CustomObject | None = None,
    ) -> None:
        super().__init__(
            runtime=runtime,
            node=node,
            parent=parent,
            agent=agent,
            inputs=inputs,
            outputs=outputs,
            run=run,
        )
        self.flow = flow

    @override
    async def run(self) -> None:
        pass
