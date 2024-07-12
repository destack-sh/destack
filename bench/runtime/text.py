import abc
from typing import override

from bench.language.code import Code
from bench.language.projection import Projection, project
from bench.language.run import ModelProvider, RunKind
from bench.runtime.core import RUN_ONCE
from bench.runtime.runner import Runner, runner


class ModelRunnerBase(Runner, abc.ABC):
    """
    The base for text function runners.
    Basically, the model generate codes that produces the answer, we run it and return that.
    """

    @override
    async def run(self) -> None:
        projection = project(self.state.node)
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


@runner(RunKind.TEXT, ModelProvider.OPENAI)
class OpenaiModelRunner(ModelRunnerBase):
    @override
    async def _do_generate_code(self, projection: Projection) -> str:
        raise NotImplementedError


@runner(RunKind.TEXT, ModelProvider.ANTHROPIC)
class AnthropicModelRunner(ModelRunnerBase):
    pass
