import abc
from dataclasses import dataclass
from typing import Any, ClassVar, Literal, Mapping, override

import anthropic
import openai

from bench.language.code import Code
from bench.language.project import Projection, ProjectOptions, project
from bench.language.render import RenderOptions, render
from bench.language.run import ModelProvider, ModelType, RunKind
from bench.runtime.core import RUN_ONCE, NotRunnableError
from bench.runtime.runner import Runner, runner
from bench.utils.utils import get_from_env


@runner(RunKind.TEXT, None)
class ModelRouter(Runner):
    """The router for text functions without explicitly assigned models/providers."""

    @override
    async def run(self) -> None:
        model_handle = await self.runtime.make_run_handle(
            RunKind.TEXT, key=ModelProvider.ANTHROPIC, node=self.node, options=RUN_ONCE, track=False
        )
        await self.runtime.run_handle(model_handle)
        self.handle.outputs = model_handle.outputs


#
# Chat models
#


@dataclass
class _ChatMessage:
    role: Literal["user", "assistant", "system"]
    content: str


class ChatModelRunnerBase(Runner, abc.ABC):
    """
    The base for chat-like text function runners.
    Basically, the model generate codes that produces the answer, we run it and return that.
    """

    @property
    def model(self) -> ModelType | None:
        if self.handle.options.model_options is not None:
            return self.handle.options.model_options.model
        else:
            return None

    @override
    async def run(self) -> None:
        projection = project(self.state.node, self.handle.inputs, options=ProjectOptions())
        code_str = await self._generate_code(projection)
        code = Code.from_string(code_str)
        code_handle = await self.runtime.make_run_handle(
            RunKind.CODE, node=self.node, code=code, options=RUN_ONCE, track=True
        )
        await self.runtime.run_handle(code_handle)
        self.handle.outputs = code_handle.outputs

    async def _make_messages(
        self, projection: Projection, *, include_system_message: bool
    ) -> list[_ChatMessage]:
        # NOTE :Incomplete: add attempts feedback to messages
        # NOTE :Incomplete: add images/file references to messages
        # rendered_context = render(...)  # nocheckin: context
        render_options = RenderOptions(scope=self.node)
        rendered_task = render(self.node, options=render_options)
        rendered_inputs = render(self.inputs, options=render_options)
        messages: list[_ChatMessage] = [
            # _ChatMessage("user", f"#\n# Some context for your task\n# \n\n{rendered_context}"),
            _ChatMessage("user", f"#\n# Inputs for your task\n# \n\n{rendered_inputs}"),
            _ChatMessage("user", f"#\n# Your task is `{self.node.name}`\n# \n\n{rendered_task}"),
            _ChatMessage(
                "assistant",
                "# here is how I'll return the answer explained in comments, followed by the code: \n",
            ),
        ]
        if include_system_message:
            messages = [_ChatMessage("system", SYSTEM_MESSAGE), *messages]
        return messages

    @abc.abstractmethod
    async def _generate_code(self, projection: Projection) -> str: ...


openai_client = openai.AsyncClient(
    api_key=get_from_env("OPENAI_API_KEY", description="OpenAI API key")
)
anthropic_client = anthropic.AsyncClient(
    api_key=get_from_env("ANTHROPIC_API_KEY", description="Anthropic API key")
)

SYSTEM_MESSAGE = ""


@runner(RunKind.TEXT, ModelProvider.OPENAI)
class OpenaiModelRunner(ChatModelRunnerBase):
    DEFAULT_MODEL = ModelType.GPT40
    MODEL_BY_TYPE: ClassVar[Mapping[ModelType, str]] = {ModelType.GPT40: "gpt-4o"}

    def _convert_message(self, message: _ChatMessage) -> Any:  # nocheckin: type this
        return {"role": message.role, "content": message.content}

    @override
    async def _generate_code(self, projection: Projection) -> str:
        model = self.model or self.DEFAULT_MODEL
        model_key = self.MODEL_BY_TYPE.get(model)
        if model_key is None:
            raise NotRunnableError(f"unsupported model type {self.state.key}")
        messages = await self._make_messages(projection, include_system_message=True)
        completion = await openai_client.chat.completions.create(
            messages=[self._convert_message(message) for message in messages],
            model=model_key,
            temperature=0.1,
            user=str(self.runtime.package.id),
        )
        assert completion.choices[0].message.content, "empty completion"
        return completion.choices[0].message.content


@runner(RunKind.TEXT, ModelProvider.ANTHROPIC)
class AnthropicModelRunner(ChatModelRunnerBase):
    DEFAULT_MODEL = ModelType.CLAUDE_3_5_SONNET
    MODEL_BY_TYPE: ClassVar[Mapping[ModelType, str]] = {
        ModelType.CLAUDE_3_5_SONNET: "claude-3-5-sonnet-20240620"
    }

    def _convert_message(self, message: _ChatMessage) -> anthropic.types.MessageParam:
        assert message.role != "system", "system messages are not supported"
        return {"role": message.role, "content": message.content}

    @override
    async def _generate_code(self, projection: Projection) -> str:
        model = self.model or self.DEFAULT_MODEL
        model_key = self.MODEL_BY_TYPE.get(model)
        if model_key is None:
            raise NotRunnableError(f"unsupported model type {self.state.key}")
        messages = await self._make_messages(projection, include_system_message=False)
        completion = await anthropic_client.messages.create(
            system=SYSTEM_MESSAGE,
            messages=[self._convert_message(message) for message in messages],
            model=model_key,
            temperature=0.1,
            max_tokens=1024 * 8,
        )
        assert completion.content, "empty completion"
        assert (
            completion.content[0].type == "text"
        ), f"unexpected completion type: {completion.content}"
        return completion.content[0].text
