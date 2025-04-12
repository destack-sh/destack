from abc import ABC
from typing import TYPE_CHECKING, ClassVar, override

import structlog
from opentelemetry import trace

from bench.language import (
    Agent,
    BreakpointScope,
    BreakpointSite,
    CustomObject,
    IsType,
    Link,
    LinkType,
    RunOptions,
    RunType,
)
from bench.runtime.core import RunIn, Runner, Runtime

if TYPE_CHECKING:
    from .flow import FlowRunner

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class LinkRunner(Runner[Link], ABC):
    runner_type: ClassVar[RunType] = RunType.LINK

    def __init__(
        self,
        *,
        runtime: Runtime,
        node: Link,
        options: RunOptions,
        run: RunIn,
        flow: "FlowRunner | None" = None,
        parent: Runner | None = None,
        agent: Agent | None = None,
        inputs: CustomObject | None = None,
        outputs: IsType | CustomObject | None = None,
    ) -> None:
        super().__init__(
            runtime=runtime,
            node=node,
            options=options,
            parent=parent,
            agent=agent,
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
                if bp.scope == BreakpointScope.LINK and bp.site in sites:
                    return True
        return False

    @override
    async def run(self) -> None:
        pass


LINK_RUNNER_BY_LINK_TYPE: dict[LinkType, type[LinkRunner]] = {
    LinkType.MANUAL: LinkRunner,
    LinkType.DECIDE: LinkRunner,
    LinkType.REQUIRE: LinkRunner,
}
