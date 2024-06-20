from typing import final

import structlog

from bench.test.simulation.activity import ActivityBase
from bench.test.simulation.spec import SimulationSpec

logger = structlog.get_logger(__name__)


@final
class Simulation:
    """An active simulation"""

    def __init__(self, spec: SimulationSpec):
        self.spec = spec
        self.activities: list[ActivityBase] = []
        for activity in spec.activities:
            ...

    async def run(self):
        raise NotImplementedError
