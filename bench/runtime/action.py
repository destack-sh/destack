from typing import ClassVar, cast, override

from bench.language.block import ActionBlock
from bench.language.const import BlockType
from bench.language.flow import ActionStep, StepType
from bench.language.run import RunKind
from bench.runtime.runner import Runner


class ActionRunner(Runner):
    kind: ClassVar[RunKind] = RunKind.CODE

    @override
    async def run_once(self) -> None:
        assert self.node.type in (BlockType.ACTION, StepType.ACTION), f"unexpected node {self!r}"
        action = cast(ActionStep | ActionBlock, self.node)
        raise NotImplementedError("nocheckin: run action")
