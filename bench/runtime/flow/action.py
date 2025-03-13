from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Any, ClassVar, cast, final, override

import structlog
from opentelemetry import trace

from bench.language import (
    Action,
    ActionType,
    Aliasing,
    BreakpointScope,
    BreakpointSite,
    CustomObject,
    IsRuntime,
    ModelDeveloper,
    ModelType,
    RunnableNode,
    RunOptions,
    RunSpanType,
    RunType,
    TypeBase,
    TypeKind,
    code,
)
from bench.runtime.core import (
    ATTEMPT_ONCE,
    NotSupportedError,
    RunImpossibleError,
    RunIn,
    Runner,
    Runtime,
    make_runner,
)
from bench.runtime.model.chat import get_chat_model_runner_cls

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
        context: IsRuntime,
        parent: Runner | None = None,
        resources: CustomObject | None = None,
        inputs: CustomObject | None = None,
        outputs: TypeBase | CustomObject | None = None,
        flow: "FlowRunner | None" = None,
    ) -> None:
        super().__init__(
            runtime=runtime,
            node=node,
            options=options,
            context=context,
            parent=parent,
            resources=resources,
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


class StaticActionRunner(ActionRunner):
    @final
    @override
    async def run(self) -> None:
        """Run the static and the dynamic parts of the Action."""
        # static implementation
        await self.run_static()

        # dynamic calls (if needed)
        has_plan = (
            not self.node.type.is_boundary
            and self.outputs is not None
            and self.outputs._type.kind == TypeKind.PARTIAL_OBJECT
            and (getattr(self.outputs, "plans", None))
        )
        if (
            self.flow is not None
            and not has_plan
            and any(p.source_id == self.node.id and not p.is_manual for p in self.flow.node.links)
        ):
            model_developer = ModelDeveloper.OPENAI
            model_type = ModelType.OPENAI_GPT4_5
            model_runner_cls = get_chat_model_runner_cls(
                model_developer=model_developer, model_type=model_type
            )
            model_runner = model_runner_cls(
                runtime=self.runtime,
                node=self.node,
                model_type=model_type,
                options=self.options,
                context=self.context,
                resources=self.resources,
                inputs=self.inputs,
                outputs=self.outputs or self.output_type,
                parent=cast(Runner[Any], self),
                run=RunSpanType.FLOW_PLAN,
            )
            try:
                await self.runtime.run_runner(model_runner)
            finally:
                if model_runner.code is not None:
                    # track code in both RunSpan and Run
                    self.tracked.code = model_runner.code
                    if self.tracked_run is not None and self.tracked_run is not self.tracked:
                        self.tracked_run.code = model_runner.code

    @abstractmethod
    async def run_static(self) -> None:
        """Run the static part of the Action."""
        ...


class DynamicActionRunner(ActionRunner):
    @final
    @override
    async def run(self) -> None:
        from bench.runtime.model import (
            AnthropicChatModelRunner,
            GeminiChatModelRunner,
            OpenaiChatModelRunner,
        )

        # determine model
        model_developer = self.options.model_developer or ModelDeveloper.OPENAI
        if model_developer == ModelDeveloper.OPENAI:
            model_runner_cls = OpenaiChatModelRunner
            model_type = self.options.model_type or ModelType.OPENAI_GPT4_5
        elif model_developer == ModelDeveloper.ANTHROPIC:
            model_runner_cls = AnthropicChatModelRunner
            model_type = self.options.model_type or ModelType.ANTHROPIC_CLAUDE_3_7_SONNET
        elif model_developer == ModelDeveloper.GOOGLE:
            model_runner_cls = GeminiChatModelRunner
            model_type = self.options.model_type or ModelType.GOOGLE_GEMINI_2_0_FLASH
        else:
            raise NotSupportedError(f"unsupported model developer {model_developer!r}")

        # run model
        # nocheckin: implicit Flow for 'dynamic' Actions (Package.main_flow)
        #  wait isn't there a turtles problem with implementing DynamicActions using dynamic actions?
        #  or it just a Receive with a Tool in the middle?
        #  or maybe just leave Package.main_flow for another 'default Flow'
        #   and actually implement dynamic Actions right here somehow?
        model_runner = model_runner_cls(
            runtime=self.runtime,
            node=self.node,
            model_type=model_type,
            options=self.options,
            context=self.context,
            resources=self.resources,
            inputs=self.inputs,
            outputs=self.outputs or self.output_type,
            parent=cast(Runner[Any], self),
            run=RunSpanType.MODEL_GENERATE,
        )
        try:
            await self.runtime.run_runner(model_runner)
        finally:
            if model_runner.code is not None:
                # track code in both RunSpan and Run
                self.tracked.code = model_runner.code
                if self.tracked_run is not None and self.tracked_run is not self.tracked:
                    self.tracked_run.code = model_runner.code
            self.outputs = model_runner.outputs


#
# Flow
#


class StartActionRunner(StaticActionRunner):
    @override
    async def run_static(self) -> None:
        pass  # nothing to do


class ReceiveActionRunner(StaticActionRunner):
    @override
    async def run_static(self) -> None:
        pass  # nothing to do?


class CompleteActionRunner(ActionRunner):
    @override
    async def run(self) -> None:
        if self.flow is not None:
            self.flow._complete(outputs=self.outputs)


#
# Tool
#


class CodeActionRunner(ActionRunner):
    # NOTE: Code runner is static and must generate its own Plans (i.e. we don't auto-generate them)
    #  (because this is only used for manual stuff; therefore we don't inherit StaticActionRunner)

    @override
    async def run(self) -> None:
        from bench.runtime.code import CodeFunctionRunner

        code = self.node.code or CODE_PASS
        code_runner = CodeFunctionRunner(
            runtime=self.runtime,
            node=self.node,
            options=ATTEMPT_ONCE,
            context=self.context,
            run=RunSpanType.DELEGATE,
            code=code,
            aliasing=Aliasing(),
            resources=self.resources,
            inputs=self.inputs,
            outputs=self.outputs or self.output_type,
        )
        self.tracked.code = code
        if self.tracked_run is not None and self.tracked_run is not self.tracked:
            self.tracked_run.code = code
        await self.runtime.run_runner(code_runner)
        self.outputs = code_runner.outputs


class ToolActionRunner(StaticActionRunner):
    def _get_resumable_subrunner(
        self,
        node: RunnableNode,
        resources: CustomObject | None,
        inputs: CustomObject | None,
        output_type: TypeBase | None = None,
    ):
        """
        Gets the Runner for the given Node within this Runner.
        If there is an interrupted Run inside the current Run for the node, we'll just return that.
        NOTE: we assume there will only be one Runner for a given RunAttempt and Node.
        """
        assert self.tracked_run is not None, f"{self!r} must be tracked"
        for run in self.tracked_run.runs:
            # try to resume interrupted Run
            if run.status.is_interrupted and run.runnable == node:
                return self.runtime.restore_runner(run)
        else:
            # make new Runner
            runner = make_runner(
                runtime=self.runtime,
                node=node,
                run="track",
                context=self.context,
                resources=resources,
                inputs=inputs,
                outputs=output_type,
            )
            return runner

    @override
    async def run_static(self) -> None:
        tool = self.node.tool
        tool_type = self.node.type
        if tool is not None:
            # delegate to tool node
            tool_runner: Runner[Any] = self._get_resumable_subrunner(
                node=tool,
                # inputs/resources are both PartialAction with node=tool
                resources=self.resources,
                inputs=self.inputs,
                output_type=self.output_type,
            )
            await self.runtime.run_runner(tool_runner)
            self.outputs = tool_runner.outputs
        elif tool_type != ActionType.TOOL:  # if it's still tool it wasn't set
            # delegate to built-in action
            tool_runner_cls = ACTION_RUNNER_BY_ACTION_TYPE[tool_type]
            tool_runner: Runner[Any] = tool_runner_cls(
                runtime=self.runtime,
                node=self.node,
                run=RunSpanType.DELEGATE,
                options=self.options,
                context=self.context,
                parent=cast(Runner[Any], self),
                resources=self.resources,
                inputs=self.inputs,
                outputs=self.output_type or self.outputs,
                flow=self.flow,
            )
            await self.runtime.run_runner(tool_runner)
            self.outputs = tool_runner.outputs
        else:
            raise RunImpossibleError("no tool given")


ACTION_RUNNER_BY_ACTION_TYPE: dict[ActionType, type[ActionRunner]] = {
    # flow
    ActionType.START: StartActionRunner,
    ActionType.RECEIVE: ReceiveActionRunner,
    ActionType.COMPLETE: CompleteActionRunner,
    # tool
    ActionType.CODE: CodeActionRunner,
    ActionType.TOOL: ToolActionRunner,
    # dynamic
    ActionType.DO: DynamicActionRunner,
}
