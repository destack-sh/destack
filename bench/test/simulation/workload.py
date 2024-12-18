import abc
from dataclasses import dataclass
from typing import TYPE_CHECKING, final, override
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language import NodeReference
from bench.language.bench import Bench, Client, Package
from bench.language.block import Block
from bench.language.connection import Engine, RemoteEngine
from bench.language.const import (
    BENCH_NODE_TYPES,
    PUBLIC_NODE_TYPES,
    SOURCE_NODE_TYPES,
    BlockType,
    EditType,
    NodeType,
)
from bench.language.graph import NodeSuperGraph
from bench.language.log import Log
from bench.language.node import EMPTY_SCOPE_DATA, GraphScope
from bench.language.session import Session
from bench.language.user import User
from bench.proto.wire import HostClient, RpcMetadata, SupervisorClient
from bench.proto.wiring import unpack_builtin_object
from bench.test.simulation.spec import WorkloadSpec, WorkloadType
from bench.test.simulation.utils import SampledFloat, SampledInt, to_value
from bench.test.utils import assert_graph_equals
from bench.utils.func import repr_enums
from bench.utils.oracle import Oracle
from bench.utils.string import Casing, to_casing
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
        logger = structlog.get_logger(to_casing(self.__class__.__name__, Casing.SNAKE))
        self.log = logger.bind(workload=self)
        self._started_at_ns: int | None = None
        self._terminated_at_ns: int | None = None
        self._do_init()

    def _do_init(self):  # noqa: B027
        """Initializes the workload state."""
        pass

    def __str__(self) -> str:
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

    @final
    @tracer.start_as_current_span("workload.prepare")
    async def prepare(self):
        """Prepare the workload before running it."""
        await self._do_prepare()
        self.log.debug("workload.prepare", span="current")

    async def _do_prepare(self):  # noqa: B027
        """Prepares the workload before running it."""
        pass

    @final
    async def run(self):
        """Runs the full workload until some termination condition is met."""
        self._started_at_ns = self.oracle.time_ns()
        try:
            n_runs = 1
            repeat = to_value(self.random, self.spec.repeat)
            repeat_interval = to_value(self.random, self.spec.repeat_interval)
            while n_runs <= repeat:
                with tracer.start_as_current_span(f"workload.{self.name}"):
                    await self._do_run_once()
                    self.log.info("workload.run", run=n_runs, span="current")
                    await self.oracle.sleep(repeat_interval)
                n_runs += 1
        finally:
            self._terminated_at_ns = self.oracle.time_ns()

    @abc.abstractmethod
    async def _do_run_once(self):
        """Runs one repetition of the workload."""
        raise NotImplementedError

    @final
    @tracer.start_as_current_span("workload.check")
    async def check(self):
        """Validate any post-run conditions."""
        await self._do_check()
        if self.spec.group:
            group = self.simulation.get_workload_group(self.spec.group)
            # NOTE :Performance :Test: check in-group pairings only as needed
            await self._do_check_group(group)
        self.log.debug("workload.check", span="current")

    async def _do_check(self):  # noqa: B027
        """Validates any post-run conditions."""
        pass

    async def _do_check_group(self, group: list["WorkloadBase"]):  # noqa: B027
        """Validates any post-run conditions for a group of workloads. Called for every workload in the group."""
        pass


def _make_remote_engines(
    bench_id: UUID,
    rpc_metadata: RpcMetadata,
    supervisor_client: SupervisorClient,
    host_client: HostClient,
) -> tuple[Engine, ...]:
    """Gets the graph engines to connect with a remote Bench"""
    engines = (
        # global engine
        RemoteEngine(
            name="remote-global",
            scope=EMPTY_SCOPE_DATA,
            node_types=PUBLIC_NODE_TYPES,
            remote=supervisor_client,
            write_retry=RETRY_GRPC_FOREVER,
            rpc_metadata=rpc_metadata,
        ),
        # bench engine
        RemoteEngine(
            name="remote-bench",
            scope=GraphScope(bench_id=bench_id)._to_data(),
            node_types=BENCH_NODE_TYPES,
            remote=host_client,
            write_retry=RETRY_GRPC_FOREVER,
            rpc_metadata=rpc_metadata,
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
    nonce = str(UUID(int=oracle.random.getrandbits(128)))
    supervisor_client = await simulation._supervisor.connect(client)
    host_client = await host.connect(client)
    engines = _make_remote_engines(bench_id, client.rpc_metadata, supervisor_client, host_client)
    if supergraph is None:
        root_ptr = NodeReference(node_type=NodeType.BENCH, id=bench_id, ck=bench_id)
        supergraph = NodeSuperGraph(root_ptr)
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
    session.user = unpack_builtin_object(
        client.user.user_data,
        session=session,
        supergraph=supergraph,
        expect=User,
        skip_add_self=False,
    )
    session.client = unpack_builtin_object(
        client.client_data,
        session=session,
        supergraph=supergraph,
        expect=Client,
        skip_add_self=False,
    )
    session._subject = session.user
    return session


async def get_package(bench_id: UUID, session: Session, *, live: bool):
    """Gets the entire main package source"""
    # resolve package pointer
    bench_ptr = NodeReference(node_type=NodeType.BENCH, id=bench_id, ck=bench_id)
    bench = await Bench.include_descendants(NodeType.PACKAGE).get(bench_ptr)
    assert bench.main_package is not None, f"{bench!r} has no main package"
    pkg_stub = bench.main_package

    # get package source
    pkg = await (
        Package.include_descendants(*SOURCE_NODE_TYPES)
        .include_ancestors(Bench)
        .select_all()
        .deselect(Bench.encryption_key)
        .get(pkg_stub.to_ref(), live=live)
    )
    return pkg


@dataclass
class SingleClientWorkloadSpec(WorkloadSpec):
    client: str = ""
    bench: str = ""


class SingleClientWorkloadBase[SpecT: SingleClientWorkloadSpec](WorkloadBase[SpecT]):
    """Base class for workloads with a single client."""

    @override
    @final
    async def _do_prepare(self):
        self.bench_id = self.simulation.resolve_bench_id(self.spec.bench)
        self.client = self.simulation.get_client(self.spec.client)
        self.host = self.simulation.get_host(self.spec.bench)
        self.session = await make_remote_session(
            self.bench_id, self.client, self.host, self.oracle, self.simulation
        )
        await self.session.open(set_in_context=False)
        async with self.session.active(readonly=False):
            await self._do_prepare_in_session(self.session)
            await self.session.commit()

    async def _do_prepare_in_session(self, session: Session):
        """Prepare the workload in the given session."""
        pass  # do nothing by default

    @override
    @final
    async def _do_run_once(self):
        async with self.session.active(readonly=False):
            await self._do_run_once_in_session(self.session)
            await self.session.commit()

    @abc.abstractmethod
    async def _do_run_once_in_session(self, session: Session):
        """Runs one repetition of the workload in the given session."""
        raise NotImplementedError


@dataclass
class WriteBlockTreeSpec(SingleClientWorkloadSpec):
    type: WorkloadType = WorkloadType.WRITE_BLOCK_TREE
    block_types: tuple[BlockType, ...] = (BlockType.PAGE, BlockType.ACTION)
    edit_types: tuple[EditType, ...] = (EditType.CREATE, EditType.DELETE)
    transactions: int | SampledInt = 1
    transactions_interval: float | SampledFloat = 0.0
    edits_per_transaction: int | SampledInt = 10
    live: bool = True


@workload(WorkloadType.WRITE_BLOCK_TREE, WriteBlockTreeSpec)
class WriteBlockTreeWorkload(SingleClientWorkloadBase[WriteBlockTreeSpec]):
    """Write a random tree of blocks."""

    def __str__(self):
        return f"pkg={self.pkg!r}, node_subtypes={repr_enums(self.spec.block_types)}, edit_types={repr_enums(self.spec.edit_types)}"

    @override
    def _do_init(self):
        self.pkg: Package | None = None
        self.block_num = 0
        self.all_edits: list[tuple[EditType, Block]] = []

    @override
    async def _do_prepare_in_session(self, session: Session):
        self.pkg = await get_package(self.bench_id, session, live=self.spec.live)

    @override
    async def _do_run_once_in_session(self, session: Session):
        assert self.pkg is not None, f"{self!r} not ready"
        max_transactions = to_value(self.random, self.spec.transactions)
        n_transactions = 0
        while n_transactions < max_transactions:
            # make random edits
            max_edits = to_value(self.random, self.spec.edits_per_transaction)
            for _ in range(max_edits):
                edit_type = self.random.choice(self.spec.edit_types)
                blocks = self.pkg._graph.nodes_of_type(Block)
                if edit_type == EditType.CREATE:
                    block_type = self.random.choice(self.spec.block_types)
                    parent = self.random.choice((self.pkg, *blocks))
                    block = Block.new(block_type, name=f"{block_type.bench_name}{self.block_num}")
                    self.block_num += 1
                    parent.blocks.append(block)
                elif edit_type == EditType.DELETE:
                    if not blocks:
                        continue  # no blocks to delete yet
                    block = self.random.choice(blocks)
                    block.delete()
                else:
                    raise NotImplementedError(f"unexpected edit type {edit_type}")
                self.all_edits.append((edit_type, block))
            await self.session.commit()
            n_transactions += 1

            # check we're in sync with host
            host = self.simulation.get_host(self.spec.bench)
            assert_graph_equals(
                self.pkg._graph,
                host.service.main_package._graph,
                ignore_node_types=(NodeType.BENCH,),  # not in host package graph
            )

            # and wait for next tx
            wait = to_value(self.random, self.spec.transactions_interval)
            await self.oracle.sleep(wait)


@dataclass
class ReadPackageSpec(SingleClientWorkloadSpec):
    type: WorkloadType = WorkloadType.READ_PACKAGE
    live: bool = True


@workload(WorkloadType.READ_PACKAGE, ReadPackageSpec)
class ReadPackageWorkload(SingleClientWorkloadBase[ReadPackageSpec]):
    """Reads an entire package."""

    def __str__(self):
        return f"pkg={self.pkg!r}"

    @override
    def _do_init(self):
        self.pkg: Package | None = None

    @override
    async def _do_prepare_in_session(self, session: Session):
        self.pkg = await get_package(self.bench_id, session, live=self.spec.live)

    @override
    async def _do_run_once_in_session(self, session: Session):
        pass  # nothing to do?

    @override
    async def _do_check_group(self, group: list[WorkloadBase]):
        # check that all packages are the same
        assert self.pkg is not None, f"{self!r} not ready"
        for other in group:
            if other is self:
                continue
            pkg = getattr(other, "pkg", None)
            assert isinstance(pkg, Package), f"{other!r} has no package"
            assert_graph_equals(self.pkg._graph, pkg._graph)


@dataclass
class WatchLogsSpec(SingleClientWorkloadSpec):
    type: WorkloadType = WorkloadType.WATCH_LOGS
    tail_user: str | None = None
    limit: int | SampledInt = 50
    live: bool = True
    min_expected_count: int | None = None


@workload(WorkloadType.WATCH_LOGS, WatchLogsSpec)
class WatchLogsWorkload(SingleClientWorkloadBase[WatchLogsSpec]):
    @override
    async def _do_prepare_in_session(self, session: Session):
        self.limit = to_value(self.random, self.spec.limit)
        log_query = Log.order_by("-created_at").first(self.limit)
        if self.spec.tail_user:
            tail_user = self.simulation.get_user(self.spec.tail_user)
            log_query = log_query.where(user=tail_user.user_ptr)
        self.connection = await log_query.search_live()

    @property
    def logs(self) -> list[Log]:
        return self.connection.result.roots

    @override
    async def _do_run_once_in_session(self, session: Session):
        pass  # nothing to do?

    @override
    async def _do_check(self):
        logs = self.connection.result.roots
        if self.spec.min_expected_count is not None:
            assert (
                len(logs) >= self.spec.min_expected_count
            ), f"too few logs {len(logs)} < {self.spec.min_expected_count}"
        assert len(logs) <= self.limit, f"too many logs {len(logs)} > {self.limit}"
        assert len(self.connection.result.graph.nodes) == len(logs)

    @override
    async def _do_check_group(self, group: list[WorkloadBase]):
        for other in group:
            if other is self or not isinstance(other, WatchLogsWorkload):
                continue
            assert len(self.logs) == len(other.logs), f"{self!r} != {other!r}"
            for log_a, log_b in zip(self.logs, other.logs):
                assert log_a == log_b, f"{log_a!r} != {log_b!r}"
                assert log_a.equals(log_b), f"{log_a!r} != {log_b!r}"
