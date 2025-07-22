from collections.abc import Sequence
from typing import TYPE_CHECKING, Optional, override

from destack.language import EMPTY_LIST, Entity, Graph, Node, NodeType, TraitType, expand_node_types
from destack.language.registry import NODE_CLASS_BY_TYPE
from destack.utils.fractional import INTEGER_ZERO
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    from destack.language import NodeType, TraitType

type_ = type


class MemoryGraph(Graph):
    __slots__ = ("nodes_by_id", "nodes_by_parent")

    def __init__(self):
        self.nodes_by_id: dict[UUID, Entity] = {}
        self.nodes_by_parent: dict[UUID, dict[NodeType, list[Entity]]] = {}

    @override
    def get(self, id: UUID) -> Optional["Entity"]:
        return self.nodes_by_id.get(id)

    @override
    def has(self, id: UUID) -> bool:
        return id in self.nodes_by_id

    @override
    def clear(self):
        # nodes
        self.nodes_by_id.clear()
        self.nodes_by_parent.clear()

    @override
    def add(self, node: "Entity"):
        if (existing := self.nodes_by_id.get(node.id)) is not None:
            raise ValueError(f"node {node!r} already in {self!r}: {existing!r}")
        # node
        self.nodes_by_id[node.id] = node
        # parent
        if (parent_ptr := node.parent_ptr) is not None:
            if parent_ptr.id not in self.nodes_by_parent:
                self.nodes_by_parent[parent_ptr.id] = {}
            child_node_type = node.metatype
            if child_node_type not in self.nodes_by_parent[parent_ptr.id]:
                self.nodes_by_parent[parent_ptr.id][child_node_type] = []
            self.nodes_by_parent[parent_ptr.id][child_node_type].append(node)

    @override
    def remove(self, node: "Entity"):
        # parent
        if (parent_ptr := node.parent_ptr) is not None:
            if parent_ptr.id not in self.nodes_by_parent:
                self.nodes_by_parent[parent_ptr.id] = {}
            child_node_type = node.metatype
            if child_node_type not in self.nodes_by_parent[parent_ptr.id]:
                self.nodes_by_parent[parent_ptr.id][child_node_type] = []
            self.nodes_by_parent[parent_ptr.id][child_node_type].remove(node)
            if not self.nodes_by_parent[parent_ptr.id][child_node_type]:
                self.nodes_by_parent[parent_ptr.id].pop(child_node_type)
                if not self.nodes_by_parent[parent_ptr.id]:
                    self.nodes_by_parent.pop(parent_ptr.id)
        # node
        self.nodes_by_id.pop(node.id)

    @override
    def get_children[M: "Entity" = "Entity"](
        self,
        node: "Node",
        type: NodeType | type[M] | None = None,
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
        self, node: "Entity", type: NodeType | type[M] | None = None
    ) -> Sequence[M]:
        ancestors: list[Entity] = []
        current = node.parent_ptr
        node_types = expand_node_types(type, expand_inheritance=True)

        # traverse up the parent chain
        while current is not None:
            parent_node = self.get(current.id)
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
        type: NodeType | type[M] | None = None,
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
