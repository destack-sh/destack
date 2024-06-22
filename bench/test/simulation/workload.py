import abc
from dataclasses import dataclass
from typing import TYPE_CHECKING, final
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.bench import Bench, Branch, Package
from bench.language.block import Block
from bench.language.connection import GraphEngine, RemoteEngine
from bench.language.const import (
    BENCH_NODE_TYPES,
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
from bench.language.user import User
from bench.proto.wire import GraphScope, HostClient, SupervisorClient
from bench.proto.wiring import unpack_object
from bench.test.simulation.spec import WorkloadSpec, WorkloadType
from bench.test.simulation.utils import SampledFloat, SampledInt, to_value
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
        self._started_at_ns: int | None = None
        self._terminated_at_ns: int | None = None

    def __str__(self):
        return ""

    @final
    def __repr__(self):
        content_str = str(self)
        if self._started_at_ns is not None:
            if self._terminated_at_ns is not None:
                duration = (self._terminated_at_ns - self._started_at_ns) / 10**9
                runtime_ns = f"runtime={(duration):.3f}s"
            else:
                duration = (self.oracle.time_ns() - self._started_at_ns) / 10**9
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
        pass

    @final
    async def run(self):
        self._started_at_ns = self.oracle.time_ns()
        try:
            n_runs = 1
            repeat = to_value(self.random, self.spec.repeat)
            repeat_interval = to_value(self.random, self.spec.repeat_interval)
            while n_runs <= repeat:
                with tracer.start_as_current_span(f"workload.{self.name}"):
                    await self._do_run()
                    await self.oracle.sleep(repeat_interval)
                n_runs += 1
        finally:
            self._terminated_at_ns = self.oracle.time_ns()

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


async def make_remote_session(
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
        _origin=client.client_origin,
        _supervisor=supervisor_client,
        _host=host_client,
        _oracle=oracle,
        _supergraph=supergraph,
    )
    subject = unpack_object(
        client.user.user_data, session=session, supergraph=supergraph, expect=User
    )
    session._subject = subject
    return session


async def get_package(bench_id: UUID, session: Session, *, live: bool):
    """Gets the entire main package source"""
    # resolve package pointer
    bench_ptr = NodeReference(type=NodeType.BENCH, id=bench_id, ck=bench_id)
    bench = await Bench.descendants(NodeType.BRANCH, NodeType.PACKAGE).get(bench_ptr)
    assert bench.main_branch is not None, f"{bench!r} has no main branch"
    assert bench.main_branch.main_package is not None, f"{bench!r} has no main package"
    pkg_stub = bench.main_branch.main_package

    # get package source
    pkg = await (
        Package.descendants(*SOURCE_NODE_TYPES)
        .ancestors(Bench, Branch)
        .select_all()
        .exclude(Bench.encryption_key)
        .get(pkg_stub.to_ref(), live=live)
    )
    return pkg


@dataclass
class WriteBlockTreeSpec(WorkloadSpec):
    type: WorkloadType = WorkloadType.WRITE_BLOCK_TREE
    bench: str = ""
    client: str = ""
    block_types: tuple[BlockType, ...] = (BlockType.PAGE, BlockType.TEXT)
    edit_types: tuple[EditType, ...] = (EditType.CREATE, EditType.DELETE)
    transactions: int | SampledInt = 1
    transactions_interval: float | SampledFloat = 0.0
    edits_per_transaction: int | SampledInt = 10


@workload(WorkloadType.WRITE_BLOCK_TREE, WriteBlockTreeSpec)
class WriteBlockTreeWorkload(WorkloadBase[WriteBlockTreeSpec]):
    async def _do_run(self):
        # prepare
        bench_id = self.simulation.resolve_bench_id(self.spec.bench)
        client = self.simulation.get_client(self.spec.client)
        host = self.simulation.get_host(self.spec.bench)
        session = await make_remote_session(bench_id, client, host, self.oracle, self.simulation)
        max_transactions = to_value(self.random, self.spec.transactions)

        # run
        async with session:
            pkg = await get_package(bench_id, session, live=True)

            n_transactions = 0
            while n_transactions < max_transactions:
                max_edits = to_value(self.random, self.spec.edits_per_transaction)
                for _ in range(max_edits):
                    edit_type = self.random.choice(self.spec.edit_types)
                    if edit_type == EditType.CREATE:
                        block_type = self.random.choice(self.spec.block_types)
                        blocks = pkg._graph.nodes_of_type(Block)
                        parent = self.random.choice((pkg, *blocks))
                        num_blocks_of_type = len([b for b in blocks if b.type == block_type])
                        block = Block.new(
                            block_type, name=f"{block_type.bench_name}{num_blocks_of_type + 1}"
                        )
                        parent.blocks.append(block)
                    elif edit_type == EditType.DELETE:
                        blocks = pkg._graph.nodes_of_type(Block)
                        if not blocks:
                            continue  # no blocks to delete yet
                        block = self.random.choice(blocks)
                        block.delete()
                    else:
                        raise NotImplementedError(f"unexpected edit type {edit_type}")
                await session.commit()
                n_transactions += 1

                wait = to_value(self.random, self.spec.transactions_interval)
                await self.oracle.sleep(wait)


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
        session = await make_remote_session(bench_id, client, host, self.oracle, self.simulation)

        # run
        async with session:
            pkg = await get_package(bench_id, session, live=self.spec.live)
            ...  # nocheckin evaluate package after workload/simulation is complete
