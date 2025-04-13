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
    ModelDeveloper,
    ModelType,
    Runnable,
    Span,
    SpanType,
)
from bench.runtime.core import RunIn, Runner, Runtime
from bench.runtime.model import get_chat_model_runner_cls
from bench.utils.tenacity import RetryOptions

from .instruct import build_prompt

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


AgentAction = Union[AgentComplete, AgentWait, AgentContinue]


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
        self._next_action: AgentAction = AgentContinue()

    def complete(self):
        """Complete the current Run."""
        self._next_action = AgentComplete()

    def wake(self):
        """Wake the Agent."""
        # nocheckin: handle resuming AgentRunner somehow?
        # self._next_action = AgentContinue()
        pass

    def wait(self, duration: float):
        """Wait for the Agent."""
        self._next_action = AgentWait(seconds=duration)

    @tracer.start_as_current_span("agent.think")
    async def _think(self):
        """Prepare the next actions in this Flow (if any)."""
        attempts: list[Span] = []
        retry = THINK_RETRY_OPTIONS.new(oracle=self.runtime.oracle)
        thread = self.thread.thread
        agent = self.node
        while retry.should_retry:
            retry.on_attempt()

            # update thread cursor
            if (cursor := agent.main_cursor) is None:
                cursor = Cursor(type=CursorType.THREAD, target=thread, owned_by=agent)
                thread.cursors.append(cursor)
                agent.main_cursor = cursor
            assert cursor.type == CursorType.THREAD
            cursor.status = CursorStatus.THINKING
            if (new_seen_at := self.thread.last_message_at) is not None and (
                cursor.seen_at is None or new_seen_at > cursor.seen_at
            ):
                cursor.seen_at = new_seen_at
            cursor.active_at = self.runtime.oracle.utc()

            # make prompt
            prompt = build_prompt(
                agent=self.node, runner=cast(AgentRunner[Agent], self), previous_attempts=attempts
            )

            # run model
            model_developer = ModelDeveloper.OPENAI
            model_type = ModelType.OPENAI_GPT4_0
            model_runner_cls = get_chat_model_runner_cls(
                model_developer=model_developer, model_type=model_type
            )
            model_runner = model_runner_cls(
                runtime=self.runtime,
                node=self.node,
                model_type=model_type,
                prompt=prompt,
                parent=cast(Runner[Runnable], self),
                agent=self.agent,
                run=SpanType.AGENT_THINK,
            )
            attempt = model_runner.tracked_span
            assert attempt is not None, f"{model_runner!r} has no Span"
            attempts.append(attempt)
            try:
                await self.runtime.run_runner(model_runner)
                logger.debug(
                    "agent.think", flow=self.node, runner=self, attempt=attempt, span="current"
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

    async def run(self) -> None:
        # run main loop
        self._next_action = AgentContinue()
        while True:
            self._next_action = AgentComplete()
            await self._think()
            if isinstance(self._next_action, AgentComplete):
                break
            elif isinstance(self._next_action, AgentWait):
                await self.runtime.oracle.sleep(self._next_action.seconds)
            elif isinstance(self._next_action, AgentContinue):
                continue
            else:
                assert_never(self._next_action)
