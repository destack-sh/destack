import abc
from typing import override

from bench.language.projection import Projection, project
from bench.language.run import ModelProvider, RunnableKind
from bench.runtime.runner import Runner, runner


class ModelRunnerBase(Runner, abc.ABC):
    """The base for text function runners."""

    @override
    async def run(self) -> None:
        projection = project(self.state.node)

    @abc.abstractmethod
    async def _do_generate_code(self, projection: Projection) -> str: ...


@runner((RunnableKind.TEXT, None))
class ModelRouter(ModelRunnerBase):
    """The router for text functions without explicitly assigned models/providers."""

    @override
    async def run(self) -> None:
        raise NotImplementedError


@runner((RunnableKind.TEXT, ModelProvider.OPENAI))
class OpenaiModelRunner(ModelRunnerBase):
    @override
    async def _do_generate_code(self, projection: Projection) -> str:
        raise NotImplementedError


@runner((RunnableKind.TEXT, ModelProvider.ANTHROPIC))
class AnthropicModelRunner(ModelRunnerBase):
    pass
