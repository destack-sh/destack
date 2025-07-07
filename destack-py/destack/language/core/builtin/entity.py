from collections.abc import Sequence
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Any,
    Optional,
    Self,
    Union,
    cast,
)

from destack.utils.fractional import get_order_key

from .common import EnumType, ResourceStatus, RoleType, TraitType, ValueFactory
from .const import ACTIVE_SNAPSHOT
from .enum import Enum, builtin_enum
from .node import Node, NodeType, builtin_node
from .property import (
    builtin_property,
    builtin_property_parent,
    builtin_property_runtime,
)
from .trait import (
    INTER_ORDER_TYPES,
    IsArchivable,
    IsCustomizable,
    IsDeletable,
    IsExtensible,
    IsOrdered,
    IsOwnable,
    IsScriptable,
    IsSourceable,
    IsSpatial,
    IsTaggable,
)

if TYPE_CHECKING:
    from destack.language import (
        Folder,
        Icon,
        IsSubject,
        NodeDefinitionReference,
        NodeReference,
        Space,
    )

# pyright: reportIncompatibleVariableOverride=false

type_ = type
object_set_ = object.__setattr__


@builtin_enum(EnumType.MATERIALIZATION)
class Materialization(Enum):
    """The materialization level of an Entity."""

    PARTIAL = 1, "Partial Node"
    FULL = 32, "Full Node"


@builtin_node(
    NodeType.ENTITY,
    is_abstract=True,
    event_types=(NodeType.EDIT_EVENT,),
)
class Entity(Node):
    """
    An Entity is a versioned, stateful Node.
    """

    # 10-20: entity materialization
    materialization: Materialization = builtin_property(
        10,
        is_readonly=True,
        is_managed=True,
        is_eq=False,
        is_hash=False,
        is_repr=False,
        default=Materialization.FULL,
    )
    snapshot: Optional["Snapshot"] = builtin_property(
        11,
        is_readonly=True,
        is_managed=True,
        node_space_from="self",
        description="The Snapshot this Entity is part of.",
    )
    predecessor: Optional[Self] = builtin_property(
        12,
        is_readonly=True,
        is_managed=True,
        node_space_from="self",
        description="The previous Entity this Entity is based on (from another Snapshot).",
    )
    template: Optional[Self] = builtin_property(
        13,
        is_readonly=True,
        is_managed=True,
        description="The template this Entity instance is based on (from the template tree).",
    )
    instance_root: Optional["Entity"] = builtin_property(
        14,
        is_readonly=True,
        is_managed=True,
        node_space_from="self",
        description="The (root) Entity in this Entity's instance tree (not the template tree).",
    )
    # Entity.set_properties: 15
    if TYPE_CHECKING:
        snapshot_ptr: Optional["NodeReference"] = None
        predecessor_ptr: Optional["NodeReference"] = None
        template_ptr: Optional["NodeReference"] = None
        instance_root_ptr: Optional["NodeReference"] = None

    # 20-40: node tracking
    created_at: datetime = builtin_property(
        20,
        is_managed=True,
        is_eq=False,
        is_readonly=True,
        can_write=RoleType.SYSTEM,
    )
    created_by: Optional["IsSubject"] = builtin_property(
        21,
        default=None,
        is_managed=True,
        is_eq=False,
        is_readonly=True,
        node_space_from="self",
        can_write=RoleType.SYSTEM,
    )
    updated_at: datetime = builtin_property(
        22,
        is_managed=True,
        is_eq=False,
        can_write=RoleType.SYSTEM,
    )
    updated_by: Optional["IsSubject"] = builtin_property(
        23,
        default=None,
        is_managed=True,
        is_eq=False,
        node_space_from="self",
        can_write=RoleType.SYSTEM,
    )
    if TYPE_CHECKING:
        created_by_ptr: Optional[NodeReference] = None
        updated_by_ptr: Optional[NodeReference] = None
    # revision? epoch?

    def _do_set(self, key: str, value: Any):
        """Set a Property on this Node (direct SET/CLEAR operations)."""
        prop = self.__tracked_properties__.get(key)
        if prop is not None and not self._is_new:
            self._session.update_set_property(self, prop, value)
        object_set_(self, key, value)

    if not TYPE_CHECKING:
        __setattr__ = _do_set

    def move_to(self, parent: "Entity"):
        """Move this Node to a new parent."""
        raise NotImplementedError

    def add_child(
        self,
        child: "Entity",
        *,
        after: "Entity | None" = None,
        before: "Entity | None" = None,
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
        nodes: tuple[Entity, ...] = (child, *child._graph.get_descendants(child))

        assert isinstance(self, child.__parent_classes__), (
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
                assert isinstance(child, Entity), f"{child!r} is not an Entity"
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
        if isinstance(child, IsSpatial) and (
            (isinstance(self, IsSpatial) and (space_ptr := self.space_ptr) is not None)
            or (self.metatype == NodeType.SPACE and (space_ptr := self.to_ref()) is not None)
        ):
            for node in nodes:
                if isinstance(node, IsSpatial):
                    node.space_ptr = space_ptr

        # create new nodes
        if child._is_new and self._is_attached:
            for node in nodes:
                assert isinstance(node, Entity), f"{node!r} of {self!r} is not an Entity"
                node._ref = None  # invalidate cached ref
                session.create(node)

        return self

    def add_children(
        self,
        *children: "Entity",
        after: "Entity | None" = None,
        before: "Entity | None" = None,
    ) -> Self:
        """
        Append multiple Nodes as children of this Node.
        TODO :Performance: batch Node.add_children (per type?)
        """
        for child in children:
            self.add_child(child, after=after, before=before)
        return self

    def remove_child(self, child: "Entity") -> Self:
        """
        Remove a child from this Node.
        If the Node IsDeletable, it will be deleted; otherwise, it will be erased.
        """
        raise NotImplementedError

    def get_children[N: Entity = Entity](
        self,
        type: NodeType | TraitType | type[N] | None = None,
        include_deleted: bool = False,
        include_archived: bool = False,
    ) -> Sequence[N]:
        """Gets the children of this Node."""
        return self._graph.get_children(self, type=type)

    def get_child[N: Entity = Entity](
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

    def child[N: Entity = Entity](
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

    def get_ancestors[N: Entity = Entity](
        self,
        type: NodeType | TraitType | type[N] | None = None,
        include_deleted: bool = False,
        include_archived: bool = False,
    ) -> Sequence[N]:
        """Gets the ancestors of this Node."""
        return self._graph.get_ancestors(self, type=type)

    def get_descendants[N: Entity = Entity](
        self,
        type: NodeType | TraitType | type[N] | None = None,
        include_deleted: bool = False,
        include_archived: bool = False,
    ) -> Sequence[N]:
        """Gets the descendants of this Node."""
        return self._graph.get_descendants(self, type=type)

    def into(
        self,
        snapshot: "Snapshot",
        *,
        materialization: Materialization = Materialization.PARTIAL,
    ) -> "Self":
        """
        Turn this Entity into its corresponding Entity in the given Snapshot.
        """

        # bail if already in the given Snapshot
        if (snapshot_ptr := self.snapshot_ptr) is not None and snapshot_ptr.id == snapshot.id:
            return self

        raise NotImplementedError


@builtin_node(NodeType.CUSTOM_ENTITY_DEFINITION)
class CustomEntityDefinition(
    IsSpatial,
    IsCustomizable,
    IsTaggable,
    IsOwnable,
    IsDeletable,
    IsScriptable,
    IsSourceable,
    Entity,
):
    """
    A definition for a custom Entity type.
    Custom Entities are instantiated either as:
     1) their respective extensible base type (like ContainerView)
     2) plain CustomEntity instance (default if not extending any other type)
    """

    parent: Optional["Folder"] = builtin_property_parent()

    base_type: "NodeDefinitionReference" = builtin_property(40)
    base_traits: list["NodeDefinitionReference"] = builtin_property(41)
    is_abstract: bool = builtin_property(45, default=False)

    prototype: Optional["Entity"] = builtin_property(
        50,
        description="A custom Entity's prototype is the default template new CustomEntity instances are based on.",
    )

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)


@builtin_node(NodeType.CUSTOM_TRAIT_DEFINITION)
class CustomTraitDefinition(
    IsSpatial,
    IsSourceable,
    IsDeletable,
    IsScriptable,
    IsCustomizable,
    Entity,
):
    """
    A CustomTraitDefinition defines a kind of CustomTrait.
    """

    parent: Optional["Folder"] = builtin_property_parent()
    base_type: Optional["NodeDefinitionReference"] = builtin_property(40)
    base_traits: list["NodeDefinitionReference"] = builtin_property(41)
    is_abstract: bool = builtin_property(45, default=False)

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)


@builtin_node(NodeType.RECORD, is_abstract=True)
class Record(
    IsSpatial,
    IsExtensible,
    IsArchivable,
    IsDeletable,
    IsOwnable,
    Entity,
):
    """
    A generic Record instance of a CustomEntityDefinition like a relational Table.
    The Archivable, Deletable, and Ownable traits are always present for plain Records
     (but must be explicitly added to the CustomEntityDefinition to use them).
    More specific base Entity types will be instanced of that base type instead.
    """

    definition: "CustomEntityDefinition" = builtin_property(
        6,
        is_managed=True,
        is_readonly=True,
        description="The CustomEntityDefinition this Record is an instance of.",
    )
    if TYPE_CHECKING:
        definition_ptr: Optional[NodeReference] = None


@builtin_node(NodeType.RESOURCE, is_abstract=True)
class Resource(IsDeletable, IsExtensible, Entity):
    """
    A Resource represents an external asset outside of Destack.
    The lifecycle of a Resource may be managed by some Provisioner (Service).
    """

    status: ResourceStatus = builtin_property(90, default=ResourceStatus.PENDING)


@builtin_node(NodeType.METRIC, is_abstract=True)
class Metric(IsSpatial, IsSourceable, Entity):
    """An Entity that represents a Metric."""

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)


@builtin_enum(EnumType.SNAPSHOT_TYPE)
class SnapshotType(Enum):
    """The type of a Snapshot."""

    PARTIAL = 1, "Partial", "A partial Snapshot (partial/full Nodes, partial Graph)"
    FULL = 2, "Full", "A full Snapshot (full Nodes, full Graph)"


@builtin_enum(EnumType.SNAPSHOT_STATUS)
class SnapshotStatus(Enum):
    """The status of a Snapshot."""

    CREATING = 1
    ACTIVE = 10
    READONLY = 50


@builtin_node(NodeType.SNAPSHOT)
class Snapshot(
    IsSpatial,
    IsOwnable,
    IsArchivable,
    IsDeletable,
    Entity,
):
    """
    A Snapshot is a point in Space time.
    Snapshots cannot be instanced, and they cannot be part of any other Snapshot.
    """

    parent: Union["Space", "Snapshot", None] = builtin_property_parent(is_readonly=True)

    snapshot: "Snapshot" = builtin_property(
        11,
        is_readonly=True,
        is_managed=True,
        node_space_from="self",
        default_factory=ValueFactory.SELF,
        description="The Snapshot itself. Cannot be any other Snapshot than this Snapshot",
    )
    predecessor: Optional["Snapshot"] = builtin_property(
        12,
        is_readonly=True,
        is_managed=True,
        node_space_from="self",
        description="The previous Snapshot this Snapshot is based on.",
    )

    type: SnapshotType = builtin_property(
        100,
        is_readonly=True,
        default=SnapshotType.PARTIAL,
    )
    name: str = builtin_property(101, is_repr=True)

    status: SnapshotStatus = builtin_property(110, default=SnapshotStatus.ACTIVE)

    _token: Any | None = builtin_property_runtime()

    def into(
        self,
        snapshot: "Snapshot",
        *,
        materialization: Materialization = Materialization.FULL,
    ) -> "Self":
        if snapshot.id == self.id:
            return self
        else:
            raise RuntimeError(f"cannot turn {self!r} into another Snapshot ({snapshot!r})")

    def __enter__(self) -> "Self":
        assert self._token is None, f"already in {self!r}"
        self._token = ACTIVE_SNAPSHOT.set(self)
        return self

    def __exit__(self, exc_type, exc_value, traceback) -> None:
        assert self._token is not None, f"not in {self!r}"
        ACTIVE_SNAPSHOT.reset(self._token)
        self._token = None
