import abc
from typing import ClassVar, Mapping, override

import anthropic
import openai

from bench.language.code import Code
from bench.language.project import Projection, ProjectOptions, project
from bench.language.run import ModelProvider, ModelType, RunKind
from bench.runtime.core import RUN_ONCE, NotRunnableError
from bench.runtime.runner import Runner, runner
from bench.utils.utils import get_from_env


class ModelRunnerBase(Runner, abc.ABC):
    """
    The base for text function runners.
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
        code_str = await self._do_generate_code(projection)
        code = Code.from_string(code_str)
        code_handle = await self.runtime.make_run_handle(
            RunKind.CODE, node=self.node, code=code, options=RUN_ONCE, track=True
        )
        await self.runtime.run_handle(code_handle)
        self.handle.outputs = code_handle.outputs

    @abc.abstractmethod
    async def _do_generate_code(self, projection: Projection) -> str: ...


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


openai_client = openai.AsyncClient(
    api_key=get_from_env("OPENAI_API_KEY", description="OpenAI API key")
)
anthropic_client = anthropic.AsyncClient(
    api_key=get_from_env("ANTHROPIC_API_KEY", description="Anthropic API key")
)


@runner(RunKind.TEXT, ModelProvider.OPENAI)
class OpenaiModelRunner(ModelRunnerBase):
    DEFAULT_MODEL = ModelType.GPT40
    MODEL_BY_TYPE: ClassVar[Mapping[ModelType, str]] = {ModelType.GPT40: "gpt-4o"}

    @override
    async def _do_generate_code(self, projection: Projection) -> str:
        model = self.model or self.DEFAULT_MODEL
        model_key = self.MODEL_BY_TYPE.get(model)
        if model_key is None:
            raise NotRunnableError(f"unsupported model type {self.state.key}")
        messages = ...  # nocheckin
        completion = await openai_client.chat.completions.create(
            messages=messages, model=model_key, temperature=0.1
        )
        assert completion.choices[0].message.content, "empty completion"
        return completion.choices[0].message.content


@runner(RunKind.TEXT, ModelProvider.ANTHROPIC)
class AnthropicModelRunner(ModelRunnerBase):
    DEFAULT_MODEL = ModelType.CLAUDE_3_5_SONNET
    MODEL_BY_TYPE: ClassVar[Mapping[ModelType, str]] = {
        ModelType.CLAUDE_3_5_SONNET: "claude-3-5-sonnet-20240620"
    }

    @override
    async def _do_generate_code(self, projection: Projection) -> str:
        model = self.model or self.DEFAULT_MODEL
        model_key = self.MODEL_BY_TYPE.get(model)
        if model_key is None:
            raise NotRunnableError(f"unsupported model type {self.state.key}")
        messages = ...  # nocheckin
        completion = await anthropic_client.messages.create(
            messages=messages,
            model=model_key,
            temperature=0.1,
            max_tokens=1024 * 8,
        )
        assert completion.content, "empty completion"
        assert (
            completion.content[0].type == "text"
        ), f"unexpected completion type: {completion.content}"
        return completion.content[0].text
