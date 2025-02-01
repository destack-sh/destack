from dataclasses import dataclass
from typing import final, override
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language import (
    SOURCE_NODE_TYPES,
    Bench,
    Block,
    BlockType,
    EditType,
    Log,
    NodeSuperGraph,
    NodeType,
    Package,
    Session,
    repr_enums,
)
from bench.test.simulation.core import (
    ClientHandle,
    HostHandle,
    SampledFloat,
    SampledInt,
    Simulation,
    make_remote_session,
    to_value,
)
from bench.test.utils import assert_graph_equals
from bench.utils.oracle import Oracle

from .spec import WorkloadSpec, WorkloadType
from .workload import Workload, workload_

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass
class ClientWorkloadSpec(WorkloadSpec):
    client: str = ""
    bench: str = ""


class ClientWorkload[SpecT: ClientWorkloadSpec](Workload[SpecT]):
    """Workloads that run on a single Client."""

    @override
    @final
    async def prepare(self):
        self.bench_id: UUID = self.simulation.get_bench_id(self.spec.bench)
        self.client: ClientHandle = self.simulation.get_client(self.spec.client)
        self.host: HostHandle = self.simulation.get_host(self.spec.bench)
        self.session: Session = await make_remote_session(
            simulation=self.simulation,
            bench_id=self.bench_id,
            client=self.client,
            host=self.host,
            oracle=self.oracle,
        )
        self.supergraph: NodeSuperGraph = self.session._supergraph

        await self.session.open(_set_in_context=False)
        async with self.session.active(readonly=False):
            self.bench = await Bench.get(id=self.bench_id, live=True)
            self.main_package = await (
                Package.include_descendants(*SOURCE_NODE_TYPES)
                .include_ancestors(Bench)
                .select_all()
                .deselect(Bench.encryption_key)
                .get(self.bench.main_package_ptr, live=True)
            )
            await self.prepare_in_session(self.session)
            await self.session.commit()

    async def prepare_in_session(self, session: Session):
        """Prepare the workload in the given session."""
        pass  # do nothing by default

    @override
    @final
    async def run_once(self):
        async with self.session.active(readonly=False):
            await self.run_once_in_session(self.session)
            await self.session.commit()

    async def run_once_in_session(self, session: Session):
        """Runs one repetition of the workload in the given session."""
        pass


@dataclass
class WriteBlockTreeSpec(ClientWorkloadSpec):
    type: WorkloadType = WorkloadType.WRITE_BLOCK_TREE
    block_types: tuple[BlockType, ...] = (BlockType.PAGE, BlockType.TEXT)
    edit_types: tuple[EditType, ...] = (EditType.CREATE, EditType.DELETE)
    transactions: int | SampledInt = 1
    transactions_interval: float | SampledFloat = 0.0
    edits_per_transaction: int | SampledInt = 10
    live: bool = True


@workload_(WorkloadType.WRITE_BLOCK_TREE, WriteBlockTreeSpec)
class WriteBlockTreeWorkload(ClientWorkload[WriteBlockTreeSpec]):
    """Write a random tree of blocks."""

    def __init__(self, spec: WriteBlockTreeSpec, oracle: Oracle, simulation: Simulation):
        super().__init__(spec, oracle, simulation)
        self.block_num = 0
        self.all_edits: list[tuple[EditType, Block]] = []

    def __str__(self):
        return f"node_subtypes={repr_enums(self.spec.block_types)}, edit_types={repr_enums(self.spec.edit_types)}"

    @override
    async def run_once_in_session(self, session: Session):
        max_transactions = to_value(self.random, self.spec.transactions)
        n_transactions = 0
        while n_transactions < max_transactions:
            # make random edits
            max_edits = to_value(self.random, self.spec.edits_per_transaction)
            for _ in range(max_edits):
                edit_type = self.random.choice(self.spec.edit_types)
                blocks = self.main_package._graph.nodes_of_type(Block)
                if edit_type == EditType.CREATE:
                    block_type = self.random.choice(self.spec.block_types)
                    parent = self.random.choice((self.main_package, *blocks))
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
                self.main_package._graph,
                host.service.main_package._graph,
                ignore_node_types=(NodeType.BENCH,),  # not in host package graph
            )

            # and wait for next tx
            wait = to_value(self.random, self.spec.transactions_interval)
            await self.oracle.sleep(wait)


@dataclass
class ReadPackageSpec(ClientWorkloadSpec):
    type: WorkloadType = WorkloadType.READ_PACKAGE
    live: bool = True


@workload_(WorkloadType.READ_PACKAGE, ReadPackageSpec)
class ReadPackageWorkload(ClientWorkload[ReadPackageSpec]):
    """Reads an entire package."""

    def __str__(self):
        return f"pkg={self.main_package!r}"

    @override
    async def check_group(self, group: list[Workload]):
        # check that all packages are the same
        for other in group:
            if other is self or not isinstance(other, ClientWorkload):
                continue
            assert_graph_equals(self.main_package._graph, other.main_package._graph)


@dataclass
class WatchLogsSpec(ClientWorkloadSpec):
    type: WorkloadType = WorkloadType.WATCH_LOGS
    tail_user: str | None = None
    limit: int | SampledInt = 50
    live: bool = True
    min_expected_count: int | None = None


@workload_(WorkloadType.WATCH_LOGS, WatchLogsSpec)
class WatchLogsWorkload(ClientWorkload[WatchLogsSpec]):
    @override
    async def prepare_in_session(self, session: Session):
        self.limit = to_value(self.random, self.spec.limit)
        log_query = Log.order_by("-created_at").first(self.limit)
        if self.spec.tail_user:
            tail_user = self.simulation.get_user(self.spec.tail_user)
            log_query = log_query.where(user=tail_user.user_ptr)
        _, self.connection = await log_query.search_connection(live=True)

    @property
    def logs(self) -> list[Log]:
        return self.connection.result.roots

    @override
    async def check_self(self):
        logs = self.connection.result.roots
        if self.spec.min_expected_count is not None:
            assert (
                len(logs) >= self.spec.min_expected_count
            ), f"too few logs {len(logs)} < {self.spec.min_expected_count}"
        assert len(logs) <= self.limit, f"too many logs {len(logs)} > {self.limit}"
        assert len(self.connection.result.graph.nodes) == len(logs)

    @override
    async def check_group(self, group: list[Workload]):
        for other in group:
            if other is self or not isinstance(other, WatchLogsWorkload):
                continue
            assert len(self.logs) == len(other.logs), f"{self!r} != {other!r}"
            for log_a, log_b in zip(self.logs, other.logs):
                assert log_a == log_b, f"{log_a!r} != {log_b!r}"
                assert log_a.equals(log_b), f"{log_a!r} != {log_b!r}"
