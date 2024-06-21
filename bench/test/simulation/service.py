import abc
import enum
from typing import TYPE_CHECKING, final, override
from uuid import UUID

from betterproto import ServiceStub

from bench.proto.services import ServiceBase
from bench.proto.wire import HostClient, SupervisorClient
from bench.system.host import Host
from bench.system.supervisor import Supervisor
from bench.test.simulation.spec import HostSpec, ServiceSpec
from bench.utils.oracle import Oracle

if TYPE_CHECKING:
    from bench.test.simulation.simulation import Simulation


class ServiceStatus(enum.Enum):
    UP = 1
    FAILING = 2
    DOWN = 3
    RECOVERING = 4


class ServiceHandle[S: ServiceBase, C: ServiceStub](abc.ABC):
    """Wrapper for a simulated  service we can monkey around with"""

    service_cls: type[S]
    client_cls: type[C]

    def __init__(self, spec: ServiceSpec, oracle: Oracle, simulation: "Simulation"):
        self.spec = spec
        self.oracle = oracle
        self.simulation = simulation
        self._service: S | None = None

    @property
    def service(self) -> S:
        assert self._service is not None, f"f{self!r} not started yet"
        return self._service

    @abc.abstractmethod
    async def _do_start(self) -> S:
        pass

    @final
    async def _make_client(self) -> C:
        raise NotImplementedError

    @final
    async def run(self):
        raise NotImplementedError

    def close(self):
        if self._service is not None:
            self._service.close()

    async def wait_closed(self):
        if self._service is not None:
            await self._service.wait_closed()
            self._service = None


@final
class SupervisorHandle(ServiceHandle[Supervisor, SupervisorClient]):
    """A global Supervisor"""

    service_cls = Supervisor
    client_cls = SupervisorClient

    @override
    async def _do_start(self):
        return Supervisor(global_store=self.simulation.global_store, oracle=self.oracle)


@final
class HostHandle(ServiceHandle[Host, HostClient]):
    """A Host for a Bench"""

    def __init__(self, bench_id: UUID, spec: HostSpec, oracle: Oracle, simulation: "Simulation"):
        self.bench_id = bench_id

    @override
    async def _do_start(self) -> Host:
        return Host(
            bench_id=self.bench_id, global_store=self.simulation.global_store, oracle=self.oracle
        )
