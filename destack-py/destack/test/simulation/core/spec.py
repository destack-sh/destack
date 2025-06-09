from dataclasses import dataclass, field
from datetime import timedelta
from typing import TYPE_CHECKING, Literal

import structlog

from destack.language import REGION, ClientType, Region
from destack.test.conftest import TestProfile

from .sample import SampledFloat

if TYPE_CHECKING:
    from destack.test.simulation.workload import WorkloadSpec

logger = structlog.get_logger(__name__)


@dataclass
class SimulationSpec:
    """The Simulation configuration."""

    name: str
    description: str
    profile: TestProfile = TestProfile.DEFAULT
    seed: int = 0
    network: "NetworkSpec" = field(default_factory=lambda: NetworkSpec())
    users: tuple["UserSpec", ...] = ()
    destackes: tuple["DestackSpec", ...] = ()
    machines: tuple["MachineSpec", ...] = ()
    clients: tuple["ClientSpec", ...] = ()
    runtimes: tuple["RuntimeSpec", ...] = ()
    supervisor: "SupervisorSpec" = field(default_factory=lambda: SupervisorSpec())
    hosts: tuple["HostSpec", ...] = ()
    workloads: tuple["WorkloadSpec", ...] = ()
    system: bool = False


@dataclass
class UserSpec:
    """A User"""

    name: str
    region: Region = REGION


@dataclass
class OrganizationSpec:
    """An Organization"""

    name: str
    members: tuple["UserSpec", ...] = ()


@dataclass
class DestackSpec:
    """A Destack"""

    name: str
    owner: str


@dataclass
class NetworkSpec:
    """The network conditions (client<->service and service<->external)"""

    latency: float | SampledFloat = 0.0
    partition_probability: float | SampledFloat = 0.0


@dataclass
class ServiceSpec:
    """A Service to simulate"""

    failure_probability: float | SampledFloat = 0.0
    recovery_probability: float | SampledFloat = 1.0
    recovery_time: float | SampledFloat = 0.0


@dataclass
class SupervisorSpec(ServiceSpec):
    """A Supervisor service"""

    pass


@dataclass
class HostSpec(ServiceSpec):
    """A Host Service to run a Destack"""

    destack: str = ""


@dataclass
class MachineSpec:
    """A Machine that's a Runtime for a Destack"""

    name: str = ""
    destack: str = ""


@dataclass
class RuntimeSpec(ServiceSpec):
    """A Machine that's a Runtime for a Destack"""

    name: str = ""
    machine: str = ""
    max_processs: int = 1


@dataclass
class ClientSpec:
    """A Client"""

    name: str
    parent: tuple[Literal["user", "machine"], str]
    type: ClientType = ClientType.DESKTOP
    time_offset: timedelta | None = None
