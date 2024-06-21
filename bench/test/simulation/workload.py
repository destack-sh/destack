import abc
from contextlib import asynccontextmanager
from dataclasses import dataclass
from typing import TYPE_CHECKING, final
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.bench import Bench, Branch, Package
from bench.language.connection import GraphEngine, RemoteEngine
from bench.language.const import (
    BENCH_NODE_TYPES,
    EDIT_TYPES,
    IN_PACKAGE_NODE_TYPES,
    PUBLIC_NODE_TYPES,
    SOURCE_NODE_TYPES,
    BlockType,
    EditType,
    NodeType,
)
from bench.language.expression import NodeReference
from bench.language.graph import NodeSuperGraph
from bench.language.session import Session
from bench.proto.wire import GraphScope, HostClient, SupervisorClient
from bench.test.simulation.spec import WorkloadSpec, WorkloadType
from bench.test.simulation.utils import SampledInt, to_value, to_value_maybe
from bench.utils.oracle import Oracle
from bench.utils.tenacity import RETRY_GRPC_FOREVER

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

if TYPE_CHECKING:
    from bench.test.simulation.simulation import ClientHandle, HostHandle, Simulation


_workload_cls_by_type: dict[WorkloadType, type["WorkloadBase"]] = {}
_workload_spec_cls_by_type: dict[WorkloadType, type[WorkloadSpec]] = {}


def get_workload_cls(typ: WorkloadType) -> type["WorkloadBase"]:
    workload_cls = _workload_cls_by_type.get(typ)
    assert workload_cls is not None, f"unknown workload type {typ}"
    return workload_cls


def workload(typ: WorkloadType, spec_cls: type[WorkloadSpec]):
    def _decorator(cls):
        assert typ not in _workload_cls_by_type, f"duplicate workload type {typ}"
        _workload_cls_by_type[typ] = cls
        _workload_spec_cls_by_type[typ] = spec_cls
        return cls

    return _decorator


class WorkloadBase[SpecT: WorkloadSpec](abc.ABC):
    def __init__(self, spec: SpecT, oracle: Oracle, simulation: "Simulation"):
        self.spec = spec
        self.oracle = oracle
        self.simulation = simulation

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
        pass

    @final
    async def run(self):
        n_runs = 1
        while n_runs <= self.spec.repeat:
            with tracer.start_as_current_span(f"workload.{self.name}"):
                await self._do_run()
                await self.oracle.sleep(self.spec.repeat_interval)
            n_runs += 1

    @abc.abstractmethod
    async def _do_run(self):
        raise NotImplementedError


def _make_remote_engines(
    bench_id: UUID,
    client: "ClientHandle",
    supervisor_client: SupervisorClient,
    host_client: HostClient,
) -> tuple[GraphEngine, ...]:
    """Gets the graph engines to connect with a remote Bench"""
    engines = (
        # global engine
        RemoteEngine(
            scope=GraphScope(),
            node_types=PUBLIC_NODE_TYPES,
            remote=supervisor_client,
            retry=RETRY_GRPC_FOREVER,
            rpc_metadata=client.rpc_metadata,
        ),
        # bench engine
        RemoteEngine(
            scope=GraphScope(bench_id=str(bench_id)),
            node_types=BENCH_NODE_TYPES | IN_PACKAGE_NODE_TYPES,
            remote=host_client,
            retry=RETRY_GRPC_FOREVER,
            rpc_metadata=client.rpc_metadata,
        ),
    )
    return engines


async def _make_remote_session(
    bench_id: UUID,
    client: "ClientHandle",
    host: "HostHandle",
    oracle: Oracle,
    simulation: "Simulation",
    supergraph: NodeSuperGraph | None = None,
):
    """Create a Session to a remote Bench"""
    supervisor_client = await simulation._supervisor.connect(client)
    host_client = await host.connect(client)
    engines = _make_remote_engines(bench_id, client, supervisor_client, host_client)
    if supergraph is None:
        root_ptr = NodeReference(type=NodeType.BENCH, id=bench_id, ck=bench_id)
        supergraph = NodeSuperGraph(root_ptr)
    session = Session(
        _is_readonly=False,
        _default_scope=GraphScope(bench_id=str(bench_id)),
        _engines=engines,
        _supervisor=supervisor_client,
        _host=host_client,
        _oracle=oracle,
        _supergraph=supergraph,
    )
    return session


@asynccontextmanager
async def _package_session(session: Session, bench_id: UUID, live: bool):
    async with session:
        # get main package
        bench_ptr = NodeReference(type=NodeType.BENCH, id=bench_id, ck=bench_id)
        bench = await Bench.descendants(NodeType.BRANCH, NodeType.PACKAGE).get(bench_ptr)
        assert bench.main_branch is not None, f"{bench!r} has no main branch"
        assert bench.main_branch.main_package is not None, f"{bench!r} has no main package"
        main_package = bench.main_branch.main_package

        # get query
        pkg = await (
            Package.descendants(*SOURCE_NODE_TYPES)
            .ancestors(Bench, Branch)
            .select_all()
            .exclude(Bench.encryption_key)
            .get(main_package.to_ref(), live=live)
        )
        connection = pkg._connection
        assert connection is not None, f"{pkg!r} has no connection"

        yield session


@dataclass
class WriteBlockTreeSpec(WorkloadSpec):
    type: WorkloadType = WorkloadType.WRITE_BLOCK_TREE
    bench: str = ""
    client: str = ""
    block_types: tuple[BlockType, ...] = (BlockType.PAGE, BlockType.TEXT)
    edit_types: tuple[EditType, ...] = EDIT_TYPES.tuple
    transactions: int | SampledInt | None = None
    edits_per_transaction: int | SampledInt = 10


@workload(WorkloadType.WRITE_BLOCK_TREE, WriteBlockTreeSpec)
class WriteBlockTreeWorkload(WorkloadBase[WriteBlockTreeSpec]):
    async def _do_run(self):
        # prepare
        bench_id = self.simulation.resolve_bench_id(self.spec.bench)
        client = self.simulation.get_client(self.spec.client)
        host = self.simulation.get_host(self.spec.bench)
        session = await _make_remote_session(bench_id, client, host, self.oracle, self.simulation)
        max_transactions = to_value_maybe(self.random, self.spec.transactions)

        # run
        async with _package_session(session, bench_id, live=True):
            n_transactions = 0
            while max_transactions is None or n_transactions < max_transactions:
                n_transactions += 1
                n_edits = to_value(self.random, self.spec.edits_per_transaction)

                ...  # nocheckin


@dataclass
class ReadPackageSpec(WorkloadSpec):
    type: WorkloadType = WorkloadType.READ_PACKAGE
    bench: str = ""
    client: str = ""
    live: bool = True


@workload(WorkloadType.READ_PACKAGE, ReadPackageSpec)
class ReadPackageWorkload(WorkloadBase[ReadPackageSpec]):
    async def _do_run(self):
        # prepare
        bench_id = self.simulation.resolve_bench_id(self.spec.bench)
        client = self.simulation.get_client(self.spec.client)
        host = self.simulation.get_host(self.spec.bench)
        session = await _make_remote_session(bench_id, client, host, self.oracle, self.simulation)

        # run
        async with _package_session(session, bench_id, self.spec.live):
            ...  # nocheckin
