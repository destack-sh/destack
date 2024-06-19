import enum
from dataclasses import dataclass


@dataclass
class SimulationSpec:
    name: str
    workloads: list["WorkloadSpec"]


class WorkloadType(enum.StrEnum):
    pass


@dataclass
class WorkloadSpec:
    name: str
    type: WorkloadType


# ... in specific Workload types
