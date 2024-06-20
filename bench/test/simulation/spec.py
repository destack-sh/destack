import enum
from dataclasses import dataclass, field
from datetime import timedelta

import structlog

logger = structlog.get_logger(__name__)


@dataclass
class SimulationSpec:
    """A simulation to run."""

    name: str
    seed: int = 0
    network: "NetworkSpec" = field(default_factory=lambda: NetworkSpec())
    monkey: "MonkeySpec" = field(default_factory=lambda: MonkeySpec())
    benches: tuple["BenchSpec", ...] = field(default_factory=lambda: (BenchSpec(),))
    clients: tuple["ClientSpec", ...] = field(default_factory=lambda: (ClientSpec(name="Client1"),))
    activities: tuple["ActivitySpec", ...] = ()


@dataclass
class MonkeySpec:
    """Inject failures into the services"""

    failure_probability: float = 0.0
    recovery_probability: float = 1.0
    recovery_time_min: float = 0.0
    recovery_time_mean: float = 0.0


@dataclass
class NetworkSpec:
    """The network conditions (client<->service and service<->external)"""

    latency_min: float = 0.0
    latency_mean: float = 0.0
    partition_probability: float = 0.0


@dataclass
class BenchSpec:
    """A bench to operate on"""

    name: str = "testbench"
    owner: str = "testuser"


class ClientType(enum.StrEnum):
    USER = "user"


@dataclass
class ClientSpec:
    """A client for doing.. stuff"""

    name: str
    username: str = "testuser"
    time_offset: timedelta | None = None
    type: ClientType = ClientType.USER


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
    repeat: int = 1
    client: str | None = None
    session: str | None = None
