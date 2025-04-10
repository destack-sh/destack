import textwrap
from abc import ABC
from typing import TYPE_CHECKING

import regex
import structlog
from opentelemetry import trace

from bench.language import Agent, Code, ModelDeveloper, ModelType, Runnable, RunOptions
from bench.runtime.core import NotSupportedError, RunIn, Runner, Runtime

from .macro import FUNCTION_MACROS
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
        options: RunOptions,
        run: RunIn,
        prompt: Prompt,
        parent: Runner | None = None,
        agent: Agent | None = None,
    ) -> None:
        super().__init__(
            runtime=runtime,
            node=node,
            model_type=model_type,
            options=options,
            parent=parent,
            run=run,
            agent=agent,
        )
        self.model_type = model_type
        self.prompt = prompt
        self.code: Code | None = None

    def clean_code(self, code: str) -> str:
        """Standardize code completion."""
        # clean completion
        code = code.strip()
        # strip ``` ... ``` wrapper
        code = regex.sub(r"^```[a-zA-Z]*\n", "", code)
        code = regex.sub(r"\n```$", "", code)
        # replace suspicious unicode characters
        code = code.replace("’", "'")  # noqa: RUF001
        code = code.replace("‘", "'")  # noqa: RUF001
        code = code.replace("“", '"')
        code = code.replace("”", '"')
        # dedent
        code = textwrap.dedent(code)
        return code

    def expand_code(self, code: str) -> str:
        """Expand macros into a code completion."""
        for macro in FUNCTION_MACROS:
            code = macro.expand(code)
        return code


def get_chat_model_runner_cls(
    model_developer: ModelDeveloper, model_type: ModelType
) -> type[ChatModelRunner]:
    """Get the ChatModelRunner class for the given model type."""
    from bench.runtime.model import AnthropicChatModelRunner, OpenAIChatModelRunner

    if model_developer == ModelDeveloper.OPENAI:
        return OpenAIChatModelRunner
    elif model_developer == ModelDeveloper.ANTHROPIC:
        return AnthropicChatModelRunner
    else:
        raise NotSupportedError(f"unsupported model developer {model_developer!r}")
