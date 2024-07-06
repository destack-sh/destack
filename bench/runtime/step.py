import abc
from typing import override

from bench.language.block import Block
from bench.language.run import RunnableKind
from bench.language.step import Step
from bench.runtime.runner import Runner, runner


class StepRunnerBase(Runner[Step], abc.ABC):
    pass


@runner((RunnableKind.STEP, None))
class FlowRunner(Runner[Block]):
    @override
    async def run(self) -> None:
        raise NotImplementedError
