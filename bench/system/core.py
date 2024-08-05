import abc
import asyncio
from contextlib import asynccontextmanager
from dataclasses import dataclass
from datetime import datetime
from itertools import chain
from typing import Any, ClassVar, Collection, Generator, Iterable, Optional, final, override
from uuid import UUID

import bitarray
import structlog
from opentelemetry import trace

from bench.language import Bench, Node, NodeReference, NodeType, Store
from bench.language.connection import GraphEngine, PostgresEngine
from bench.language.const import (
    GLOBAL_NODE_TYPES,
    LOCAL_NODE_TYPES,
    VERSION,
    EditType,
    Region,
)
from bench.language.graph import NodeGraphLike, NodeSuperGraph
from bench.language.node import EMPTY_SCOPE, GraphScope
from bench.language.session import Session
from bench.proto import wiring
from bench.proto.wire import EditData, GraphScopeData
from bench.sql.client import AsyncPostgresConnection
from bench.utils.func import bittuple
from bench.utils.oracle import Oracle
from bench.utils.task import TaskManager
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

BEGINNING_OF_TIME = datetime.fromisoformat("1970-01-01T00:00:00+00:00")


def system_store_from_env() -> Store:
    """Get the default global store configured in the environment"""
    host = get_from_env("GLOBAL_PG_HOST", description="Global Postgres host")
    name = get_from_env("GLOBAL_PG_NAME", description="Global Postgres database name")
    username = get_from_env("GLOBAL_PG_USERNAME", description="Global Postgres username")
    password = get_from_env("GLOBAL_PG_PASSWORD", description="Global Postgres password")
    encryption_key = get_from_env("GLOBAL_PG_CRYPTO_KEY", description="Global encryption key")

    system_bench_ptr = NodeReference(type=NodeType.BENCH, id=UUID(int=0), ck=UUID(int=0))
    supergraph = NodeSuperGraph(root_ptr=system_bench_ptr)
    system_bench_stub = Bench(
        id=UUID(int=0),
        name="System",
        slug="system",
        region=Region.ZURICH,
        encryption_key=encryption_key,
        _supergraph=supergraph,
        # set timestamps to avoid drawing from oracle (which we don't have here)
        # (also these are technically 'eternal' nodes anyway)
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    store = Store(
        parent=system_bench_stub,
        name="Global Store",
        version=VERSION,
        external_name=name,
        connection_uri=f"postgresql://{username}:{password}@{host}/{name}",
        _supergraph=supergraph,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    return store


def pg_engine_from_store(
    store: Store,
    *,
    scope: GraphScopeData = EMPTY_SCOPE._to_data(),  # noqa: B008
    node_types: bittuple[NodeType] = GLOBAL_NODE_TYPES,
):
    """Get the postgres engine for a store"""
    assert store.parent is not None, f"missing parent for {store!r}"
    return PostgresEngine(store=store, bench=store.parent, scope=scope, node_types=node_types)


def local_pg_engine_from_store(store: Store):
    """Get the postgres engine for a local store"""
    assert store.bench is not None, f"missing bench for {store!r}"
    return pg_engine_from_store(
        store, scope=GraphScope(bench_id=store.bench.id)._to_data(), node_types=LOCAL_NODE_TYPES
    )


def global_session(
    store: Store,
    engines: tuple[GraphEngine, ...],
    oracle: Oracle,
    *,
    supergraph: NodeSuperGraph | None = None,
    epoch: Optional[int] = None,
    readonly: bool = False,
    split_read: bool = True,
):
    """Create a Session in a global store"""
    assert store.parent is not None, f"missing parent for {store!r}"
    return Session(
        parent=None,
        _default_scope=EMPTY_SCOPE._to_data(),
        _engines=engines,
        _system_epoch=epoch,
        _supergraph=supergraph or store.parent._supergraph.instance(),
        _oracle=oracle,
        _is_readonly=readonly,
        _split_read=split_read,
    )


def global_pg_cursor(store: Store, *, autocommit: bool = False):
    assert store.parent is not None, f"missing parent for {store!r}"
    return AsyncPostgresConnection(store, store.parent, autocommit=autocommit)


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
    supergraph: NodeSuperGraph,
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
            removed.pop(node.id, None)
            added[node.id] = node
        elif edit.type in (EditType.MOVE, EditType.UPDATE):
            if node.id not in added:
                updated[node.id] = node
        elif edit.type in (EditType.ARCHIVE, EditType.DELETE, EditType.ERASE):
            added.pop(node.id, None)
            updated.pop(node.id, None)
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
    for edit in cascaded_edits:
        node_type = NodeType(edit.node_ptr.type)
        edited_types[node_type.ord] = True
        # unpack
        if edit.type in (EditType.ARCHIVE, EditType.DELETE, EditType.ERASE):
            assert edit.old_node_partial, f"missing old node data for {edit!r}"
            node = wiring.unwrap_some_node(edit.old_node_partial)
        elif edit.type in (EditType.UNARCHIVE, EditType.RESTORE):
            assert edit.new_node_partial, f"missing new node data for {edit!r}"
            node = wiring.unwrap_some_node(edit.new_node_partial)
        else:
            raise RuntimeError(f"unexpected cascaded edit type {edit.type} in {edit!r}")
        assert node.parent_ptr, f"missing parent ptr for {node!r} in {edit!r}"
        parent_id = UUID(node.parent_ptr.id)
        for graph in graphs:
            parent = graph.get(parent_id)
            if parent is not None:
                break
        else:
            # cascaded edits should bei in pre-order, so the parent must exist
            raise RuntimeError(f"missing parent {parent_id} for {node!r} in {edit!r}")
        # cascaded nodes may also be regularly edited nodes, so we add/update them
        node = wiring.unpack_object(
            node, supergraph=supergraph, parent=parent, session=session, expect=Node
        )
        if node.id not in parent._graph:
            parent._graph.add(node)
        else:
            parent._graph.update(node)

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


class HostApi(abc.ABC):
    """Base interface for the Host so we can pass it around more easily (and stub it)."""

    @abc.abstractmethod
    def on_error(self, source: "HostPlugin", error: Exception) -> None:
        """Handle a fatal error."""
        ...

    @property
    @abc.abstractmethod
    def global_store(self) -> Store:
        """The global store for the Host."""
        ...

    @property
    @abc.abstractmethod
    def oracle(self) -> Oracle: ...

    @abc.abstractmethod
    @asynccontextmanager
    async def session(
        self, *, readonly: bool = False, autocommit: bool = False
    ) -> Generator[Session, None, None]:
        """Gets the Session for short-lived, *exclusive access."""
        ...

    def __mapping__(self):
        # NOTE: __mapping__ is required for grpclib base classes (we subclass this in Host)
        return {}


class HostProxy(HostApi):
    def __init__(self, global_store: Store, session: Session):
        self._global_store = global_store
        self._session = session

    def on_error(self, source: Any, error: Exception) -> None:
        pass

    @property
    def global_store(self) -> Store:
        return self._global_store

    @property
    def oracle(self) -> Oracle:
        return self._session._oracle

    @asynccontextmanager
    async def session(self, *, readonly: bool = False, autocommit: bool = False):
        yield self._session


class HostPlugin[T: Node](abc.ABC):
    """A plugin into the Host operating system of a Bench."""

    """The type of nodes to subscribe to for edits."""
    watch_types: ClassVar[bittuple[NodeType]]

    def __init__(self, host: HostApi, bench: "Bench"):
        self.host = host
        self.bench = bench
        self.tasks = TaskManager(
            owner=self,
            logger=logger,
            on_error=lambda e: host.on_error(source=self, error=e),
            task_id_prefix=f"{self.bench.slug}_{self.__class__.__name__}",
            oracle=host.oracle,
        )

    def __str__(self) -> str:
        return ""

    def __repr__(self) -> str:
        content_str = str(self)
        if content_str:
            return f"<{self.__class__.__name__} {content_str} in '{self.bench.slug}'>"
        else:
            return f"<{self.__class__.__name__} in '{self.bench.slug}'>"

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
        self.tasks.close()

    async def wait_closed(self) -> None:
        """After closing, wait for any stuff you need to wait for (if any)."""
        await self.tasks.wait_closed()

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

    def __init__(self, host: HostApi, bench: "Bench"):
        super().__init__(host, bench)
        self._commit_queue: asyncio.Queue[Commit[T]] = asyncio.Queue()

    @override
    async def start(self) -> None:
        await super().start()
        self.tasks.start_queue(self._commit_queue, self.on_commit_deferred, skip_errors=True)

    @override
    @final
    async def on_commit(self, session: Session, commit: Commit) -> None:
        self._commit_queue.put_nowait(commit)
        await self._do_on_commit(session, commit)

    async def _do_on_commit(self, session: Session, commit: Commit[T]) -> None:
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
        self.tasks.check_no_errors()

    async def on_commit_deferred(self, commit: Commit) -> None:
        """
        React to the committed changes (outside the request, later).
        NOTE :Robustness: the nodes in each commit may change before this is called
        """
        pass
