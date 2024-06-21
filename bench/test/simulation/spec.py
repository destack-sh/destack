import enum
from dataclasses import dataclass, field
from datetime import timedelta

import structlog

from bench.language.const import ClientType

logger = structlog.get_logger(__name__)


@dataclass
class SimulationSpec:
    """A simulation to run."""

    name: str
    seed: int = 0
    network: "NetworkSpec" = field(default_factory=lambda: NetworkSpec())
    clients: tuple["ClientSpec", ...] = ()
    supervisor: "SupervisorSpec" = field(default_factory=lambda: SupervisorSpec())
    hosts: tuple["HostSpec", ...] = ()
    workloads: tuple["WorkloadSpec", ...] = ()


@dataclass
class ServiceSpec:
    """A service to simulate"""

    pass


@dataclass
class NetworkSpec:
    """The network conditions (client<->service and service<->external)"""

    latency_min: float = 0.0
    latency_mean: float = 0.0
    partition_probability: float = 0.0


@dataclass
class SupervisorSpec(ServiceSpec):
    """A supervisor service"""

    failure_probability: float = 0.0
    recovery_probability: float = 1.0
    recovery_time_min: float = 0.0
    recovery_time_mean: float = 0.0


@dataclass
class HostSpec(ServiceSpec):
    """A host to run a Bench"""

    bench: "BenchSpec"
    failure_probability: float = 0.0
    recovery_probability: float = 1.0
    recovery_time_min: float = 0.0
    recovery_time_mean: float = 0.0


@dataclass
class BenchSpec:
    """A bench to operate on"""

    name: str
    owner: str


@dataclass
class ClientSpec:
    """A client for doing.. stuff"""

    name: str
    username: str
    type: ClientType = ClientType.BENCH_DESKTOP
    time_offset: timedelta | None = None


class WorkloadType(enum.StrEnum):
    REPLAY_LOG = "replay_log"
    WRITE_BLOCK_TREE = "write_block_tree"
    READ_PACKAGE = "read_package"


@dataclass
class WorkloadSpec:
    """Some workload to run"""

    type: WorkloadType
    name: str = None  # type: ignore (default to 'type' in __post_init__)
    repeat: int = 1
    repeat_interval: float = 0.0

    def __post_init__(self):
        if self.name is None:
            if hasattr(self, "client"):
                self.name = f"{self.type.value}-{getattr(self, 'client')}"
            else:
                self.name = self.type.value
