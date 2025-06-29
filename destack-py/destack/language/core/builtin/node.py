from collections.abc import Sequence
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Optional,
    Self,
    Union,
    cast,
    dataclass_transform,
)

from destack.language.registry import NODE_CLASS_BY_TYPE, NODE_TYPE_BY_CLASS
from destack.proto import AnyNodeProto
from destack.utils.fractional import get_order_key
from destack.utils.func import get_superclasses
from destack.utils.uuid import UUID

from .common import NodeType, ResourceStatus, RoleType, TraitType
from .object import _process_object_cls
from .property import (
    _PROPERTY_SPECIFIERS,
    _resolve_trait_type,
    property_,
    property_parent_,
    property_runtime_,
)
from .trait import (
    INTER_ORDER_TYPES,
    HasName,
    IndexIn,
    IsActionable,
    IsDeletable,
    IsExtensible,
    IsOrdered,
    IsOwnable,
    IsScriptable,
    IsSourceable,
    IsTaggable,
    IsTracked,
    NodeBase,
    Spatial,
)

if TYPE_CHECKING:
    from destack.language import (
        Folder,
        Graph,
        NodeReference,
        QueryConnection,
        Session,
        Supergraph,
    )

# pyright: reportIncompatibleVariableOverride=false

type_ = type


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def builtin_node(
    node_type: NodeType | None,
    root_type: NodeType | None = NodeType.SPACE,
    pretend_frozen: bool = False,  # :PretendFrozen
    index: tuple[IndexIn, ...] = (),
    is_abstract: bool = False,
):
    """Register a class as a concrete node for the given node type."""

    # default index for nodes with parents
    if root_type:
        index = (*index, IndexIn(columns=("parent_id",), cover=("id",)))

    def decorate(cls: type["Node"]) -> type["Node"]:
        assert cls.__name__ == "Node" or issubclass(cls, Node), f"{cls.__name__} is not a Node"
        cls.__is_trait__ = False  # override Trait.__is_trait__
        traits = set()
        base_traits = set()
        for superclass in get_superclasses(cls):
            if trait := _resolve_trait_type(superclass.__name__):
                traits.add(trait)
        for base in cls.__bases__:
            if trait := _resolve_trait_type(base.__name__):
                base_traits.add(trait)
        cls.__traits__ = tuple(traits)
        cls.__base_traits__ = tuple(base_traits)
        cls.__is_abstract__ = is_abstract

        cls, _ = _process_object_cls(
            cls=cls,
            object_type=node_type,
            is_concrete=node_type is not None,
            is_node=True,
            is_root_node=root_type is None,
            is_frozen=pretend_frozen,
            is_abstract=is_abstract,
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
class Node[NodeProtoT: AnyNodeProto](NodeBase[NodeProtoT]):
    """
    A Node with Properties and a persistent identity.
    """

    metatype: ClassVar[NodeType]

    # 1-14: node identity
    # Node.metatype: 1
    id: UUID = property_(2, is_managed=True, is_eq=False, can_write=RoleType.SYSTEM)
    parent: Optional["Node"] = property_parent_(node_is_customizable=True)
    # Node.store: 4
    # Spatial.space: 5
    # IsCustomNode.definition: 6
    # Entity.materialization: 7
    # Entity.snapshot/template: 8-11
    # Entity.set_properties: 12
    if TYPE_CHECKING:
        parent_ptr: Optional[NodeReference] = None

    _session: "Session" = property_runtime_()
    _supergraph: "Supergraph" = property_runtime_()
    _graph: "Graph" = property_runtime_(default=None)
    _connection: "QueryConnection | None" = property_runtime_(default=None)
    _ref: "Optional[NodeReference]" = property_runtime_(default=None)
    _is_new: bool = property_runtime_(default=False)
    _is_attached: bool = property_runtime_(default=False)
    _dirty: dict[str, Any] | None = property_runtime_(default=None)

    def __eq__(self, other: Any):
        """Equals the Node's identity."""
        return type(self) is type(other) and (self.id == other.id)

    def __hash__(self):
        """Hash the Node's identity."""
        return self.id.int

    def _do_set(self, key: str, value: Any):
        """Set a Property on this Node."""
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
        """Gets a reference to this Node."""
        raise NotImplementedError  # generated

    def to_ref(self) -> "NodeReference":
        """Gets a reference to this Node."""
        if self._ref is None:
            self._ref = self.__to_ref__()
        return self._ref

    def erase(self):
        """Erase this Node from this universe forever."""
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
        if isinstance(child, IsOrdered):
            order_trait = next(
                (trait for trait in child.__traits__ if trait in INTER_ORDER_TYPES), None
            )
            existing_nodes = self._graph.get_children(self, type=order_trait or child.metatype)
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

        # assign space
        if isinstance(child, Spatial) and (
            (isinstance(self, Spatial) and (space_ptr := self.space_ptr) is not None)
            or (self.metatype == NodeType.SPACE and (space_ptr := self.to_ref()) is not None)
        ):
            for node in nodes:
                if isinstance(node, Spatial):
                    node.space_ptr = space_ptr

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
        self,
        type: NodeType | TraitType | type[N] | None = None,
        include_deleted: bool = False,
        include_archived: bool = False,
    ) -> Sequence[N]:
        """Gets the children of this Node."""
        return self._graph.get_children(self, type=type)

    def get_child[N: Node = Node](
        self,
        type: NodeType | type[N] | TraitType,
        name: str,
        include_deleted: bool = False,
        include_archived: bool = False,
    ) -> N | None:
        """Gets a specific child of this Node by name."""
        if isinstance(type, type_):
            type = type.metatype
        for child in self._graph.get_children(self, type=type):
            if getattr(child, "name", None) == name:
                return cast(N, child)
        return None

    def child[N: Node = Node](
        self,
        type: NodeType | type[N] | TraitType,
        name: str,
        include_deleted: bool = False,
        include_archived: bool = False,
    ) -> N:
        """Gets a specific child of this Node by name, or raises an error if not found."""
        child = self.get_child(type, name)
        if child is None:
            raise LookupError(f"no child {name} of {self!r}")
        return cast(N, child)

    def get_ancestors[N: Node = Node](
        self,
        type: NodeType | TraitType | type[N] | None = None,
        include_deleted: bool = False,
        include_archived: bool = False,
    ) -> Sequence[N]:
        """Gets the ancestors of this Node."""
        return self._graph.get_ancestors(self, type=type)

    def get_descendants[N: Node = Node](
        self,
        type: NodeType | TraitType | type[N] | None = None,
        include_deleted: bool = False,
        include_archived: bool = False,
    ) -> Sequence[N]:
        """Gets the descendants of this Node."""
        return self._graph.get_descendants(self, type=type)

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


@builtin_node(NodeType.ENTITY)
class Entity(IsTracked, Node):
    """
    An Entity is a versioned Node in primary relational storage (OLTP).
    """

    # nocheckin: support Entity branching & variants
    # primary key: (id, snapshot_id)
    # two pairs of ids (root container, base pointer):
    #  - time: (snapshot_id, base_id)
    #  - space: (instance_id, template_id)
    # when merging: time before space (id+snapshot_id over template)
    # snapshot and template properties must be READ ONLY (no write)
    # materialization: MaterializationType = property_(
    #     7,
    #     is_managed=True,
    #     is_eq=False,
    #     is_hash=False,
    #     is_repr=False,
    #     default=MaterializationType.FULL_GRAPH,
    # )
    # snapshot: Optional["Snapshot"] = property_(
    #     8,
    #     can_write=None,
    #     is_managed=True,
    #     node_space_from="self",
    #     description="The Snapshot this Entity is part of.",
    # )
    # base: Optional["Snapshot"] = property_(
    #     9,
    #     can_write=None,
    #     is_managed=True,
    #     node_is_customizable=False,
    #     node_space_from="self",
    #     description="The Snapshot this Entity's snapshot is based on.",
    # )
    # instance: Optional["Entity"] = property_(
    #     10,
    #     can_write=None,
    #     is_managed=True,
    #     node_is_customizable=False,
    #     description="The (root) Entity in this Entity's instance tree.",
    # )
    # template: Optional["Entity"] = property_(
    #     11,
    #     can_write=None,
    #     is_managed=True,
    #     node_is_customizable=False,
    #     description="The template this Entity instance is based on.",
    # )
    # Entity.set_properties/set_fields: 12-13
    if TYPE_CHECKING:
        snapshot_ptr: Optional["NodeReference"] = None
        base_ptr: Optional["NodeReference"] = None
        instance_ptr: Optional["NodeReference"] = None
        template_ptr: Optional["NodeReference"] = None


@builtin_node(NodeType.CUSTOM_ENTITY_DEFINITION)
class CustomEntityDefinition(
    Spatial,
    HasName,
    IsTaggable,
    IsOwnable,
    IsDeletable,
    IsScriptable,
    IsSourceable,
    IsActionable,
    Entity,
):
    """
    A definition for a custom Entity type (instantiated in CustomEntities).
    Custom Entities may be materialized as physical or logical tables in primary storage.
    """

    # type?
    parent: Optional["Folder"] = property_parent_(node_is_customizable=False)
    prototype: Optional["CustomEntity"] = property_(
        6,
        description="A custom Entity's prototype is the default template new CustomEntity instances are based on.",
    )
    traits: list[TraitType] = property_(40)


@builtin_node(NodeType.CUSTOM_ENTITY)
class CustomEntity(
    Spatial,
    IsExtensible,
    IsDeletable,
    Entity,
):
    """
    A CustomEntity is an instance of a CustomEntityDefinition.
    """

    parent: Union["CustomEntityDefinition", "CustomEntity", None] = property_parent_(
        node_is_customizable=True
    )
    definition: "CustomEntityDefinition" = property_(
        6,
        description="The CustomEntityDefinition this CustomEntity is an instance of.",
        is_managed=True,
        can_write=None,
    )
    if TYPE_CHECKING:
        definition_ptr: Optional[NodeReference] = None


@builtin_node(NodeType.EVENT, pretend_frozen=True)
class Event[N: Node = Node](Spatial):
    """
    An Event represents something happening in a Space.
    """

    node: Optional["Node"] = property_(35, description="The Node this Event is about.")
    if TYPE_CHECKING:
        node_ptr: Optional[NodeReference] = None


@builtin_node(NodeType.CUSTOM_EVENT_DEFINITION)
class CustomEventDefinition(
    Spatial,
    HasName,
    IsSourceable,
    Entity,
):
    """A CustomEventDefinition defines a kind of CustomEvent."""

    pass


@builtin_node(NodeType.CUSTOM_EVENT, pretend_frozen=True)
class CustomEvent(Event):
    """An instance of a CustomEventDefinition."""

    definition: "CustomEventDefinition" = property_(
        40, description="The CustomEventDefinition this CustomEvent is an instance of."
    )


@builtin_node(NodeType.RESOURCE)
class Resource(Entity):
    """
    A Resource represents an external asset.
    The lifecycle of a Resource may be managed by some provisioner.
    """

    parent: Optional["Folder"] = property_parent_(node_is_customizable=False)
    status: ResourceStatus = property_(40, default=ResourceStatus.PENDING)
    target_status: Optional[datetime] = property_(41)


@builtin_node(NodeType.METRIC)
class Metric(IsSourceable, Entity):
    """An Entity that represents a Metric."""


@builtin_node(NodeType.MEASUREMENT)
class Measurement(Event):
    """An Event that represents a Measurement."""

    definition: "Metric" = property_(6, is_managed=True, can_write=None)
