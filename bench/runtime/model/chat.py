from abc import ABC
from typing import TYPE_CHECKING

import structlog
from opentelemetry import trace

from bench.language import Agent, Code, Runnable
from bench.runtime.core import RunIn, Runner, Runtime

from .model import ModelRunner
from .prompt import Prompt

if TYPE_CHECKING:
    pass


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class ChatModelRunner[R: Runnable](ModelRunner[R], ABC):
    """Run a chat-based Model."""

    def __init__(
        self,
        *,
        runtime: Runtime,
        node: R,
        run: RunIn,
        prompt: Prompt,
        model_id: str | None = None,
        parent: Runner | None = None,
        agent: Agent | None = None,
    ) -> None:
        super().__init__(
            runtime=runtime,
            node=node,
            model_id=model_id,
            parent=parent,
            run=run,
            agent=agent,
        )
        self.prompt = prompt
        self.code: Code | None = None
