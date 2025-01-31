import abc
from datetime import datetime
from typing import assert_never, final
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language import (
    BENCH_NODE_TYPES,
    EMPTY_SCOPE_DATA,
    PUBLIC_NODE_TYPES,
    Client,
    GraphScope,
    NodeReference,
    NodeSuperGraph,
    NodeType,
    RemoteEngine,
    Session,
    User,
)
from bench.language.resource import Machine
from bench.proto.wiring import unpack_builtin_object
from bench.test.simulation.core import (
    ClientHandle,
    HostHandle,
    MachineHandle,
    Simulation,
    UserHandle,
    to_value,
)
from bench.utils.oracle import Oracle
from bench.utils.string import Casing, to_casing
from bench.utils.tenacity import RETRY_GRPC_FOREVER

from .spec import WorkloadSpec, WorkloadType

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


_workload_cls_by_type: dict[WorkloadType, type["Workload"]] = {}
_workload_spec_cls_by_type: dict[WorkloadType, type[WorkloadSpec]] = {}


def get_workload_cls(typ: WorkloadType) -> type["Workload"]:
    workload_cls = _workload_cls_by_type.get(typ)
    assert workload_cls is not None, f"unknown workload type {typ}"
    return workload_cls


def workload_(typ: WorkloadType, spec_cls: type[WorkloadSpec]):
    def _decorator(cls):
        assert typ not in _workload_cls_by_type, f"duplicate workload type {typ}"
        _workload_cls_by_type[typ] = cls
        _workload_spec_cls_by_type[typ] = spec_cls
        return cls

    return _decorator


class Workload[SpecT: WorkloadSpec](abc.ABC):
    """A simulated Workload."""

    def __init__(self, spec: SpecT, oracle: Oracle, simulation: "Simulation"):
        self.spec = spec
        self.oracle = oracle
        self.simulation = simulation
        logger = structlog.get_logger(to_casing(self.__class__.__name__, Casing.SNAKE))
        self.log = logger.bind(workload=self)
        self.started_at: datetime | None = None
        self.terminated_at: datetime | None = None

    def __str__(self) -> str:
        return ""

    @final
    def __repr__(self):
        content_str = str(self)
        if self.started_at is not None:
            if self.terminated_at is not None:
                duration = (self.terminated_at - self.started_at).total_seconds()
                runtime_ns = f"runtime={(duration):.3f}s"
            else:
                duration = (self.oracle.utc() - self.started_at).total_seconds()
                runtime_ns = f"runtime={(duration):.3f}s"
        else:
            runtime_ns = "runtime=<not started>"
        if content_str:
            return f"<{self.__class__.__name__} {content_str}, {runtime_ns}>"
        else:
            return f"<{self.__class__.__name__} {runtime_ns}>"

    @property
    def name(self):
        return self.spec.name

    @property
    def network(self):
        return self.simulation.network

    @property
    def random(self):
        return self.simulation.random

    async def prepare(self):  # noqa: B027
        """Prepare the Workload before running it."""
        pass  # nothing to do by default

    @final
    async def run(self):
        """Runs the full Workload."""
        self.started_at = self.oracle.utc()
        try:
            n_runs = 1
            repeat = to_value(self.random, self.spec.repeat)
            repeat_interval = to_value(self.random, self.spec.repeat_interval)
            while n_runs <= repeat:
                with tracer.start_as_current_span(f"workload.{self.name}"):
                    await self.run_once()
                    self.log.info("workload.run", run=n_runs, span="current")
                    await self.oracle.sleep(repeat_interval)
                n_runs += 1
        finally:
            self.terminated_at = self.oracle.utc()

    @abc.abstractmethod
    async def run_once(self):
        """Runs one repetition of the Workload."""
        raise NotImplementedError

    @final
    @tracer.start_as_current_span("workload.check")
    async def check(self):
        """Validate any post-run conditions."""
        await self.check_self()
        if self.spec.group:
            group = self.simulation.get_workload_group(self.spec.group)
            # NOTE :Performance :Test: check in-group pairings only as needed
            await self.check_group(group)
        self.log.debug("workload.check", span="current")

    async def check_self(self):  # noqa: B027
        """Validates any post-run conditions."""
        pass

    async def check_group(self, group: list["Workload"]):  # noqa: B027
        """Validates any post-run conditions for a group of workloads. Called for every workload in the group."""
        pass

    async def make_remote_session(
        self,
        bench_id: UUID,
        client: "ClientHandle",
        host: "HostHandle",
        oracle: Oracle,
        supergraph: NodeSuperGraph | None = None,
    ):
        """Create a Session to a remote Bench's Host"""
        nonce = str(UUID(int=oracle.random.getrandbits(128)))
        supervisor_client = await self.simulation.supervisor.connect(client)
        host_client = await host.connect(client)
        engines = (
            # global engine
            RemoteEngine(
                name="remote-global",
                scope=EMPTY_SCOPE_DATA,
                node_types=PUBLIC_NODE_TYPES,
                remote=supervisor_client,
                write_retry=RETRY_GRPC_FOREVER,
                rpc_metadata=client.rpc_metadata,
            ),
            # bench engine
            RemoteEngine(
                name="remote-bench",
                scope=GraphScope(bench_id=bench_id)._to_data(),
                node_types=BENCH_NODE_TYPES,
                remote=host_client,
                write_retry=RETRY_GRPC_FOREVER,
                rpc_metadata=client.rpc_metadata,
            ),
        )
        if supergraph is None:
            root_ptr = NodeReference(node_type=NodeType.BENCH, id=bench_id, ck=bench_id)
            supergraph = NodeSuperGraph(name="Remote", root_ptr=root_ptr)
        session = Session(
            _is_readonly=False,
            _default_scope=GraphScope(bench_id=bench_id)._to_data(),
            _engines=engines,
            _origin=client.to_origin(nonce=nonce),
            _supervisor=supervisor_client,
            _host=host_client,
            _oracle=oracle,
            _supergraph=supergraph,
        )
        if isinstance(client.parent, UserHandle):
            session.user = unpack_builtin_object(
                client.parent.user_data,
                session=session,
                supergraph=supergraph,
                expect=User,
                skip_add_self=False,
            )
        elif isinstance(client.parent, MachineHandle):
            session.machine = unpack_builtin_object(
                client.parent.machine_data,
                session=session,
                supergraph=supergraph,
                expect=Machine,
                skip_add_self=False,
            )
        else:
            assert_never(client.parent)
        session.client = unpack_builtin_object(
            client.client_data,
            session=session,
            supergraph=supergraph,
            expect=Client,
            skip_add_self=False,
        )
        session._subject = session.user
        return session
