from collections.abc import Collection, Sequence
from datetime import datetime
from typing import TYPE_CHECKING, Optional, override

from destack.language import EMPTY_LIST, Entity, Graph, Node, NodeType, TraitType, expand_node_types
from destack.language.registry import NODE_CLASS_BY_TYPE
from destack.utils.fractional import INTEGER_ZERO
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    from destack.language import Event, NodeType, Snapshot, TraitType

type_ = type


class MemoryGraph(Graph):
    """A Graph that stores Nodes in memory."""

    __slots__ = ("nodes_by_id", "nodes_by_parent")

    def __init__(self):
        self.nodes_by_id: dict[UUID, Entity] = {}
        self.nodes_by_parent: dict[UUID, dict[NodeType, list[Entity]]] = {}

    #
    # Meta
    #

    @override
    async def open(self) -> None:
        pass

    @override
    async def close(self) -> None:
        pass

    #
    # Write
    #

    def snapshot(
        self,
        space_id: UUID,
        branch_id: UUID | None,
        snapshot_id: UUID | None,
        epoch: int,
    ) -> "Snapshot":
        """Create a Snapshot."""
        raise NotImplementedError

    def insert(
        self,
        space_id: UUID,
        branch_id: UUID | None,
        snapshot_id: UUID | None,
        entities: "Sequence[Entity]",
    ) -> None:
        """Insert Entities into the Graph directly."""
        raise NotImplementedError

    @override
    def append(self, events: "Sequence[Event]") -> None:
        """Append Events to the Graph. EditEvents are reflected immediately."""
        raise NotImplementedError

    @override
    def restate(self, events: "Sequence[Event]") -> None:
        """Restate Events to the Graph. EditEvents are reflected immediately."""
        raise NotImplementedError

    @override
    async def prune(
        self,
        space_id: UUID,
        branch_id: UUID | None,
        snapshot_id: UUID | None,
    ) -> None:
        """Prune the Graph."""
        raise NotImplementedError

    @override
    async def commit(self) -> None:
        """Ensure Events/Entities are persisted in the Graph."""
        raise NotImplementedError

    #
    # Read
    #

    @override
    def seek(
        self,
        space_id: UUID,
        branch_id: UUID | None,
        snapshot_id: UUID | None,
        type: NodeType | Collection[NodeType] | None = None,
        after: datetime | int | None = None,
        before: datetime | int | None = None,
    ) -> "Sequence[Event]":
        """Seek Events from the Graph."""
        raise NotImplementedError

    @override
    def get(
        self,
        id: UUID,
        space_id: UUID,
        branch_id: UUID,
        snapshot_id: UUID,
        include_deleted: bool = False,
    ) -> Optional["Entity"]:
        return self.nodes_by_id.get(id)

    @override
    def get_children[M: "Entity" = "Entity"](
        self,
        node: "Node",
        space_id: UUID,
        branch_id: UUID,
        snapshot_id: UUID,
        type: NodeType | type[M] | None = None,
        include_deleted: bool = False,
    ) -> Sequence[M]:
        # bail if no children
        if not self.nodes_by_parent:
            return ()
        children_by_type = self.nodes_by_parent.get(node.id)
        if not children_by_type:
            return ()

        if type is None:
            # collect children across all types
            children: list = []
            is_ordered = False
            for children_of_type in children_by_type.values():
                node_cls = type_(children_of_type[0])
                if TraitType.ORDERED in node_cls.__traits__:
                    is_ordered = True
                children.extend(children_of_type)
            if is_ordered:
                children.sort(key=lambda n: getattr(n, "order_key", INTEGER_ZERO))
            return children
        else:
            # turn into type
            node_types = expand_node_types(type, expand_inheritance=True)
            if not node_types:
                return ()
            node_cls = NODE_CLASS_BY_TYPE[node_types[0]]

            # collect
            if len(node_types) == 1:
                # collect for single node type
                children: list = children_by_type.get(node_types[0], EMPTY_LIST)
                if children and TraitType.ORDERED in node_cls.__traits__:
                    children.sort(key=lambda n: getattr(n, "order_key", INTEGER_ZERO))
                return children
            else:
                # collect for trait (multiple node types)
                children: list = []
                for node_type in node_types:
                    children.extend(children_by_type.get(node_type, EMPTY_LIST))
                if children and TraitType.ORDERED in node_cls.__traits__:
                    children.sort(key=lambda n: getattr(n, "order_key", INTEGER_ZERO))
                return children

    @override
    def get_ancestors[M: "Entity" = "Entity"](
        self,
        node: "Entity",
        space_id: UUID,
        branch_id: UUID,
        snapshot_id: UUID,
        type: NodeType | type[M] | None = None,
        include_deleted: bool = False,
    ) -> Sequence[M]:
        ancestors: list[Entity] = []
        current = node.parent_ptr
        node_types = expand_node_types(type, expand_inheritance=True)

        # traverse up the parent chain
        while current is not None:
            parent_node = self.get(current.id, space_id, branch_id, snapshot_id)
            if parent_node is None:
                break
            if node_types is None or parent_node.metatype in node_types:
                ancestors.append(parent_node)
            current = parent_node.parent_ptr

        return ancestors  # type: ignore (must be right type)

    @override
    def get_descendants[M: "Entity" = "Entity"](
        self,
        node: "Entity",
        space_id: UUID,
        branch_id: UUID,
        snapshot_id: UUID,
        type: NodeType | type[M] | None = None,
        include_deleted: bool = False,
    ) -> Sequence[M]:
        if not self.nodes_by_parent:
            return ()
        queue: list[Entity] = [node]
        descendants: list[Entity] = []
        node_types = expand_node_types(type, expand_inheritance=True)

        # BFS
        while queue:
            current = queue.pop(0)
            children_by_type = self.nodes_by_parent.get(current.id)
            if not children_by_type:
                continue
            for children_of_type in children_by_type.values():
                queue.extend(children_of_type)

            if not node_types:
                for children_of_type in children_by_type.values():
                    descendants.extend(children_of_type)
            else:
                for node_t in node_types:
                    descendants.extend(children_by_type.get(node_t, ()))

        return descendants  # type: ignore (must be right type)
