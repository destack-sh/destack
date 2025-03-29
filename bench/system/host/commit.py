from dataclasses import dataclass
from typing import (
    TYPE_CHECKING,
    Callable,
    Collection,
    Sequence,
)
from uuid import UUID

import bitarray
import structlog
from opentelemetry import trace

from bench.language import (
    EditType,
    Node,
    NodeGraph,
    NodeSuperGraph,
    NodeType,
    Session,
    bittuple,
)
from bench.proto import EditData, wiring

if TYPE_CHECKING:
    pass

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
    edited: tuple[T, ...]
    edited_by_id: dict[UUID, T]
    epoch: int

    def __str__(self):
        return f"added={self.added!r}, updated={self.updated!r}, removed={self.removed!r}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    @property
    def is_empty(self) -> bool:
        return not self.added and not self.updated and not self.removed

    def has(
        self, filter: NodeType | tuple[NodeType, ...] | bittuple[NodeType] | Callable[[T], bool]
    ) -> bool:
        """Check if the diff contains any nodes matching the filter."""
        if callable(filter):
            return any(filter(node) for node in self.edited)
        elif isinstance(filter, NodeType):
            return filter in self.edited_types
        elif isinstance(filter, tuple):
            return any(t in self.edited_types for t in filter)
        else:
            return (self.edited_types.bits & filter.bits).any()

    def trim_to(
        self, filter: NodeType | tuple[NodeType, ...] | bittuple[NodeType] | Callable[[T], bool]
    ) -> "Commit[T]":
        """Trims the diff to only include nodes matching the filter."""
        if callable(filter):
            added = tuple(node for node in self.added if filter(node))
            updated = tuple(node for node in self.updated if filter(node))
            removed = tuple(node for node in self.removed if filter(node))
            edited = tuple(node for node in self.edited if filter(node))
            edited_by_id = {id: node for id, node in self.edited_by_id.items() if filter(node)}
            return Commit(
                edits=[e for e in self.edits if any(filter(n) for n in self.edited)],
                cascaded_edits=[
                    e for e in self.cascaded_edits if any(filter(n) for n in self.edited)
                ],
                edited_types=self.edited_types,  # can't trim bits since we don't know types
                added=added,
                updated=updated,
                removed=removed,
                edited=edited,
                edited_by_id=edited_by_id,
                epoch=self.epoch,
            )
        else:
            if isinstance(filter, NodeType):
                filter = bittuple(filter)
            elif isinstance(filter, tuple):
                filter = bittuple(*filter)
            added = tuple(node for node in self.added if node.metatype in filter)
            updated = tuple(node for node in self.updated if node.metatype in filter)
            removed = tuple(node for node in self.removed if node.metatype in filter)
            edited = tuple(node for node in self.edited if node.metatype in filter)
            edited_by_id = {
                id: node for id, node in self.edited_by_id.items() if node.metatype in filter
            }
            return Commit(
                edits=[e for e in self.edits if NodeType(e.node_ptr.node_type) in filter],
                cascaded_edits=[
                    e for e in self.cascaded_edits if NodeType(e.node_ptr.node_type) in filter
                ],
                edited_types=self.edited_types & filter,
                added=added,
                updated=updated,
                removed=removed,
                edited=edited,
                edited_by_id=edited_by_id,
                epoch=self.epoch,
            )


def unpack_commit(
    session: Session,
    graph: NodeGraph,
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
            # NOTE :Broken: not sure how to handle upsert here yet (just error for now)
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
            raise RuntimeError(
                f"missing node {node_id!r} in {supergraph!r} for {wiring.describe_edit(edit)}"
            )
        # map
        _add_edit(edit, node)

    # any cascaded edits are expected to be full trusted nodes
    for edit in cascaded_edits:
        node_type = NodeType(edit.node_ptr.node_type)
        edited_types[node_type.ord] = True
        # unpack
        if edit.type in (EditType.DELETE, EditType.ERASE):
            assert edit.HasField("node_data"), f"missing node_data for {wiring.describe_edit(edit)}"
            node = wiring.unwrap_some_node(edit.node_data)
        elif edit.type == EditType.RESTORE:
            assert edit.HasField("node_data"), f"missing node_data for {wiring.describe_edit(edit)}"
            node = wiring.copy_struct(wiring.unwrap_some_node(edit.node_data))
            if edit.type == EditType.RESTORE:
                node.ClearField("deleted_at")
        else:
            raise RuntimeError(
                f"unexpected cascaded edit type {edit.type} in {wiring.describe_edit(edit)}"
            )
        # cascaded nodes may also be regularly edited nodes, so we add/update them
        node = wiring.unpack_builtin_object(
            node, supergraph=supergraph, session=session, expect=Node
        )
        if node.id not in graph:
            graph.add(node)
        else:
            graph.update(node)

        # map
        _add_edit(edit, node)

    edited = {}
    edited.update(added)
    edited.update(updated)
    edited.update(removed)
    commit = Commit(
        edits=edits,
        cascaded_edits=cascaded_edits,
        edited_types=bittuple.from_ord(NodeType, edited_types),
        added=tuple(added.values()),
        updated=tuple(updated.values()),
        removed=tuple(removed.values()),
        edited=tuple(edited.values()),
        edited_by_id=edited,
        epoch=epoch,
    )
    return commit
