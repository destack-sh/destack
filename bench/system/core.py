import abc
import asyncio
from contextlib import asynccontextmanager
from dataclasses import dataclass
from typing import ClassVar, override
from uuid import UUID

import bitarray
import structlog

from bench.language import Bench, Node, NodeType, Store
from bench.language.bench import Region
from bench.language.connection import PostgresEngine, StoreEngine
from bench.language.const import GLOBAL_NODE_TYPES, VERSION, EditType
from bench.language.graph import NodeGraph
from bench.language.session import CommitHook, Session
from bench.proto import wire, wiring
from bench.proto.wire import EditData, GraphScope
from bench.sql.client import GLOBAL_PG_CRYPTO_KEY, _PgStoreConnection
from bench.utils.func import bittuple
from bench.utils.task import TaskManager
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)

GLOBAL_PG_HOST = get_from_env("GLOBAL_PG_HOST")
GLOBAL_PG_NAME = get_from_env("GLOBAL_PG_NAME")
GLOBAL_PG_USERNAME = get_from_env("GLOBAL_PG_USERNAME")
GLOBAL_PG_PASSWORD = get_from_env("GLOBAL_PG_PASSWORD")

SYSTEM_BENCH_STUB = Bench(
    name="System (Stub)", slug="system", region=Region.GLOBAL, encryption_key=GLOBAL_PG_CRYPTO_KEY
)

GLOBAL_STORE = Store(
    parent=SYSTEM_BENCH_STUB,
    name="Global Store",
    version=VERSION,
    external_name=GLOBAL_PG_NAME,
    connection_uri=f"postgresql://{GLOBAL_PG_USERNAME}:{GLOBAL_PG_PASSWORD}@{GLOBAL_PG_HOST}/{GLOBAL_PG_NAME}",
)
GLOBAL_POSTGRES_ENGINE = PostgresEngine(
    store=GLOBAL_STORE, bench=SYSTEM_BENCH_STUB, scope=GraphScope(), node_types=GLOBAL_NODE_TYPES
)


@asynccontextmanager
async def global_pg_cursor(autocommit: bool = False):
    async with _PgStoreConnection(GLOBAL_STORE, SYSTEM_BENCH_STUB, autocommit=autocommit) as cur:
        yield cur


@asynccontextmanager
async def global_session(on_commit_hook: CommitHook | None = None):
    async with Session(
        parent=None,
        _default_scope=GraphScope(),
        _engines=(GLOBAL_POSTGRES_ENGINE,),
        _on_commit_hook=on_commit_hook,
    ) as session:
        yield session


@asynccontextmanager
async def local_session(scope: GraphScope, engines: tuple[StoreEngine, ...]):
    """Session for local operations (no remote calls)."""
    session = Session(_default_scope=scope, _engines=engines)
    async with session:
        yield session


@dataclass(slots=True)
class CommittedChange[T: Node]:
    """A simplified diff of edited Nodes from a commit. Here archive/soft-delete => remove."""

    edits: list[EditData]
    cascaded_edits: list[EditData]
    edited_types: bittuple[NodeType]
    added: list[T]
    updated: list[T]
    removed: list[T]

    def __str__(self):
        return f"added={self.added!r}, updated={self.updated!r}, removed={self.removed!r}"

    def __repr__(self):
        return f"<GraphDiff {self!s}>"

    def trim_to(self, node_types: bittuple[NodeType]) -> "CommittedChange[T]":
        """Trims the diff to only include the given node types."""
        return CommittedChange(
            edits=self.edits,
            cascaded_edits=self.cascaded_edits,
            edited_types=self.edited_types & node_types,
            added=[node for node in self.added if node.metatype in node_types],
            updated=[node for node in self.updated if node.metatype in node_types],
            removed=[node for node in self.removed if node.metatype in node_types],
        )


def unpack_committed_change(
    graph: NodeGraph[Node], edits: list[EditData], cascaded_edits: list[EditData]
) -> CommittedChange:
    """
    Get the summarized, unpacked nodes that change in the given edits.
    Successive edits cancel each other out (create X -> delete X, no X in the change).
    Archived/soft-deleted nodes are treated as removed.
    """

    edited_types = bitarray.bitarray(NodeType.get_max_ord())
    added: dict[UUID, Node] = {}
    updated: dict[UUID, Node] = {}
    removed: dict[UUID, Node] = {}

    def _add_edit(edit: EditData, node: Node):
        if edit.type in (EditType.CREATE, EditType.UNARCHIVE, EditType.RESTORE):
            # NOTE :Broken: not sure how to handle upsert here yet (just error for now)
            added[node.id] = node
            if node.id in removed:
                del removed[node.id]
        elif edit.type in (EditType.MOVE, EditType.UPDATE):
            if node.id not in added:
                updated[node.id] = node
        elif edit.type in (EditType.ARCHIVE, EditType.SOFT_DELETE, EditType.DELETE):
            if node.id in added:
                del added[node.id]
            removed[node.id] = node
        else:
            raise RuntimeError(f"unexpected edit type {edit.type} in {edit!r} for {graph!r}")

    # the nodes edited in 'edits' are expected to be in 'graph'
    for edit in edits:
        node_type = NodeType(edit.node_type)
        edited_types[node_type.ord] = True
        # unpack
        node = wiring.unwrap_some_node(edit.node)
        node = graph.get(UUID(node.id))
        assert node is not None, f"missing node {node!r} in {graph!r} for {edit!r}"
        # map
        _add_edit(edit, node)

    # any cascaded edits are expected to be full trusted nodes (from archive/unarchive/...)
    unpacked_nodes: dict[UUID, Node] = {}
    for edit in cascaded_edits:
        node_type = NodeType(edit.node_type)
        edited_types[node_type.ord] = True
        # unpack
        node = wiring.unwrap_some_node(edit.node)
        assert node.parent_ptr, f"missing parent ptr for {node!r} in {edit!r}"
        parent_id = UUID(node.parent_ptr.id)
        if parent_id in graph:
            parent = graph.get(parent_id)
        elif parent_id in unpacked_nodes:
            parent = unpacked_nodes[parent_id]
        else:
            # cascaded edits should bei in pre-order, so the parent must exist
            raise RuntimeError(f"missing parent {parent_id} for {node!r} in {edit!r}")
        node = wiring.unpack_node(node, parent)
        unpacked_nodes[node.id] = node
        # map
        _add_edit(edit, node)

    commit = CommittedChange(
        edits=edits,
        cascaded_edits=cascaded_edits,
        edited_types=bittuple.from_ord(NodeType, edited_types),
        added=list(added.values()),
        updated=list(updated.values()),
        removed=list(removed.values()),
    )
    return commit


class HostSpec(abc.ABC):
    """Base interface for the Host so we can pass it around more easily (and stub it)."""

    def session(self, scope: GraphScope | None = None) -> Session:
        raise NotImplementedError


DEAD_HOST = HostSpec()


class HostPlugin[T: Node](abc.ABC):
    """A plugin on the Host system of a Bench."""

    node_types: ClassVar[bittuple[NodeType]]

    def __init__(self, host: HostSpec, bench: "Bench"):
        self._host = host
        self._bench = bench

    def __str__(self) -> str:
        return ""

    def __repr__(self) -> str:
        content_str = str(self)
        if content_str:
            return f"<{self.__class__.__name__} {content_str} in '{self._bench.slug}'>"
        else:
            return f"<{self.__class__.__name__} in '{self._bench.slug}'>"

    #
    # Lifecycle
    #

    async def start(self, tasks: TaskManager) -> None:
        """Start any work for this plugin, returning when the plugin is ready."""
        pass

    def close(self) -> None:
        """Close any stuff you need to close (if any)."""
        pass

    async def wait_closed(self) -> None:
        """After closing, wait for any stuff you need to wait for (if any)."""
        pass

    #
    # Events
    #

    def on_graph_commit(self, commit: CommittedChange[T]) -> None:
        """Synchronous event handler for a committed Host transaction"""
        pass


class AsyncHostPlugin[T: Node](HostPlugin, abc.ABC):
    """A plugin with async event handlers."""

    def __init__(self, host: HostSpec, bench: "Bench"):
        super().__init__(host, bench)
        self._commit_queue: asyncio.Queue[CommittedChange[T]] = asyncio.Queue()

    async def start(self, tasks: TaskManager) -> None:
        await super().start(tasks)
        tasks.start_queue(self._commit_queue, self.on_graph_commit_async)

    @override
    def on_graph_commit(self, commit: CommittedChange) -> None:
        self._commit_queue.put_nowait(commit)

    async def on_graph_commit_async(self, commit: CommittedChange) -> None:
        """
        Asynchronous event handler for a committed Host transaction.
        NOTE :Robustness: the nodes in each commit may change before this is called
        """
        pass
