import asyncio
from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Any, ClassVar, Literal, cast, final, override

import structlog
from opentelemetry import trace
from playwright.async_api import Page as PlaywrightPage

from bench.language import (
    Action,
    ActionType,
    Aliasing,
    BreakpointScope,
    BreakpointSite,
    Browser,
    ClickAction,
    CodeAction,
    CompleteAction,
    CreateAction,
    CustomObject,
    DeleteAction,
    DuplicateAction,
    FailAction,
    FileFormat,
    FileType,
    GetAction,
    GoBackwardAction,
    GoForwardAction,
    GoToTabAction,
    GoToUrlAction,
    HasApplicationContext,
    HasContext,
    HasNodeBase,
    InterruptionType,
    LookAction,
    ModelDeveloper,
    ModelType,
    NodeType,
    PipeType,
    PressAction,
    RunnableNode,
    RunOptions,
    RunSpanType,
    RunType,
    ScrollAction,
    SearchAction,
    SelectAction,
    Text,
    ToolAction,
    TypeAction,
    TypeBase,
    TypeKind,
    UpdateAction,
    ValidationError,
    WaitAction,
    code,
    coerce_custom_object_scalar,
    make_node_from_partial,
    patch_node_from_partial,
    upload_file,
)
from bench.runtime.browser import parse_dom_node
from bench.runtime.core import (
    ATTEMPT_ONCE,
    NotSupportedError,
    RetryableError,
    RunImpossibleError,
    RunIn,
    Runner,
    Runtime,
    make_runner,
    restore_runner,
)
from bench.runtime.core.error import IncapableError, RefusedError
from bench.runtime.model.chat import get_chat_model_runner_cls

if TYPE_CHECKING:
    from .flow import FlowRunner


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

CODE_PASS = code("pass")


class ActionRunner[A: Action = Action](Runner[A], ABC):
    """Action Runner."""

    runner_type: ClassVar[RunType] = RunType.ACTION

    def __init__(
        self,
        *,
        runtime: Runtime,
        node: A,
        run: RunIn,
        options: RunOptions,
        context: HasContext,
        parent: Runner | None = None,
        variables: CustomObject | None = None,
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
            variables=variables,
            inputs=inputs,
            outputs=outputs,
            run=run,
        )
        self.action = cast(A, self.inputs)
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


class StaticActionRunner[A: Action = Action](ActionRunner[A]):
    @final
    @override
    async def run(self) -> None:
        """Run the static and the dynamic parts of the Action."""
        # static implementation
        await self.run_static()

        # dynamic calls
        has_plan = (
            not self.node.type.is_boundary
            and self.outputs is not None
            and self.outputs._type.kind == TypeKind.PARTIAL_OBJECT
            and (getattr(self.outputs, "plans", None))
        )
        if (
            self.flow is not None
            and not has_plan
            and (outgoing_pipes := [p for p in self.flow.node.pipes if p.source_id == self.node.id])
            # NOTE :Incomplete: we could also generate calls for other missing Action inputs
            #  (Sometimes..? Only for application actions like click? For all static actions?)
            and any(p.type == PipeType.SELECT for p in outgoing_pipes)
        ):
            model_developer = ModelDeveloper.OPENAI
            model_type = ModelType.OPENAI_GPT4_0
            model_runner_cls = get_chat_model_runner_cls(
                model_developer=model_developer, model_type=model_type
            )
            model_runner = model_runner_cls(
                runtime=self.runtime,
                node=self.node,
                model_type=model_type,
                options=self.options,
                context=self.context,
                variables=self.variables,
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
                assert model_runner.outputs is not None, f"no outputs for {model_runner!r}"
                if self.outputs is not None:
                    self.outputs.set_default(model_runner.outputs)
                else:
                    self.outputs = model_runner.outputs

    @abstractmethod
    async def run_static(self) -> None:
        """Run the static part of the Action."""
        ...


class DynamicActionRunner[A: Action = Action](ActionRunner[A]):
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
            model_type = self.options.model_type or ModelType.OPENAI_O3_MINI
        elif model_developer == ModelDeveloper.ANTHROPIC:
            model_runner_cls = AnthropicChatModelRunner
            model_type = self.options.model_type or ModelType.ANTHROPIC_CLAUDE_3_5_SONNET
        elif model_developer == ModelDeveloper.GOOGLE:
            model_runner_cls = GeminiChatModelRunner
            model_type = self.options.model_type or ModelType.GOOGLE_GEMINI_2_0_FLASH
        else:
            raise NotSupportedError(f"unsupported model developer {model_developer!r}")

        # run model
        model_runner = model_runner_cls(
            runtime=self.runtime,
            node=self.node,
            model_type=model_type,
            options=self.options,
            context=self.context,
            variables=self.variables,
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
        self.outputs = self.inputs


class CompleteActionRunner(ActionRunner[CompleteAction]):
    @override
    async def run(self) -> None:
        self.outputs = self.inputs
        if self.flow is not None:
            self.flow._complete(outputs=self.outputs)


class FailActionRunner(ActionRunner[FailAction]):
    @override
    async def run(self) -> None:
        title = self.action.error_title or "Flow failed"
        text = self.action.error_text or Text.plain(f"Flow failed at {self.node!r}")
        raise RetryableError(title=title, text=text)


#
# Tool
#


class CodeActionRunner(ActionRunner[CodeAction]):
    # NOTE: Code runner is static and must generate its own Plans
    #  (so we don't auto-generate them to simplify things, thus we don't inherit StaticActionRunner)

    @override
    async def run(self) -> None:
        from bench.runtime.code import CodeFunctionRunner

        code = self.action.code or CODE_PASS
        code_runner = CodeFunctionRunner(
            runtime=self.runtime,
            node=self.node,
            options=ATTEMPT_ONCE,
            context=self.context,
            run=RunSpanType.DELEGATE,
            code=code,
            aliasing=Aliasing(),
            variables=self.variables,
            inputs=self.inputs,
            outputs=self.outputs or self.output_type,
        )
        self.tracked.code = code
        if self.tracked_run is not None and self.tracked_run is not self.tracked:
            self.tracked_run.code = code
        await self.runtime.run_runner(code_runner)
        self.outputs = code_runner.outputs


class ToolActionRunner(StaticActionRunner[ToolAction]):
    def _get_resumable_subrunner(
        self,
        node: RunnableNode,
        variables: CustomObject | None,
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
                return restore_runner(self.runtime, run)
        else:
            # make new Runner
            runner = make_runner(
                runtime=self.runtime,
                node=node,
                run="track",
                context=self.context,
                variables=variables,
                inputs=inputs,
                outputs=output_type,
            )
            return runner

    @override
    async def run_static(self) -> None:
        tool = self.action.tool
        tool_selection = self.node.tool_selection
        tool_type = self.action.type
        if tool_selection is not None and not tool_selection.supports(tool_type, tool):
            raise RefusedError(f"tool {tool!r} not supported")
        if tool is not None:
            # delegate to tool node
            tool_runner: Runner[Any] = self._get_resumable_subrunner(
                node=tool,
                # inputs/variables are both PartialAction with node=tool
                variables=self.inputs,
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
                variables=self.inputs,
                inputs=self.inputs,
                outputs=self.output_type or self.outputs,
                flow=self.flow,
            )
            await self.runtime.run_runner(tool_runner)
            self.outputs = tool_runner.outputs
        else:
            raise RunImpossibleError("no tool given")


#
# Read
#


class GetActionRunner(StaticActionRunner[GetAction]):
    @override
    async def run_static(self) -> None:
        raise NotImplementedError


class SearchActionRunner(StaticActionRunner[SearchAction]):
    @override
    async def run_static(self) -> None:
        raise NotImplementedError


#
# Write
#


class CreateActionRunner(StaticActionRunner[CreateAction]):
    @override
    async def run_static(self) -> None:
        node_partial = self.action.node_partial
        assert isinstance(node_partial, CustomObject), f"bad node_partial: {node_partial!r}"

        # create node from partial
        node = make_node_from_partial(node_partial)
        if node.parent is None:
            parent_types = node.__parent_property__.reference_nodes or ()
            if parent_types == "any" or NodeType.PACKAGE in parent_types:
                bench = self.session.bench
                assert bench is not None, f"no bench for {self!r}"
                node.parent = bench.main_package
            elif NodeType.BENCH in parent_types:
                bench = self.session.bench
                assert bench is not None, f"no bench for {self!r}"
                node.parent = bench
            elif (
                isinstance(node, HasNodeBase)
                and node.base is not None
                and (parent_types == "any" or node.base.metatype in parent_types)
            ):
                node.parent = node.base
        assert node.is_attached, f"node {node!r} must be attached"
        self.session._create(node)
        logger.debug("action.create", action=self.node, node=node)

        assert self.output_type is not None, f"no output type for {self!r}"
        self.outputs = coerce_custom_object_scalar({"node": node}, self.output_type, as_packed=True)


class DuplicateActionRunner(StaticActionRunner[DuplicateAction]):
    @override
    async def run_static(self) -> None:
        node = self.action.node
        node_partial = self.action.node_partial
        assert node is not None, "no node to clone"
        assert isinstance(node_partial, CustomObject), f"bad node_partial: {node_partial!r}"

        # clone node with partial override
        cloned_node = node.clone(recursive=not self.action.is_shallow, detach=True)
        patch_node_from_partial(cloned_node, node_partial)
        cloned_node_parent = cloned_node.parent or node.parent
        assert cloned_node_parent is not None, f"cloned node {cloned_node!r} must be attached"
        cloned_node_parent.append(cloned_node)
        logger.debug("action.clone", action=self.node, node=cloned_node, partial=node_partial)

        assert self.output_type is not None, f"no output type for {self!r}"
        self.outputs = coerce_custom_object_scalar(
            {"duplicated_node": cloned_node}, self.output_type, as_packed=True
        )


class UpdateActionRunner(StaticActionRunner[UpdateAction]):
    @override
    async def run_static(self) -> None:
        node = self.action.node
        assert node is not None, "no node to update"
        node_partial = self.action.node_partial
        assert isinstance(node_partial, CustomObject), f"bad node_partial: {node_partial!r}"

        # update with partial patch
        patch_node_from_partial(node, node_partial)
        logger.debug("action.update", action=self.node, node=node, partial=node_partial)


class DeleteActionRunner(StaticActionRunner[DeleteAction]):
    @override
    async def run_static(self) -> None:
        node = self.action.node
        assert node is not None, "no node to delete"

        # delete
        node.delete()
        logger.debug("action.delete", action=self.node, node=node)


#
# Async
#


class YieldActionRunner(StaticActionRunner):
    @override
    async def run_static(self) -> None:
        interruption = self._trap_interruption(InterruptionType.YIELD)
        self.outputs = interruption.outputs


class WaitActionRunner(StaticActionRunner[WaitAction]):
    @override
    async def run_static(self) -> None:
        # NOTE: obviously WaitStep should be Interruption/Trigger-driven
        if self.action.delay is not None:
            await asyncio.sleep(self.action.delay.total_seconds())


#
# Application
# NOTE :Incomplete: for now Application Actions only work with Browser (via Playwright)
#


class ApplicationActionRunner[A: Action = Action](StaticActionRunner[A]):
    def _get_application(self) -> Browser:
        if (application := cast(HasApplicationContext, self.action).application) is not None:
            return application
        else:
            return self._get_ready_resource_or_error(Browser)

    async def _focus_element(
        self,
        inputs: HasApplicationContext,
        pw_page: PlaywrightPage,
        button: Literal["left", "right", "middle"] = "left",
    ):
        """Focuses the element."""
        if (element_id := inputs.element_id) is not None:
            selector = f"[data-bench-highlight-id='{element_id}']"
            await pw_page.focus(selector)
        elif (element_position := inputs.element_position) is not None:
            await pw_page.mouse.click(element_position.x, element_position.y, button=button)
        else:
            raise IncapableError("no element to focus")


class LookActionRunner(ApplicationActionRunner[LookAction]):
    @override
    async def run_static(self) -> None:
        # TODO :Performance: obviously LookAction could be a lot more efficient
        #  (defer uploads, ensure extension script is preloaded, ...)
        browser = self._get_application()
        pw_browser = await self.runtime.playwright.get_client(browser)
        pw_page = pw_browser.pages[0]
        dom_nodes_js = await pw_page.evaluate(
            self.runtime.playwright.get_extension_script_js() + "\n cleanup(); highlight()"
        )
        dom_nodes = [parse_dom_node(dom_node_js) for dom_node_js in dom_nodes_js]
        screenshot_bytes = await pw_page.screenshot(full_page=False, animations="disabled")
        now = self.session._oracle.utc()
        screenshot = await upload_file(
            screenshot_bytes,
            type=FileType.IMAGE,
            format=FileFormat.PNG,
            name=f"{browser.name} Screenshot {now.strftime('%Y-%m-%d %H:%M:%S.%f')}",
        )
        assert self.output_type is not None, f"no output type for {self!r}"
        self.outputs = coerce_custom_object_scalar(
            {"dom_nodes": dom_nodes, "screenshot": screenshot}, self.output_type
        )


class ClickActionRunner(ApplicationActionRunner[ClickAction]):
    @override
    async def run_static(self) -> None:
        browser = self._get_application()
        pw_browser = await self.runtime.playwright.get_client(browser)
        pw_page = pw_browser.pages[0]
        button = self.action.button or "left"
        if button not in ("left", "right", "middle"):
            raise ValidationError(None, f"invalid button: {button!r}")
        if (element_id := self.action.element_id) is not None:
            selector = f"[data-bench-highlight-id='{element_id}']"
            await pw_page.click(selector)
        elif (element_position := self.action.element_position) is not None:
            await pw_page.mouse.click(element_position.x, element_position.y, button=button)
        else:
            raise IncapableError("no element to focus")
        await self.runtime.playwright.wait_for_idle(pw_page)


class PressActionRunner(ApplicationActionRunner[PressAction]):
    @override
    async def run_static(self) -> None:
        combination = self.action.combination
        if not isinstance(combination, str):
            raise ValidationError(None, f"bad keys to press: {combination!r}")
        browser = self._get_application()
        pw_browser = await self.runtime.playwright.get_client(browser)
        pw_page = pw_browser.pages[0]
        await self._focus_element(self.action, pw_page)
        delay_seconds = self.action.delay.total_seconds() if self.action.delay else 0
        await pw_page.keyboard.press(combination, delay=delay_seconds)
        await self.runtime.playwright.wait_for_idle(pw_page)


class TypeActionRunner(ApplicationActionRunner[TypeAction]):
    @override
    async def run_static(self) -> None:
        string = self.action.string
        if not isinstance(string, str):
            raise ValidationError(None, f"bad string to type: {string!r}")
        browser = self._get_application()
        pw_browser = await self.runtime.playwright.get_client(browser)
        pw_page = pw_browser.pages[0]
        await self._focus_element(self.action, pw_page)
        delay_seconds = self.action.delay.total_seconds() if self.action.delay else 0
        await pw_page.keyboard.type(string, delay=delay_seconds)


class ScrollActionRunner(ApplicationActionRunner[ScrollAction]):
    @override
    async def run_static(self) -> None:
        amount = self.action.amount
        if amount is None:
            raise ValidationError(None, "no amount to scroll")
        browser = self._get_application()
        pw_browser = await self.runtime.playwright.get_client(browser)
        pw_page = pw_browser.pages[0]
        await self._focus_element(self.action, pw_page)
        await pw_page.mouse.wheel(delta_x=amount.x, delta_y=amount.y)


class SelectActionRunner(ApplicationActionRunner[SelectAction]):
    @override
    async def run_static(self) -> None:
        raise NotImplementedError


class GoBackwardActionRunner(ApplicationActionRunner[GoBackwardAction]):
    @override
    async def run_static(self) -> None:
        browser = self._get_application()
        pw_browser = await self.runtime.playwright.get_client(browser)
        pw_page = pw_browser.pages[0]
        await pw_page.go_back()
        await self.runtime.playwright.wait_for_idle(pw_page)


class GoForwardActionRunner(ApplicationActionRunner[GoForwardAction]):
    @override
    async def run_static(self) -> None:
        browser = self._get_application()
        pw_browser = await self.runtime.playwright.get_client(browser)
        pw_page = pw_browser.pages[0]
        await pw_page.go_forward()
        await self.runtime.playwright.wait_for_idle(pw_page)


#
# Web
#


class GoToUrlActionRunner(ApplicationActionRunner[GoToUrlAction]):
    @override
    async def run_static(self) -> None:
        url = self.action.url
        if url is None:
            raise ValidationError(None, "no url to go to")
        browser = self._get_application()
        pw_browser = await self.runtime.playwright.get_client(browser)
        pw_page = pw_browser.pages[0]
        await pw_page.goto(url, wait_until="domcontentloaded")
        await self.runtime.playwright.wait_for_idle(pw_page)


class GoToTabActionRunner(ApplicationActionRunner[GoToTabAction]):
    @override
    async def run_static(self) -> None:
        tab_index = self.action.tab_index
        if tab_index is None:
            raise ValidationError(None, "no tab index to go to")
        browser = self._get_application()
        pw_browser = await self.runtime.playwright.get_client(browser)
        await pw_browser.pages[tab_index].bring_to_front()


ACTION_RUNNER_BY_ACTION_TYPE: dict[ActionType, type[ActionRunner[Any]]] = {
    # flow
    ActionType.START: StartActionRunner,
    ActionType.COMPLETE: CompleteActionRunner,
    ActionType.FAIL: FailActionRunner,
    # tool
    ActionType.CODE: CodeActionRunner,
    ActionType.TOOL: ToolActionRunner,
    # dynamic
    ActionType.ACT: DynamicActionRunner,
    ActionType.THINK: DynamicActionRunner,
    ActionType.ROUTE: DynamicActionRunner,
    ActionType.GENERATE: DynamicActionRunner,
    ActionType.TRANSFORM: DynamicActionRunner,
    ActionType.EXTRACT: DynamicActionRunner,
    ActionType.CLASSIFY: DynamicActionRunner,
    ActionType.SUMMARIZE: DynamicActionRunner,
    ActionType.COMPARE: DynamicActionRunner,
    ActionType.TRANSLATE: DynamicActionRunner,
    ActionType.CHANGE: DynamicActionRunner,
    # read
    ActionType.GET: GetActionRunner,
    ActionType.SEARCH: SearchActionRunner,
    # write
    ActionType.CREATE: CreateActionRunner,
    ActionType.DUPLICATE: DuplicateActionRunner,
    ActionType.UPDATE: UpdateActionRunner,
    ActionType.DELETE: DeleteActionRunner,
    # async
    ActionType.YIELD: YieldActionRunner,
    ActionType.WAIT: WaitActionRunner,
    # application
    ActionType.LOOK: LookActionRunner,
    ActionType.CLICK: ClickActionRunner,
    ActionType.PRESS: PressActionRunner,
    ActionType.TYPE: TypeActionRunner,
    ActionType.SCROLL: ScrollActionRunner,
    ActionType.SELECT: SelectActionRunner,
    ActionType.GO_BACKWARD: GoBackwardActionRunner,
    ActionType.GO_FORWARD: GoForwardActionRunner,
    # web
    ActionType.GO_TO_URL: GoToUrlActionRunner,
    ActionType.GO_TO_TAB: GoToTabActionRunner,
}
