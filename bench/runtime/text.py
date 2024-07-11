from typing import override

from bench.language.run import RunnableKind
from bench.runtime.runner import Runner, runner


@runner((RunnableKind.TEXT, None))
class TextRunner(Runner):
    """The router and base for text function runners."""

    @override
    async def run(self) -> None:
        raise NotImplementedError


class OpenaiRunner(TextRunner):
    pass


class AnthropicRunner(TextRunner):
    pass
