import abc
import asyncio
from contextlib import asynccontextmanager
from dataclasses import dataclass
from itertools import chain
from typing import Any, ClassVar, Collection, Generator, Iterable, final, override
from uuid import UUID

import bitarray
import structlog
from opentelemetry import trace

from bench.language import Bench, Node, NodeType, Store
from bench.language.bench import Branch, Package, Region
from bench.language.connection import PostgresEngine
from bench.language.const import GLOBAL_NODE_TYPES, VERSION, EditType
from bench.language.graph import NodeGraphLike
from bench.language.session import Session
from bench.language.transaction import unpack_node_delta
from bench.proto import wiring
from bench.proto.wire import EditData, GraphScope
from bench.sql.client import GLOBAL_PG_CRYPTO_KEY, PgStoreConnection
from bench.utils.func import bittuple
from bench.utils.task import TaskManager
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

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
    store=GLOBAL_STORE,
    bench=SYSTEM_BENCH_STUB,
    scope=GraphScope(),
    node_types=GLOBAL_NODE_TYPES,
)


LOADED_BENCH_NODE_TYPES: bittuple[NodeType] = bittuple(
    NodeType.BENCH,
    NodeType.HANDLE,
    NodeType.SERVER,
    NodeType.CLIENT,
    NodeType.MACHINE,
    NodeType.STORE,
    NodeType.DRIVE,
    NodeType.ENVIRONMENT,
    NodeType.BRANCH,
    NodeType.PACKAGE,
)
LOADED_PACKAGE_NODE_TYPES: bittuple[NodeType] = bittuple(
    NodeType.PACKAGE,
    NodeType.DEPENDENCY,
    NodeType.UPGRADE,
    NodeType.SPACE,
    NodeType.LINK,
    NodeType.NOTICE,
    NodeType.BLOCK,
    NodeType.TRIGGER,
    NodeType.FIELD,
    NodeType.QUERY,
    NodeType.VIEW,
    NodeType.STEP,
    NodeType.BADGE,
    NodeType.ROLE,
    NodeType.IDENTITY,
    NodeType.MEMBERSHIP,
    NodeType.INVITE,
)
LOADED_HOST_NODE_TYPES = LOADED_BENCH_NODE_TYPES | LOADED_PACKAGE_NODE_TYPES
BENCH_QUERY = Bench.descendants(*LOADED_BENCH_NODE_TYPES).select_all()
PACKAGE_QUERY = (
    Package.descendants(*LOADED_PACKAGE_NODE_TYPES)
    .ancestors(Bench, Branch)
    .select_all()
    .exclude(Bench.encryption_key)
)


def global_pg_cursor(autocommit: bool = False):
    return PgStoreConnection(GLOBAL_STORE, SYSTEM_BENCH_STUB, autocommit=autocommit)


def global_session():
    return Session(parent=None, _default_scope=GraphScope(), _engines=(GLOBAL_POSTGRES_ENGINE,))


@dataclass(slots=True)
class Commit[T: Node]:
    """
    A simplified diff of edited Nodes from a committed transaction for the Host system.
    In this view, archive/soft-delete => remove (and unarchive/restore => add).
    """

    edits: Collection[EditData]
    cascaded_edits: Collection[EditData]
    edited_types: bittuple[NodeType]
    added: tuple[T, ...]
    updated: tuple[T, ...]
    removed: tuple[T, ...]
    epoch: int

    def __str__(self):
        return f"added={self.added!r}, updated={self.updated!r}, removed={self.removed!r}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    @property
    def edited(self) -> Iterable[T]:
        return chain(self.added, self.updated, self.removed)

    def has(self, node_types: NodeType | tuple[NodeType, ...] | bittuple[NodeType]) -> bool:
        """Check if the diff contains any of the given node types."""
        if isinstance(node_types, NodeType):
            return node_types in self.edited_types
        elif isinstance(node_types, tuple):
            return any(t in self.edited_types for t in node_types)
        else:
            return (self.edited_types.bits & node_types.bits).any()

    def trim_to(
        self, node_types: NodeType | tuple[NodeType, ...] | bittuple[NodeType]
    ) -> "Commit[T]":
        """Trims the diff to only include the given node types."""
        if isinstance(node_types, NodeType):
            node_types = bittuple(node_types)
        elif isinstance(node_types, tuple):
            node_types = bittuple(*node_types)
        return Commit(
            edits=[e for e in self.edits if NodeType(e.node_ptr.type) in node_types],
            cascaded_edits=[
                e for e in self.cascaded_edits if NodeType(e.node_ptr.type) in node_types
            ],
            edited_types=self.edited_types & node_types,
            added=tuple(node for node in self.added if node.metatype in node_types),
            updated=tuple(node for node in self.updated if node.metatype in node_types),
            removed=tuple(node for node in self.removed if node.metatype in node_types),
            epoch=self.epoch,
        )


def unpack_commit(
    session: Session,
    graphs: Collection[NodeGraphLike],
    edits: list[EditData],
    cascaded_edits: list[EditData],
    epoch: int,
) -> Commit:
    """
    Get the summarized, unpacked nodes that change in the given edits.
    Successive edits cancel each other out (create X -> delete X, no X in the change).
    Archived/soft-deleted nodes are treated as removed.
    The first graph containing the node is used.
    """

    edited_types = bitarray.bitarray(NodeType.get_max_ord())
    added: dict[UUID, Node] = {}
    updated: dict[UUID, Node] = {}
    removed: dict[UUID, Node] = {}

    def _add_edit(edit: EditData, node: Node):
        if edit.type in (EditType.CREATE, EditType.UNARCHIVE, EditType.RESTORE):
            # TODO :Broken: not sure how to handle upsert here yet (just error for now)
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
            raise RuntimeError(f"unexpected edit type {edit.type} in {edit!r}")

    # the nodes edited in 'edits' are expected to be in 'graph'
    for edit in edits:
        node_type = NodeType(edit.node_ptr.type)
        edited_types[node_type.ord] = True
        # unpack
        node_id = UUID(edit.node_ptr.id)
        node = None
        for graph in graphs:
            if node_id in graph:
                node = graph.get(node_id)
                break
        if node is None:
            if node_type == NodeType.LOG:
                # access logs which are just created for each edit
                continue
            raise RuntimeError(f"missing node {node_id!r} in {graphs!r} for {edit!r}")
        # map
        _add_edit(edit, node)

    # any cascaded edits are expected to be full trusted nodes (from archive/unarchive/...)
    unpacked_nodes: dict[UUID, Node] = {}
    for edit in cascaded_edits:
        node_type = NodeType(edit.node_ptr.type)
        edited_types[node_type.ord] = True
        # unpack
        assert edit.new_node_packed, f"missing node data for {edit!r}"
        node = unpack_node_delta(edit.new_node_packed, node_type=node_type)
        assert node.parent_ptr, f"missing parent ptr for {node!r} in {edit!r}"
        parent_id = UUID(node.parent_ptr.id)
        for graph in graphs:
            if parent_id in graph:
                parent = graph.get(parent_id)
                break
        else:
            if parent_id in unpacked_nodes:
                parent = unpacked_nodes[parent_id]
            else:
                # cascaded edits should bei in pre-order, so the parent must exist
                raise RuntimeError(f"missing parent {parent_id} for {node!r} in {edit!r}")
        node = wiring.unpack_node(node, parent, session)
        unpacked_nodes[node.id] = node
        # map
        _add_edit(edit, node)

    commit = Commit(
        edits=edits,
        cascaded_edits=cascaded_edits,
        edited_types=bittuple.from_ord(NodeType, edited_types),
        added=tuple(added.values()),
        updated=tuple(updated.values()),
        removed=tuple(removed.values()),
        epoch=epoch,
    )
    return commit


class HostSpec(abc.ABC):
    """Base interface for the Host so we can pass it around more easily (and stub it)."""

    @abc.abstractmethod
    def on_error(self, source: "HostPlugin", error: Exception) -> None:
        """Handle a fatal error."""
        ...

    @abc.abstractmethod
    @asynccontextmanager
    async def session(
        self, *, readonly: bool = False, autocommit: bool = False
    ) -> Generator[Session, None, None]:
        """Gets the Session for short-lived, *exclusive access."""
        ...


class MockHost(HostSpec):
    def __init__(self, session: Session):
        self._session = session

    def on_error(self, source: Any, error: Exception) -> None:
        pass

    @asynccontextmanager
    async def session(self, *, readonly: bool = False, autocommit: bool = False):
        yield self._session


class HostPlugin[T: Node](abc.ABC):
    """A plugin into the Host operating system of a Bench."""

    """The type of nodes to subscribe to for edits."""
    watch_types: ClassVar[bittuple[NodeType]]

    def __init__(self, host: HostSpec, bench: "Bench"):
        self._host = host
        self._bench = bench
        self._tasks = TaskManager(
            owner=self,
            logger=logger,
            on_error=lambda e: host.on_error(source=self, error=e),
            task_id_prefix=f"{self._bench.slug}_{self.__class__.__name__}",
        )

    def __str__(self) -> str:
        return ""

    def __repr__(self) -> str:
        content_str = str(self)
        if content_str:
            return f"<{self.__class__.__name__} {content_str} in '{self._bench.slug}'>"
        else:
            return f"<{self.__class__.__name__} in '{self._bench.slug}'>"

    @property
    def name(self) -> str:
        return self.__class__.__name__

    #
    # Lifecycle
    #

    async def start(self) -> None:  # noqa: B027
        """Start any work for this plugin, returning when the plugin is ready."""
        pass

    def close(self) -> None:
        """Close any stuff you need to close (if any)."""
        self._tasks.close()

    async def wait_closed(self) -> None:
        """After closing, wait for any stuff you need to wait for (if any)."""
        await self._tasks.wait_closed()

    async def wait_idle(self, timeout: float) -> None:  # noqa: B027
        """Wait for any pending events to finish processing."""
        pass

    #
    # Events
    #

    async def extend_commit(self, session: Session, commit: Commit[T]) -> None:  # noqa: B027
        """
        Add edits that logically belong to the same transaction.
        The nodes are the partial nodes from the edit graph, not the full Host nodes.
        Edit nodes directly, flush only when necessary.
        """
        pass

    async def on_commit(self, session: Session, commit: Commit[T]) -> None:  # noqa: B027
        """
        React to the commit in a new transaction (but still in the request lifecycle).
        The nodes are the fully loaded nodes from the Host.
        Edit nodes directly, flush or commit as necessary.
        """
        pass


class DeferredHostPlugin[T: Node](HostPlugin, abc.ABC):
    """A Host plugin with async event handlers."""

    def __init__(self, host: HostSpec, bench: "Bench"):
        super().__init__(host, bench)
        self._commit_queue: asyncio.Queue[Commit[T]] = asyncio.Queue()

    @override
    async def start(self) -> None:
        await super().start()
        self._tasks.start_queue(self._commit_queue, self.on_commit_deferred, skip_errors=True)

    @override
    @final
    async def on_commit(self, session: Session, commit: Commit) -> None:
        self._commit_queue.put_nowait(commit)
        await self._on_commit(session, commit)

    async def _on_commit(self, session: Session, commit: Commit[T]) -> None:
        pass

    @final
    async def wait_idle(self, timeout: float) -> None:
        if self._commit_queue.empty():
            return  # NOTE :Robustness: not sure why we need this early exit, otherwise we stall
        try:
            await asyncio.wait_for(self._commit_queue.join(), timeout=timeout)
        except asyncio.TimeoutError as e:
            raise RuntimeError(
                f"{self!r} timed out after {timeout}s waiting for {self._commit_queue.qsize()} commits"
            ) from e
        self._tasks.check_no_errors()

    async def on_commit_deferred(self, commit: Commit) -> None:
        """
        React to the committed changes (outside the request, later).
        NOTE :Robustness: the nodes in each commit may change before this is called
        """
        pass
