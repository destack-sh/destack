from typing import override

from bench.language.run import RunnableKind
from bench.runtime.runner import Runner, runner


@runner((RunnableKind.TEXT, None))
class TextFunctionRunner(Runner):
    @override
    async def run(self) -> None:
        raise NotImplementedError
