import enum
from dataclasses import dataclass

import structlog

logger = structlog.get_logger(__name__)


@dataclass
class SimulationSpec:
    """A simulation to run."""

    name: str
    seed: int
    network: "NetworkSpec"
    services: "ServiceSpec"
    benches: tuple["BenchSpec", ...]
    clients: tuple["ClientSpec", ...]
    activities: tuple["ActivitySpec", ...]


@dataclass
class ServiceSpec:
    """The service health conditions"""

    failure_probability: float
    recovery_probability: float
    recovery_time_min: float
    recovery_time_mean: float


@dataclass
class NetworkSpec:
    """The network conditions (client<->service and service<->external)"""

    latency_min: float
    latency_mean: float
    partition_probability: float


@dataclass
class BenchSpec:
    """A bench to operate on"""

    name: str


class ClientType(enum.StrEnum):
    pass


@dataclass
class ClientSpec:
    """A client for doing.. stuff"""

    type: ClientType
    name: str
    username: str


class ActivityType(enum.StrEnum):
    REPLAY_LOG = "replay_log"
    WRITE_BLOCK_TREE = "write_block_tree"
    READ_BENCH = "read_bench"
    READ_PACKAGE = "read_package"


@dataclass
class ActivitySpec:
    """Some workload to run"""

    type: ActivityType
    name: str
    repeat: int
    client: str | None
    session: str | None
