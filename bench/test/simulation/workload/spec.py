import enum
from dataclasses import dataclass

from bench.test.simulation.core import SampledFloat, SampledInt


class WorkloadType(enum.StrEnum):
    WRITE_BLOCK_TREE = "write_block_tree"
    READ_PACKAGE = "read_package"
    WATCH_LOGS = "watch_logs"


@dataclass
class WorkloadSpec:
    """A Workload to run"""

    type: WorkloadType
    name: str = None  # type: ignore (default to 'type' in __post_init__)
    repeat: int | SampledInt = 1
    repeat_interval: float | SampledFloat = 0.0
    duration: float | SampledFloat = 0.0
    group: str | None = None

    def __post_init__(self):
        if self.name is None:
            if hasattr(self, "client"):
                self.name = f"{self.type.value}-{getattr(self, 'client')}"
            else:
                self.name = self.type.value
