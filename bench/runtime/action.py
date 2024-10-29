from typing import override

from bench.runtime.runner import Runner


class ActionRunner(Runner):
    @override
    async def run_once(self) -> None:
        raise NotImplementedError("nocheckin: run action")
