from collections.abc import Sequence
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Optional,
    Self,
    cast,
    dataclass_transform,
    override,
)

import structlog
from fastuuid import UUID
from opentelemetry import trace

from destack.language.registry import (
    NODE_CLASS_BY_TYPE,
    NODE_TYPE_BY_CLASS,
    ORDER_GROUPS_BY_NODE_TYPE,
)
from destack.pb2 import AnyNodeData
from destack.utils.fractional import get_order_key
from destack.utils.func import get_superclasses

from .const import NodeType, TraitType
from .object import _process_object_cls
from .property import (
    _PROPERTY_SPECIFIERS,
    _resolve_trait_type,
    property_,
    property_parent_,
    property_runtime_,
)
from .trait import IndexIn, NodeBase

if TYPE_CHECKING:
    from destack.language import Graph, NodeReference, QueryConnection, Session, Supergraph

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def builtin_node(
    node_type: NodeType | None,
    root_type: NodeType | None = NodeType.SPACE,
    pretend_frozen: bool = False,  # :PretendFrozen
    index: tuple[IndexIn, ...] = (),
):
    """Register a class as a concrete node for the given node type."""

    # default index for nodes with parents
    if root_type:
        index = (*index, IndexIn(columns=("parent_id",), cover=("id",)))

    def decorate(cls: type["Node"]) -> type["Node"]:
        assert cls.__name__ == "Node" or issubclass(cls, Node), f"{cls.__name__} is not a Node"
        cls.__is_trait__ = False  # override Trait.__is_trait__
        traits = set()
        for superclass in get_superclasses(cls):
            if trait := _resolve_trait_type(superclass.__name__):
                traits.add(trait)
        cls.__traits__ = tuple(traits)

        cls, _ = _process_object_cls(
            cls=cls,
            object_type=node_type,
            is_concrete=node_type is not None,
            is_node=True,
            is_root_node=root_type is None,
            is_frozen=pretend_frozen,
            traits=cls.__traits__,
        )
        cls.__indexes__ = index

        if node_type is not None:
            # register
            cls.metatype = node_type
            NODE_CLASS_BY_TYPE[node_type] = cls
            NODE_TYPE_BY_CLASS[cls] = node_type
            NODE_CLASS_BY_TYPE[node_type] = cls

        # parent/root
        parent_property = cls.__properties__.get("parent", None)
        assert parent_property is not None, f"missing parent property for {node_type}"
        cls.__parent_property__ = parent_property
        cls.__root_type__ = root_type

        return cls

    return decorate


_object_set = object.__setattr__


@builtin_node(node_type=None, root_type=None)
class Node[NodeDataT: AnyNodeData](NodeBase[NodeDataT]):
    """
    A Node with Properties and a persistent identity.
    """

    metatype: ClassVar[NodeType]

    # 1-9: node identity
    # Node.metatype: 1
    id: UUID = property_(2, is_managed=True, is_eq=False, can_write="system")
    # Node.variant_id: 3?
    parent: Optional["Node"] = property_parent_(node_is_customizable=True)
    if TYPE_CHECKING:
        parent_type: NodeType | None = None
        parent_id: Optional[UUID] = None
        parent_ptr: Optional[NodeReference] = None
    # Spatial.space: 5
    # Node.store: 6/7?

    _session: "Session" = property_runtime_()
    _supergraph: "Supergraph" = property_runtime_()
    _graph: "Graph" = property_runtime_(default=None)
    _connection: "QueryConnection | None" = property_runtime_(default=None)
    _hash: int = property_runtime_(default=None)
    _ref: "Optional[NodeReference]" = property_runtime_(default=None)
    _is_new: bool = property_runtime_(default=False)
    _is_attached: bool = property_runtime_(default=False)
    _dirty: dict[str, Any] | None = property_runtime_(default=None)

    @override
    def clone(
        self,
        *,
        reset: bool = True,
        recursive: bool = True,
        detach: bool = False,
        _map: bool | dict[UUID, "Node"] = True,
        _ignore_definition: bool = False,
        _is_nested: bool = False,
        **kwargs,
    ) -> Self:
        raise NotImplementedError  # generate

    def __eq__(self, other: Any):
        """Equals the Node's identity."""
        return type(self) is type(other) and (self.id == other.id)

    def hash(self):
        """Hash the Node's identity."""
        return self._hash

    __hash__ = hash  # type: ignore

    def _do_set(self, key: str, value: Any):
        """Set a property on this Node."""
        prop = self.__tracked_properties__.get(key)
        if prop is not None and not self._is_new:
            old_value = getattr(self, key)
            if self._dirty is None:
                self._dirty = {}
            if prop.name not in self._dirty:
                self._dirty[prop.name] = old_value  # type: ignore
            if self.id not in self._session.dirty:
                self._session.dirty[self.id] = self
        _object_set(self, key, value)

    if not TYPE_CHECKING:
        __setattr__ = _do_set

    @property
    def path(self) -> str:
        raise NotImplementedError  # generated

    def __to_ref__(self) -> "NodeReference":
        """Gets a reference to this node. May be rich in subclasses."""
        raise NotImplementedError  # generated

    def to_ref(self) -> "NodeReference":
        """Gets a reference to this node. May be rich in subclasses."""
        if self._ref is None:
            self._ref = self.__to_ref__()
        return self._ref

    def erase(self):
        """Wipe this Node from this cosmos forever."""
        self._session.erase(self)

    def move_to(self, parent: "Node"):
        """Move this Node to a new parent."""
        raise NotImplementedError

    def add_child(
        self,
        child: "Node",
        *,
        after: "Node | None" = None,
        before: "Node | None" = None,
    ) -> Self:
        """
        Append a Node as a child of this Node (and all its descendants).
        If the Node is new, it will be created in this Node's session.
        If the Node IsOrdered, it will be positioned (relative to after/before).
        (The same applies to all descendants.)
        """
        from ..runtime import SingletonGraph

        old_graph = child._graph
        new_graph = self._graph
        session = self._session
        nodes: tuple[Node, ...] = (child, *child._graph.get_descendants(child))

        assert self.metatype in child.__parent_types__, (
            f"{self!r} cannot parent {child!r} (allowed: {child.__parent_types__})"
        )
        assert old_graph is not new_graph, f"{child!r} is already in same graph of {self!r}"
        assert old_graph.supergraph is self._supergraph, (
            f"{child!r} is not in supergraph of {self!r}"
        )

        # assign order
        if (order_group := ORDER_GROUPS_BY_NODE_TYPE.get(child.metatype)) is not None:
            existing_nodes = self._graph.get_children(self, node_type=order_group)
            if existing_nodes:
                order_key = get_order_key(getattr(existing_nodes[-1], "order_key", None), None)
                child._do_set("order_key", order_key)

        # promote self to polygraph if needed
        if isinstance(new_graph, SingletonGraph):
            new_graph = self._supergraph.promote_to_polygraph(new_graph)
            self._graph = new_graph

        # move to new graph
        if len(nodes) == len(old_graph):  # all nodes were moved
            self._supergraph.remove_graph(old_graph)
        else:
            for node in nodes:
                old_graph.remove(node)
        child.parent_ptr = self.to_ref()
        for node in nodes:
            node._graph = new_graph
            new_graph.add(node)

        # create new nodes
        if child._is_new and self._is_attached:
            for node in nodes:
                node._ref = None  # invalidate cached ref
                session.create(node)

        return self

    def add_children(
        self,
        *children: "Node",
        after: "Node | None" = None,
        before: "Node | None" = None,
    ) -> Self:
        """
        Append multiple Nodes as children of this Node.
        TODO :Performance: batch Node.add_children (per type?)
        """
        for child in children:
            self.add_child(child, after=after, before=before)
        return self

    def remove_child(self, child: "Node") -> Self:
        """
        Remove a child from this Node.
        If the Node IsDeletable, it will be deleted; otherwise, it will be erased.
        """
        raise NotImplementedError

    def get_children[N: Node = Node](
        self, node_type: NodeType | TraitType | type[N] | None = None
    ) -> Sequence[N]:
        """Gets the children of this Node."""
        return self._graph.get_children(self, node_type=node_type)

    def get_child[N: Node = Node](self, node_type: NodeType | type[N], key: str) -> N | None:
        """Gets a specific child of this Node."""
        if isinstance(node_type, type):
            node_type = node_type.metatype
        for child in self._graph.get_children(self, node_type=node_type):
            if getattr(child, "name", None) == key:
                return cast(N, child)
        return None

    def child[N: Node = Node](self, node_type: NodeType | type[N], key: str) -> N:
        """Gets a specific child of this Node, or raises an error if not found."""
        child = self.get_child(node_type, key)
        if child is None:
            raise LookupError(f"no child {key} of {self!r}")
        return cast(N, child)

    def get_descendants[N: Node = Node](
        self, node_type: NodeType | TraitType | type[N] | None = None
    ) -> Sequence[N]:
        """Gets the descendants of this Node."""
        return self._graph.get_descendants(self, node_type=node_type)

    @classmethod
    def from_value(
        cls,
        _object_value: dict,
        _session: "Session | None" = None,
        _supergraph: "Supergraph | None" = None,
        _graph: "Graph | None" = None,
        _connection: "QueryConnection | None" = None,
    ) -> Self:
        node_type = NodeType(_object_value["1"])
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        node = node_cls.from_value(
            _object_value,
            _session=_session,
            _supergraph=_supergraph,
            _graph=_graph,
            _connection=_connection,
        )
        return cast(Self, node)
