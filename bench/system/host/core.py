import abc
import asyncio
from contextlib import asynccontextmanager
from dataclasses import dataclass
from itertools import chain
from typing import Any, ClassVar, Collection, Generator, Iterable, Sequence, final, override
from uuid import UUID

import bitarray
import structlog
from opentelemetry import trace

from bench.language import Bench, Node, NodeType, Store
from bench.language.const import (
    EditType,
)
from bench.language.graph import NodeGraphLike, NodeSuperGraph
from bench.language.session import Session
from bench.proto import wiring
from bench.proto.wire import EditData
from bench.utils.func import bittuple
from bench.utils.oracle import Oracle
from bench.utils.task import TaskManager

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


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
            edits=[e for e in self.edits if NodeType(e.node_ptr.node_type) in node_types],
            cascaded_edits=[
                e for e in self.cascaded_edits if NodeType(e.node_ptr.node_type) in node_types
            ],
            edited_types=self.edited_types & node_types,
            added=tuple(node for node in self.added if node.metatype in node_types),
            updated=tuple(node for node in self.updated if node.metatype in node_types),
            removed=tuple(node for node in self.removed if node.metatype in node_types),
            epoch=self.epoch,
        )


def unpack_commit(
    session: Session,
    graph: NodeGraphLike,
    supergraph: NodeSuperGraph,  # graph may not be in supergraph :StaleNodes
    edits: Sequence[EditData],
    cascaded_edits: Sequence[EditData],
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
        if edit.type in (EditType.CREATE, EditType.RESTORE):
            # TODO :Broken: not sure how to handle upsert here yet (just error for now)
            removed.pop(node.id, None)
            added[node.id] = node
        elif edit.type in (EditType.MOVE, EditType.UPDATE):
            if node.id not in added:
                updated[node.id] = node
        elif edit.type in (EditType.DELETE, EditType.ERASE):
            added.pop(node.id, None)
            updated.pop(node.id, None)
            removed[node.id] = node
        else:
            raise RuntimeError(f"unexpected edit type {edit.type} in {edit!r}")

    # the nodes edited in 'edits' are expected to be in one of the graphs
    for edit in edits:
        node_type = NodeType(edit.node_ptr.node_type)
        edited_types[node_type.ord] = True
        # unpack
        node_id = UUID(edit.node_ptr.id)
        node = graph.get(node_id) or supergraph.get(node_id)
        if node is None:
            if node_type == NodeType.LOG:
                # access logs which are just created for each edit
                continue
            raise RuntimeError(f"missing node {node_id!r} in {supergraph!r} for {edit!r}")
        # map
        _add_edit(edit, node)

    # any cascaded edits are expected to be full trusted nodes (from archive/unarchive/...)
    for edit in cascaded_edits:
        node_type = NodeType(edit.node_ptr.node_type)
        edited_types[node_type.ord] = True
        # unpack
        if edit.type in (EditType.DELETE, EditType.ERASE):
            assert edit.HasField("node_data"), f"missing node_data for {edit!r}"
            node = wiring.unwrap_some_node(edit.node_data)
        elif edit.type == EditType.RESTORE:
            assert edit.HasField("node_data"), f"missing node_data for {edit!r}"
            node = wiring.copy_struct(wiring.unwrap_some_node(edit.node_data))
            if edit.type == EditType.RESTORE:
                node.ClearField("deleted_at")
        else:
            raise RuntimeError(f"unexpected cascaded edit type {edit.type} in {edit!r}")
        assert node.parent_ptr, f"missing parent ptr for {node!r} in {edit!r}"
        parent_id = UUID(node.parent_ptr.id)
        parent = supergraph.get(parent_id)
        if parent is None:
            # cascaded edits should be in pre-order, so the parent must exist somewhere
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
        self, *, readonly: bool = False, commit: bool = False
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
    async def session(self, *, readonly: bool = False, commit: bool = False):
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
        The commit contains only direct edits, not cascaded edits.,
        Edit nodes directly, flush only when necessary.
        """
        pass

    async def on_commit(self, session: Session, commit: Commit[T]) -> None:  # noqa: B027
        """
        React to the commit in a new transaction (but still in the request lifecycle).
        The commit contains edits and cascaded edits.
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
        pass  # to be overridden
