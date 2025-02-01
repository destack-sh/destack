from dataclasses import dataclass, field
from datetime import timedelta
from typing import TYPE_CHECKING, Literal

import structlog

from bench.language import ClientType
from bench.test.conftest import TestProfile

from .sample import SampledFloat

if TYPE_CHECKING:
    from bench.test.simulation.workload import WorkloadSpec

logger = structlog.get_logger(__name__)


@dataclass
class SimulationSpec:
    """A simulation to run."""

    name: str
    description: str
    profile: TestProfile = TestProfile.DEFAULT
    seed: int = 0
    network: "NetworkSpec" = field(default_factory=lambda: NetworkSpec())
    users: tuple["UserSpec", ...] = ()
    benches: tuple["BenchSpec", ...] = ()
    machines: tuple["MachineSpec", ...] = ()
    clients: tuple["ClientSpec", ...] = ()
    supervisor: "SupervisorSpec" = field(default_factory=lambda: SupervisorSpec())
    hosts: tuple["HostSpec", ...] = ()
    workloads: tuple["WorkloadSpec", ...] = ()


@dataclass
class UserSpec:
    """A User to run a Bench"""

    name: str


@dataclass
class OrganizationSpec:
    """An Organization to run a Bench"""

    name: str
    members: tuple["UserSpec", ...] = ()


@dataclass
class BenchSpec:
    """A bench to operate on"""

    name: str
    owner: str


@dataclass
class NetworkSpec:
    """The network conditions (client<->service and service<->external)"""

    latency: float | SampledFloat = 0.0
    partition_probability: float | SampledFloat = 0.0


@dataclass
class ServiceSpec:
    """A service to simulate"""

    failure_probability: float | SampledFloat = 0.0
    recovery_probability: float | SampledFloat = 1.0
    recovery_time: float | SampledFloat = 0.0


@dataclass
class SupervisorSpec(ServiceSpec):
    """A supervisor service"""

    pass


@dataclass
class HostSpec(ServiceSpec):
    """A Host to run a Bench"""

    bench: str = ""


@dataclass
class MachineSpec(ServiceSpec):
    """A Machine that's a Runtime for a Bench"""

    name: str = ""
    bench: str = ""
    max_threads: int = 1
    max_concurrency_per_thread: int = 8


@dataclass
class ClientSpec:
    """A Client"""

    name: str
    parent: tuple[Literal["user", "machine"], str]
    type: ClientType = ClientType.DESKTOP
    time_offset: timedelta | None = None
