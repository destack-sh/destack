import enum
from dataclasses import dataclass


@dataclass
class SimulationSpec:
    name: str
    seed: int
    network: "NetworkSpec"
    benches: tuple["BenchSpec", ...]
    clients: tuple["ClientSpec", ...]
    workloads: tuple["WorkloadSpec", ...]


@dataclass
class NetworkSpec:
    one_way_latency: float


@dataclass
class BenchSpec:
    name: str


@dataclass
class ClientSpec:
    name: str


class WorkloadType(enum.StrEnum):
    pass


@dataclass
class WorkloadSpec:
    type: WorkloadType
    name: str
