from abc import ABC
from typing import TYPE_CHECKING, Any, ClassVar, override

import structlog
from opentelemetry import trace

from bench.language import (
    Action,
    ActionType,
    Agent,
    Aliasing,
    BreakpointScope,
    BreakpointSite,
    CustomObject,
    IsType,
    Runnable,
    RunOptions,
    RunType,
    Span,
    SpanType,
    code,
)
from bench.runtime.core import (
    ATTEMPT_ONCE,
    RunIn,
    Runner,
    Runtime,
    make_runner,
)

if TYPE_CHECKING:
    from .flow import FlowRunner


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

CODE_PASS = code("pass")


class ActionRunner(Runner[Action], ABC):
    """Action Runner."""

    runner_type: ClassVar[RunType] = RunType.ACTION

    def __init__(
        self,
        *,
        runtime: Runtime,
        node: Action,
        run: RunIn,
        options: RunOptions,
        agent: Agent | None = None,
        parent: Runner | None = None,
        inputs: CustomObject | None = None,
        outputs: IsType | CustomObject | None = None,
        flow: "FlowRunner | None" = None,
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
                if bp.scope == BreakpointScope.ACTION and bp.site in sites:
                    return True
        return False

    def _get_resumable_subrunner(
        self,
        node: Agent | Runnable,
        inputs: CustomObject | None,
        output_type: IsType | None = None,
    ):
        """
        Gets the Runner for the given Node within this Runner.
        If there is an interrupted Run inside the current Run for the node, we'll just return that.
        NOTE: we assume there will only be one Runner for a given RunAttempt and Node.
        """
        assert self.tracked_run is not None, f"{self!r} must be tracked"

        if isinstance(node, Agent):
            runnable = node.main_flow
            assert runnable is not None, f"{node!r} must have a default Flow"
            agent = node
        else:
            runnable = node
            agent = None

        for run in self.tracked_run.runs:
            # try to resume interrupted Run
            if (
                run.status.is_interrupted
                and run.runnable == runnable
                and (agent is None or run.agent_id == agent.id)
            ):
                return self.runtime.restore_runner(run)
        else:
            # make new Runner
            runner = make_runner(
                runtime=self.runtime,
                node=runnable,
                run="track",
                inputs=inputs,
                agent=agent,
                outputs=output_type,
            )
            return runner


class StartActionRunner(ActionRunner):
    @override
    async def run(self) -> None:
        pass  # nothing to do


class EndActionRunner(ActionRunner):
    @override
    async def run(self) -> None:
        if self.flow is not None:
            self.flow._complete(outputs=self.inputs)


class ToolActionRunner(ActionRunner):
    @override
    async def run(self) -> None:
        assert self.tracked_run is not None, f"{self!r} must be in a Run"
        if self.node.tool_ptr is not None:
            # run static tool
            tool = self.node.tool
            assert tool is not None, f"{self.node!r} is missing {self.node.tool_ptr!r}"
            tool_runner: Runner[Any] = self._get_resumable_subrunner(
                node=tool, inputs=self.inputs, output_type=self.output_type
            )
            await self.runtime.run_runner(tool_runner)
            self.outputs = tool_runner.outputs
        else:
            # run dynamic tool (from Task)
            task = self.tracked_run.run_task
            assert task is not None, f"{self!r} must have a Task (as tool is not provided)"
            tool_ptr = task.tool_ptr
            assert tool_ptr is not None, f"{task!r} for {self!r} must have a tool"
            tool = task.target
            assert tool is not None, f"{task!r} for {self!r} is missing {tool_ptr!r}"
            tool_runner: Runner[Any] = self._get_resumable_subrunner(
                node=tool, inputs=task.value, output_type=self.output_type
            )
            await self.runtime.run_runner(tool_runner)


class CodeActionRunner(ActionRunner):
    @override
    async def run(self) -> None:
        assert self.tracked_run is not None, f"{self!r} must be in a Run"
        from bench.runtime.code import CodeFunctionRunner

        # try to resume interrupted span
        resumed_span: Span | None = None
        resumed_runner: Runner | None = None
        for span in self.tracked_run.spans:
            if (
                span.status.is_interrupted
                and span.type == SpanType.CODE
                and span.action_id == self.node.id
            ):
                resumed_span = span
                resumed_runner = self.runtime.get_runner(resumed_span)
                break

        # run code
        code = (resumed_span.code if resumed_span else self.node.code) or CODE_PASS
        if resumed_runner is not None:
            code_runner = resumed_runner
        else:
            code_runner = CodeFunctionRunner(
                runtime=self.runtime,
                node=self.node,
                options=ATTEMPT_ONCE,
                run=resumed_span or SpanType.CODE,
                code=code,
                aliasing=Aliasing(),
                inputs=self.inputs,
                outputs=self.outputs or self.output_type,
            )
        self.tracked.code = code
        if self.tracked_run is not None and self.tracked_run is not self.tracked:
            self.tracked_run.code = code
        await self.runtime.run_runner(code_runner)
        self.outputs = code_runner.outputs


class DoActionRunner(ActionRunner):
    @override
    async def run(self) -> None:
        from bench.builtin import BenchFlow

        # run main flow
        flow_runner = self._get_resumable_subrunner(
            node=BenchFlow,
            inputs=self.inputs,
            output_type=self.output_type,
        )
        try:
            await self.runtime.run_runner(flow_runner)
        finally:
            self.outputs = flow_runner.outputs


ACTION_RUNNER_BY_ACTION_TYPE: dict[ActionType, type[ActionRunner]] = {
    # flow
    ActionType.START: StartActionRunner,
    ActionType.END: EndActionRunner,
    ActionType.TOOL: ToolActionRunner,
    ActionType.CODE: CodeActionRunner,
    ActionType.DO: DoActionRunner,
}
