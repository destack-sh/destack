from abc import ABC
from typing import TYPE_CHECKING

import structlog
from opentelemetry import trace

from bench.language import Agent, Code, ModelDeveloper, ModelType, Runnable
from bench.runtime.core import NotSupportedError, RunIn, Runner, Runtime

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
        model_type: ModelType,
        run: RunIn,
        prompt: Prompt,
        parent: Runner | None = None,
        agent: Agent | None = None,
    ) -> None:
        super().__init__(
            runtime=runtime,
            node=node,
            model_type=model_type,
            parent=parent,
            run=run,
            agent=agent,
        )
        self.model_type = model_type
        self.prompt = prompt
        self.code: Code | None = None


def get_chat_model_runner_cls(
    model_developer: ModelDeveloper, model_type: ModelType
) -> type[ChatModelRunner]:
    """Get the ChatModelRunner class for the given model type."""
    from bench.runtime.model import (
        AnthropicChatModelRunner,
        GeminiChatModelRunner,
        OpenAIChatModelRunner,
    )

    if model_developer == ModelDeveloper.OPENAI:
        return OpenAIChatModelRunner
    elif model_developer == ModelDeveloper.ANTHROPIC:
        return AnthropicChatModelRunner
    elif model_developer == ModelDeveloper.GOOGLE:
        return GeminiChatModelRunner
    else:
        raise NotSupportedError(f"unsupported model developer {model_developer!r}")
