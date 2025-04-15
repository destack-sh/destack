import asyncio
from dataclasses import dataclass
from typing import Union, assert_never, cast

import structlog
from opentelemetry import trace

from bench.language import (
    Agent,
    Cursor,
    CursorStatus,
    CursorType,
    CustomObject,
    IsType,
    ModelProvider,
    Run,
    Runnable,
    Span,
    SpanType,
)
from bench.runtime.core import NotSupportedError, RunIn, Runner, Runtime, restore_runner
from bench.runtime.model import ChatModelRunner
from bench.utils.tenacity import RetryOptions

from .instruct import make_agent_prompt

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

THINK_RETRY_OPTIONS = RetryOptions(
    max_attempts=10, retry_interval=0.2, backoff=1.5, max_retry_interval=10
)


@dataclass
class AgentComplete:
    pass


@dataclass
class AgentWait:
    seconds: float


@dataclass
class AgentContinue:
    pass


@dataclass
class AgentTool:
    run: Run


AgentAction = Union[AgentComplete, AgentWait, AgentContinue, AgentTool]


class AgentRunner[N: Agent = Agent](Runner[N]):
    def __init__(
        self,
        *,
        runtime: Runtime,
        node: N,
        run: RunIn,
        parent: Runner[Runnable] | None = None,
        inputs: CustomObject | None = None,
        outputs: IsType | CustomObject | None = None,
        agent: Agent | None = None,
    ) -> None:
        super().__init__(
            runtime=runtime,
            node=node,
            run=run,
            parent=parent,
            inputs=inputs,
            outputs=outputs,
            agent=agent,
        )
        self._received_wake: bool = False
        self._next_action: AgentAction = AgentContinue()

    def complete(self):
        """Complete the current Run."""
        self._next_action = AgentComplete()

    def wake(self):
        """Wake the Agent."""
        self._received_wake = True

    def wait(self, duration: float):
        """Wait for the Agent."""
        self._next_action = AgentWait(seconds=duration)

    def call(self, run: Run):
        """Call an Action."""
        self._next_action = AgentTool(run=run)

    # nocheckin: indicate current Agent activity/focus/...

    def _get_model_runner_cls(self) -> tuple[type[ChatModelRunner], str]:
        """Get the ChatModelRunner class for the given model type."""
        from bench.runtime.model import (
            AnthropicChatModelRunner,
            GoogleChatModelRunner,
            OpenAIChatModelRunner,
            OpenRouterChatModelRunner,
        )

        model_provider = self.node.model_provider or ModelProvider.OPENAI
        if model_provider == ModelProvider.OPENAI:
            return OpenAIChatModelRunner, "gpt-4.1-2025-04-14"
        elif model_provider == ModelProvider.ANTHROPIC:
            return AnthropicChatModelRunner, "claude-3-7-sonnet-20250219"
        elif model_provider == ModelProvider.GOOGLE:
            return GoogleChatModelRunner, "gemini-2.5-pro-preview-03-25"
        elif model_provider == ModelProvider.OPENROUTER:
            return OpenRouterChatModelRunner, "openrouter/quasar-alpha"
        else:
            raise NotSupportedError(f"unsupported model provider {model_provider!r}")

    @tracer.start_as_current_span("agent.tick")
    async def _tick(self):
        """Generate and execute the next Agent tick."""
        attempts: list[Span] = []
        retry = THINK_RETRY_OPTIONS.new(oracle=self.runtime.oracle)
        thread = self.thread.thread
        agent = self.node
        while retry.should_retry:
            retry.on_attempt()

            # update thread cursor
            if (cursor := agent.get_cursor(type=CursorType.THREAD)) is None:
                cursor = Cursor(type=CursorType.THREAD, target=thread, owned_by=agent)
                agent.cursors.append(cursor)
            assert cursor.type == CursorType.THREAD
            cursor.status = CursorStatus.THINKING
            if (new_seen_at := self.thread.last_message_at) is not None and (
                cursor.seen_at is None or new_seen_at > cursor.seen_at
            ):
                cursor.seen_at = new_seen_at
            cursor.active_at = self.runtime.oracle.utc()
            agent.main_cursor = cursor

            # make prompt
            prompt = make_agent_prompt(
                agent=self.node, runner=cast(AgentRunner[Agent], self), previous_attempts=attempts
            )

            # run model
            # TODO :Incomplete: bring your own models/keys (BYOK)
            model_runner_cls, model_id = self._get_model_runner_cls()
            model_runner = model_runner_cls(
                runtime=self.runtime,
                node=self.node,
                prompt=prompt,
                parent=cast(Runner[Runnable], self),
                agent=self.agent,
                run=SpanType.AGENT_TICK,
                model_id=model_id,
            )
            attempt = model_runner.tracked_span
            assert attempt is not None, f"{model_runner!r} has no Span"
            attempts.append(attempt)
            try:
                await self.runtime.run_runner(model_runner)
                logger.debug(
                    "agent.think",
                    flow=self.node,
                    runner=self,
                    model=model_runner,
                    model_id=model_runner.model_id,
                    attempt=attempt,
                    span="current",
                )
                retry.on_success()
                return  # success
            except asyncio.CancelledError:
                logger.debug(
                    "agent.think.aborted",
                    agent=self.node,
                    runner=self,
                    attempt=attempt,
                    span="current",
                )
                raise
            except BaseException as e:
                logger.debug(
                    "agent.think.error",
                    agent=self.node,
                    runner=self,
                    attempt=attempt,
                    span="current",
                    exc_info=e,
                )
                if not retry.on_error(e):
                    raise
                else:
                    interval = retry.get_wait_interval()
                    logger.trace(
                        "agent.think.retry",
                        agent=self.node,
                        runner=self,
                        attempt=attempt,
                        interval=interval,
                    )
                    await self.runtime.oracle.sleep(interval)

    @tracer.start_as_current_span("agent.call")
    async def _call(self, run: Run):
        """Call an Action as a tool."""
        try:
            runner = restore_runner(self.runtime, run)
            await self.runtime.run_runner(runner)
        except asyncio.CancelledError:
            logger.debug(
                "agent.tool.aborted",
                agent=self.node,
                runner=self,
                run=run,
            )
        except BaseException as e:
            logger.debug(
                "agent.tool.error",
                agent=self.node,
                runner=self,
                run=run,
                exc_info=e,
            )

    async def run(self) -> None:
        # run main loop
        while not isinstance(self._next_action, AgentComplete):
            # think
            self._next_action = AgentComplete()
            self._received_wake = False
            await self._tick()
            received_wake = self._received_wake
            self._received_wake = False

            # do
            if isinstance(self._next_action, AgentComplete):
                if received_wake:
                    self._next_action = AgentContinue()
            elif isinstance(self._next_action, AgentWait):
                await self.runtime.oracle.sleep(self._next_action.seconds)
            elif isinstance(self._next_action, AgentContinue):
                pass  # continue
            elif isinstance(self._next_action, AgentTool):
                await self._call(self._next_action.run)
            else:
                assert_never(self._next_action)
