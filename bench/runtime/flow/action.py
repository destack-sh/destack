import asyncio
from abc import ABC
from typing import TYPE_CHECKING, Any, ClassVar, cast, override

import structlog
from opentelemetry import trace
from playwright.async_api import Page as PlaywrightPage

from bench.language import (
    Action,
    ActionType,
    BreakpointScope,
    BreakpointSite,
    Browser,
    ClickAction,
    CodeAction,
    CreateAction,
    CustomObject,
    DelegateAction,
    DeleteAction,
    DuplicateAction,
    FailAction,
    FileFormat,
    FileType,
    GoBackwardAction,
    GoForwardAction,
    GoToTabAction,
    GoToUrlAction,
    HasApplicationContext,
    HasNodeBase,
    InterruptionType,
    NodeType,
    ObjectKind,
    ObserveAction,
    PressAction,
    ProxyReadObject,
    Run,
    RunnableNode,
    RunOptions,
    RunType,
    ScrollAction,
    SelectAction,
    Text,
    TypeAction,
    TypeBase,
    UpdateAction,
    ValidationError,
    WaitAction,
    code,
    coerce_custom_object_scalar,
    make_node_from_partial,
    patch_node_from_partial,
    upload_file,
)
from bench.runtime.browser.playwright import parse_dom_node
from bench.runtime.code.code import CodeFunctionRunner
from bench.runtime.core import ATTEMPT_ONCE, RetryableError, RunImpossibleError
from bench.runtime.core.runner import Context, Runner, make_runner, restore_runner
from bench.runtime.core.runtime import Runtime

if TYPE_CHECKING:
    from .flow import FlowRunnerBase


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

CODE_PASS = code("pass")


class ActionRunnerBase[A: Action = Action](Runner[A], ABC):
    """Action Runner."""

    kind: ClassVar[RunType] = RunType.ACTION

    def __init__(
        self,
        *,
        runtime: Runtime,
        node: A,
        track: bool,
        options: RunOptions,
        context: Context,
        parent: Runner | None = None,
        variables: CustomObject | None = None,
        inputs: CustomObject | None = None,
        output_type: TypeBase | None = None,
        run: Run | None = None,
        flow: "FlowRunnerBase | None" = None,
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
        assert self.inputs is not None, f"no inputs for {self!r}"
        self.action_inputs = cast(A, ProxyReadObject(obj=self.inputs, default=self.node))
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
        node: RunnableNode,
        variables: CustomObject | None,
        inputs: CustomObject | None,
        output_type: TypeBase | None = None,
    ):
        """
        Gets the Runner for the given Node within this Runner.
        If there is an interrupted Run inside the current Run for the node, we'll just return that.
        Assumes there will only be on Runner for a given RunAttempt and Node.
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
                track=True,
                context=self.context,
                variables=variables,
                inputs=inputs,
                output_type=output_type,
            )
            return runner

    # nocheckin: generate missing/unset outputs & Calls (in Flow? or always?)


#
# Boundary
#


class StartActionRunner(ActionRunnerBase):
    @override
    async def run(self) -> None:
        self.outputs = self.inputs


class CompleteActionRunner(ActionRunnerBase):
    @override
    async def run(self) -> None:
        self.outputs = self.inputs
        if self.flow is not None:
            self.flow._complete(outputs=self.outputs)


class FailActionRunner(ActionRunnerBase[FailAction]):
    @override
    async def run(self) -> None:
        title = self.action_inputs.error_title or "Flow failed"
        text = self.action_inputs.error_text or Text.plain(f"Flow failed at {self.node!r}")
        raise RetryableError(title=title, text=text)


class TriggerActionRunner(ActionRunnerBase):
    @override
    async def run(self) -> None:
        raise NotImplementedError


#
# Read
#


class GetActionRunner(ActionRunnerBase):
    @override
    async def run(self) -> None:
        raise NotImplementedError


class SearchActionRunner(ActionRunnerBase):
    @override
    async def run(self) -> None:
        raise NotImplementedError


#
# Write
#


class CreateActionRunner(ActionRunnerBase[CreateAction]):
    @override
    async def run(self) -> None:
        node_partial = self.action_inputs.node_partial
        assert isinstance(node_partial, CustomObject), f"unexpected {node_partial!r}"

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
        self.outputs = CustomObject.new(ObjectKind.OUTPUT, {"node": node}, self.output_type)


class DuplicateActionRunner(ActionRunnerBase[DuplicateAction]):
    @override
    async def run(self) -> None:
        node = self.action_inputs.node
        node_partial = self.action_inputs.node_partial
        assert node is not None, "no node to clone"
        assert isinstance(node_partial, CustomObject), f"unexpected {node_partial!r}"

        # clone node with partial override
        cloned_node = node.clone(recursive=not self.action_inputs.is_shallow, detach=True)
        patch_node_from_partial(cloned_node, node_partial)
        cloned_node_parent = cloned_node.parent or node.parent
        assert cloned_node_parent is not None, f"cloned node {cloned_node!r} must be attached"
        cloned_node_parent.append(cloned_node)
        logger.debug("action.clone", action=self.node, node=cloned_node, partial=node_partial)

        assert self.output_type is not None, f"no output type for {self!r}"
        self.outputs = CustomObject.new(ObjectKind.OUTPUT, {"node": cloned_node}, self.output_type)


class UpdateActionRunner(ActionRunnerBase[UpdateAction]):
    @override
    async def run(self) -> None:
        node = self.action_inputs.node
        assert node is not None, "no node to update"
        node_partial = self.action_inputs.node_partial
        assert isinstance(node_partial, CustomObject), f"unexpected {node_partial!r}"

        # update with partial patch
        patch_node_from_partial(node, node_partial)
        logger.debug("action.update", action=self.node, node=node, partial=node_partial)

        assert self.output_type is not None, f"no output type for {self!r}"
        self.outputs = CustomObject.new(ObjectKind.OUTPUT, {"node": node}, self.output_type)


class DeleteActionRunner(ActionRunnerBase[DeleteAction]):
    @override
    async def run(self) -> None:
        node = self.action_inputs.node
        assert node is not None, "no node to delete"

        # delete
        node.delete()
        logger.debug("action.delete", action=self.node, node=node)

        assert self.output_type is not None, f"no output type for {self!r}"
        self.outputs = CustomObject.new(ObjectKind.OUTPUT, {"node": node}, self.output_type)


#
# Session
#


class YieldActionRunner(ActionRunnerBase):
    @override
    async def run(self) -> None:
        interruption = self._trap_interruption(InterruptionType.YIELD)
        self.outputs = interruption.outputs


#
# Static
#


class CodeActionRunner(ActionRunnerBase[CodeAction]):
    @override
    async def run(self) -> None:
        code_runner = CodeFunctionRunner(
            runtime=self.runtime,
            node=self.node,
            code=self.node.code or CODE_PASS,
            variables=self.variables,
            inputs=self.inputs,
            output_type=self.output_type,
            track=False,
            options=ATTEMPT_ONCE,
            context=self.context,
        )
        await self.runtime.run_runner(code_runner)
        self.outputs = code_runner.outputs


class DelegateActionRunner(ActionRunnerBase[DelegateAction]):
    @override
    async def run(self) -> None:
        delegate = self.node.delegate
        if not delegate:
            raise RunImpossibleError("no delegate")
        delegate_runner = self._get_resumable_subrunner(
            node=delegate,
            variables=self.variables,
            inputs=self.inputs,
            output_type=self.output_type,
        )
        await self.runtime.run_runner(delegate_runner)


class WaitActionRunner(ActionRunnerBase[WaitAction]):
    @override
    async def run(self) -> None:
        # NOTE: obviously WaitStep should be Interrupt/Trigger-driven
        if self.action_inputs.delay is not None:
            await asyncio.sleep(self.action_inputs.delay.total_seconds())


#
# Dynamic
#


class DynamicActionRunner(ActionRunnerBase):
    @override
    async def run(self) -> None:
        raise NotImplementedError(f"nocheckin: DynamicActionRunner {self!r}")


#
# Application
# NOTE :Incomplete: for now Application Actions only work on the web (with Playwright)
#


class ApplicationActionRunnerBase[A: Action = Action](ActionRunnerBase[A]):
    async def _get_element_selector(self, inputs: HasApplicationContext) -> str | None:
        """Gets the element selector from the inputs."""
        # TODO :Incomplete: automatically generate selector for Action if non given
        #  (with run_span(APPLICATION_FIND_ELEMENT), ...)
        if inputs.element_id is not None:
            return f"[data-bench-highlight-id='{inputs.element_id}']"
        elif inputs.element_path is not None:
            return inputs.element_path
        else:
            return None

    async def _focus_element(self, inputs: HasApplicationContext, pw_page: PlaywrightPage):
        """Focuses the element."""
        selector = await self._get_element_selector(inputs)
        if selector is None:
            raise ValidationError(None, "no element selector to focus")
        await pw_page.focus(selector)


class ObserveActionRunner(ActionRunnerBase[ObserveAction]):
    @override
    async def run(self) -> None:
        # TODO :Performance: obviously ObserveAction could be a lot more efficient
        #  (defer uploads, ensure extension script is preloaded, ...)
        browser = self._get_ready_resource_or_error(Browser)
        pw_browser = await self.runtime.playwright.get_client(browser)
        pw_page = pw_browser.pages[0]
        dom_tree_js = await pw_page.evaluate(
            self.runtime.playwright.get_extension_script_js() + "\n extractDocumentDomTree(true)"
        )
        dom_tree = parse_dom_node(dom_tree_js)
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
            ObjectKind.OUTPUT, {"dom": dom_tree, "screenshot": screenshot}, self.output_type
        )


class ClickActionRunner(ApplicationActionRunnerBase[ClickAction]):
    @override
    async def run(self) -> None:
        selector = await self._get_element_selector(self.action_inputs)
        if selector is None:
            raise ValidationError(None, "no element selector to click")
        browser = self._get_ready_resource_or_error(Browser)
        pw_browser = await self.runtime.playwright.get_client(browser)
        pw_page = pw_browser.pages[0]
        await pw_page.click(selector)


class PressActionRunner(ApplicationActionRunnerBase[PressAction]):
    @override
    async def run(self) -> None:
        keys = self.action_inputs.keys
        if keys is None:
            raise ValidationError(None, "no keys to press")
        browser = self._get_ready_resource_or_error(Browser)
        pw_browser = await self.runtime.playwright.get_client(browser)
        pw_page = pw_browser.pages[0]
        await self._focus_element(self.action_inputs, pw_page)
        await pw_page.keyboard.press(keys, delay=self.action_inputs.delay)


class TypeActionRunner(ApplicationActionRunnerBase[TypeAction]):
    @override
    async def run(self) -> None:
        string = self.action_inputs.string
        if string is None:
            raise ValidationError(None, "no string to type")
        browser = self._get_ready_resource_or_error(Browser)
        pw_browser = await self.runtime.playwright.get_client(browser)
        pw_page = pw_browser.pages[0]
        await self._focus_element(self.action_inputs, pw_page)
        await pw_page.keyboard.type(string, delay=self.action_inputs.delay)


class ScrollActionRunner(ApplicationActionRunnerBase[ScrollAction]):
    @override
    async def run(self) -> None:
        amount = self.action_inputs.amount
        if amount is None:
            raise ValidationError(None, "no amount to scroll")
        browser = self._get_ready_resource_or_error(Browser)
        pw_browser = await self.runtime.playwright.get_client(browser)
        pw_page = pw_browser.pages[0]
        await self._focus_element(self.action_inputs, pw_page)
        await pw_page.mouse.wheel(delta_x=amount.x, delta_y=amount.y)


class SelectActionRunner(ApplicationActionRunnerBase[SelectAction]):
    @override
    async def run(self) -> None:
        raise NotImplementedError


class GoBackwardActionRunner(ApplicationActionRunnerBase[GoBackwardAction]):
    @override
    async def run(self) -> None:
        browser = self._get_ready_resource_or_error(Browser)
        pw_browser = await self.runtime.playwright.get_client(browser)
        pw_page = pw_browser.pages[0]
        await pw_page.go_back()


class GoForwardActionRunner(ApplicationActionRunnerBase[GoForwardAction]):
    @override
    async def run(self) -> None:
        browser = self._get_ready_resource_or_error(Browser)
        pw_browser = await self.runtime.playwright.get_client(browser)
        pw_page = pw_browser.pages[0]
        await pw_page.go_forward()


#
# Browser
#


class GoToUrlActionRunner(ApplicationActionRunnerBase[GoToUrlAction]):
    @override
    async def run(self) -> None:
        url = self.action_inputs.url
        if url is None:
            raise ValidationError(None, "no url to go to")
        browser = self._get_ready_resource_or_error(Browser)
        pw_browser = await self.runtime.playwright.get_client(browser)
        pw_page = pw_browser.pages[0]
        await pw_page.goto(url, wait_until="domcontentloaded")
        await self.runtime.playwright.wait_for_idle(pw_page)


class GoToTabActionRunner(ApplicationActionRunnerBase[GoToTabAction]):
    @override
    async def run(self) -> None:
        tab_index = self.action_inputs.tab_index
        if tab_index is None:
            raise ValidationError(None, "no tab index to go to")
        browser = self._get_ready_resource_or_error(Browser)
        pw_browser = await self.runtime.playwright.get_client(browser)
        await pw_browser.pages[tab_index].bring_to_front()


ACTION_RUNNER_BY_ACTION_TYPE: dict[ActionType, type[ActionRunnerBase[Any]]] = {
    # boundary
    ActionType.START: StartActionRunner,
    ActionType.COMPLETE: CompleteActionRunner,
    ActionType.FAIL: FailActionRunner,
    # read
    ActionType.GET: GetActionRunner,
    ActionType.SEARCH: SearchActionRunner,
    # write
    ActionType.CREATE: CreateActionRunner,
    ActionType.DUPLICATE: DuplicateActionRunner,
    ActionType.UPDATE: UpdateActionRunner,
    ActionType.DELETE: DeleteActionRunner,
    # session
    ActionType.YIELD: YieldActionRunner,
    # static
    ActionType.CODE: CodeActionRunner,
    ActionType.DELEGATE: DelegateActionRunner,
    ActionType.WAIT: WaitActionRunner,
    # dynamic
    ActionType.GENERATE: DynamicActionRunner,
    ActionType.TRANSFORM: DynamicActionRunner,
    ActionType.ROUTE: DynamicActionRunner,
    ActionType.CHANGE: DynamicActionRunner,
    # application
    ActionType.OBSERVE: ObserveActionRunner,
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
