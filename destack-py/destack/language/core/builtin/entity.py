from collections.abc import Generator, Sequence
from contextlib import contextmanager
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Optional,
    Self,
    Union,
    cast,
)

from destack.language.registry import NODE_CLASS_BY_TYPE
from destack.utils.fractional import get_order_key

from .common import EnumType, ResourceStatus, StoreDomain, TraitType, ValueFactory
from .const import ACTIVE_SNAPSHOT, UNSET
from .enum import Enum, builtin_enum
from .node import Node, NodeType, builtin_node
from .property import (
    PropertyDeclaration,
    builtin_property,
    builtin_property_parent,
    builtin_property_runtime,
)
from .trait import (
    IsExtensible,
    IsOrdered,
    IsOwnable,
    IsSourceable,
)

if TYPE_CHECKING:
    from destack.language import (
        EntityGraph,
        EntitySingletonGraph,
        Icon,
        IsActor,
        NodeReference,
        Space,
    )

# pyright: reportIncompatibleVariableOverride=false

type_ = type
object_set_ = object.__setattr__


@builtin_enum(EnumType.MATERIALIZATION)
class Materialization(Enum):
    """
    The materialization level of an Entity.

    """

    PARTIAL = 1
    FULL = 10
    ROOT = 11


@builtin_node(
    NodeType.ENTITY,
    is_abstract=True,
    event_types=(NodeType.EDIT_EVENT,),
)
class Entity(Node):
    """
    An Entity is a named, versioned, mutable Node.
    Entities can be attached to (most) other Entities to compose richer structures.

    Entities are always part of a Snapshot (in their Space).
    State transition can only be caused by Events (which are immutable).

    The specific version of an Entity is identified by an (id, snapshot_id) tuple,
     where Snapshots are 'shortcuts' to certain epochs.
    """

    __store_domain__ = StoreDomain.ENTITY
    __parent_property__: ClassVar[PropertyDeclaration] = UNSET

    parent: Optional["Entity"] = builtin_property_parent(
        description="The parent of this Entity. Most Entities can be attached to any other Entity."
    )

    # 10-20: entity materialization
    materialization: Materialization = builtin_property(
        10,
        is_managed=True,
        is_eq=False,
        is_hash=False,
        default=Materialization.ROOT,
    )
    snapshot: "Snapshot" = builtin_property(
        11,
        is_readonly=True,
        is_managed=True,
        is_eq=False,
        is_hash=False,
        default_factory=ValueFactory.SNAPSHOT,
        description="The Snapshot this Entity is part of.",
    )
    preceded_by: Optional[Self] = builtin_property(
        12,
        is_readonly=True,
        is_managed=True,
        is_eq=False,
        is_hash=False,
        description="The previous Entity this Entity is based on (from the base Snapshot).",
    )
    instantiation_root: Optional["Entity"] = builtin_property(
        14,
        is_readonly=True,
        is_managed=True,
        is_eq=False,
        is_hash=False,
        description="The (root) Entity that is being instantiated.",
    )
    # set_properties: 15
    if TYPE_CHECKING:
        snapshot_ptr: Optional["NodeReference"] = None
        preceded_by_ptr: Optional["NodeReference"] = None
        instantiation_root_ptr: Optional["NodeReference"] = None

    # 20-40: node tracking
    created_at: datetime = builtin_property(
        20,
        is_managed=True,
        is_eq=False,
        is_readonly=True,
        description="The time this Entity was created (system time).",
    )
    created_epoch: int = builtin_property(
        21,
        is_managed=True,
        is_eq=False,
        is_hash=False,
        description="The logical time this Entity was created (system time).",
    )
    created_by: Optional["IsActor"] = builtin_property(
        22,
        default=None,
        is_managed=True,
        is_eq=False,
        is_readonly=True,
        description="The Actor that created this Entity.",
    )
    updated_at: datetime = builtin_property(
        23,
        is_managed=True,
        is_eq=False,
        description="The time this Entity was last updated (system time).",
    )
    updated_epoch: int = builtin_property(
        24,
        is_managed=True,
        is_eq=False,
        is_hash=False,
        description="The logical time this Entity was last updated (system time).",
    )
    updated_by: Optional["IsActor"] = builtin_property(
        25,
        default=None,
        is_managed=True,
        is_eq=False,
        description="The Actor that last updated this Entity.",
    )
    deleted_at: Optional[datetime] = builtin_property(
        26,
        is_managed=True,
        is_eq=False,
        description="The time this Entity was deleted (system time).",
    )
    if TYPE_CHECKING:
        created_by_ptr: Optional[NodeReference] = None
        updated_by_ptr: Optional[NodeReference] = None

    name: str = builtin_property(
        50,
        is_repr=True,
        default_factory=ValueFactory.NAME,
    )

    """The specific Graph this Entity is part of."""
    _graph: "EntityGraph | EntitySingletonGraph" = builtin_property_runtime(default=None)

    def _do_set(self, key: str, value: Any):
        """Set a Property on this Node (direct SET/CLEAR operations)."""
        prop = self.__tracked_properties__.get(key)
        if prop is not None and not self._is_new:
            self._session.update_set_property(self, prop, value)
        object_set_(self, key, value)

    if not TYPE_CHECKING:
        __setattr__ = _do_set

    def delete(self):
        """Delete this Entity."""
        assert not self.deleted_at, f"{self!r} is already deleted"
        self._session.delete(self)

    def restore(self):
        """Restore this deleted Entity from the trash."""
        assert self.deleted_at, f"{self!r} is not deleted"
        self._session.restore(self)

    def _assign_order(
        self,
        child: "Entity",
        after: "Entity | None" = None,
        before: "Entity | None" = None,
        *,
        _existing_nodes: Sequence["Entity"] | None = None,
    ):
        """Assign an order key to a child Entity."""
        order_base = next(
            (
                node_type
                for node_type in child.__inherits__
                if TraitType.OWNED in NODE_CLASS_BY_TYPE[node_type].__traits__
            ),
            child.metatype,
        )
        if _existing_nodes is None:
            _existing_nodes = cast(
                Sequence[Entity], self._graph.get_children(self, type=order_base)
            )
        if _existing_nodes:
            if after is None:
                after = _existing_nodes[-1]
            after_order_key = after.order_key if isinstance(after, IsOrdered) else None
            if (
                isinstance(before, IsOrdered)
                and after_order_key is not None
                and before.order_key > after_order_key
            ):
                before_order_key = before.order_key
            else:
                before_order_key = None
            assert isinstance(child, Entity), f"{child!r} is not an Entity"
            order_key = get_order_key(after_order_key, before_order_key)
            child._do_set("order_key", order_key)

    def detach(self):
        """
        Detach this Entity from its parent. Error if it has no parent.
        Does not delete or archive the Entity, just removes it from that tree.
        """
        if self.parent_ptr is None:
            raise ValueError(f"{self!r} has no parent to detach from")
        self.move_to(parent=None)

    def move_to(
        self,
        parent: "Entity | None",
        *,
        after: "Entity | None" = None,
        before: "Entity | None" = None,
    ):
        """
        Move this Entity to a new parent Entity.
        If the Entity IsOrdered, it will be positioned (relative to after/before).
        If the Entity is new, it will be automatically created in this Entity's Session (for convenience).
        (The same applies to all descendants.)
        """
        from ..runtime import EntitySingletonGraph

        # prepare graph & nodes
        supergraph = self._supergraph
        old_graph = self._graph
        if parent is not None:
            # move to new parent
            assert old_graph.supergraph is parent._supergraph, (
                f"{self!r} is not in supergraph of {parent!r}"
            )
            if not isinstance(parent, self.__parent_classes__):
                raise ValueError(
                    f"{parent!r} cannot parent {self!r} (allowed: {self.__parent_types__})"
                )
            if self.space_ptr.id != parent.space_ptr.id:
                raise ValueError(f"cannot move {self!r} to {parent!r} (different Space)")
            new_graph = parent._graph
            parent_ptr = parent.to_ref()
            # promote parent to polygraph if needed
            if isinstance(new_graph, EntitySingletonGraph):
                new_graph = supergraph.promote_to_polygraph(new_graph)
                parent._graph = new_graph
        else:
            # detach from parent
            if self.parent_ptr is None:
                return  # nothing to do
            new_graph = supergraph.create_entity_graph()
            parent_ptr = None
        session = self._session
        nodes: tuple[Entity, ...] = (self, *self._graph.get_descendants(self))

        # assign order
        if parent is not None and isinstance(self, IsOrdered):
            parent._assign_order(self, after=after, before=before)

        # move to new graph
        self.parent_ptr = parent_ptr
        if old_graph is not new_graph:
            if len(nodes) == len(old_graph):  # all nodes were moved
                supergraph.remove_graph(old_graph)
            else:
                for node in nodes:
                    old_graph.remove(node)
            for node in nodes:
                node._graph = new_graph
                new_graph.add(node)

        # create new nodes
        if self._is_new and parent is not None and not parent._is_new:
            for node in nodes:
                assert isinstance(node, Entity), f"{node!r} of {parent!r} is not an Entity"
                node._ref = None  # invalidate cached ref
                session.create(node)

    def add_sibling(
        self,
        sibling: "Entity",
        *,
        after: "Entity | None" = None,
        before: "Entity | None" = None,
    ) -> Self:
        """
        Add an Entity as a sibling of this Entity.
        If the Entity IsOrdered, it will be positioned (relative to after/before).
        If the Entity is new, it will be automatically created in this Entity's Session (for convenience).
        (The same applies to all descendants.)
        """
        sibling.move_to(self.parent, after=after, before=before)
        return self

    def add_siblings(
        self,
        *siblings: "Entity",
        after: "Entity | None" = None,
        before: "Entity | None" = None,
    ) -> Self:
        """Add multiple Entities as siblings of this Entity."""
        for sibling in siblings:
            sibling.move_to(self.parent, after=after, before=before)
        return self

    def add_child(
        self,
        child: "Entity",
        *,
        after: "Entity | None" = None,
        before: "Entity | None" = None,
    ) -> Self:
        """
        Append an Entity as a child of this Entity (and all its descendants).
        If the Entity IsOrdered, it will be positioned (relative to after/before).
        If the Entity is new, it will be automatically created in this Entity's Session (for convenience).
        (The same applies to all descendants.)
        """
        child.move_to(self, after=after, before=before)
        return self

    def add_children(
        self,
        *children: "Entity",
        after: "Entity | None" = None,
        before: "Entity | None" = None,
    ) -> Self:
        """
        Append multiple Entities as children of this Entity.
        """
        for child in children:
            child.move_to(self, after=after, before=before)
        return self

    def remove_child(self, child: "Entity") -> Self:
        """
        Remove a child Entity from this Entity.
        The child will NOT be deleted, it will simply be detached.
        """
        child.detach()
        return self

    def add_tag(self, tag: "Tag") -> "Tagging":
        """Add or get a Tagging for a Tag on this Entity."""
        for tagging in self.get_children(Tagging):
            if tagging.tag.id == tag.id:
                return tagging
        else:
            # create new Tagging
            tagging = Tagging(tag=tag)
            self.add_child(tagging)
            return tagging

    def remove_tag(self, tag: "Tag") -> "Tagging | None":
        """Remove a Tag from this Entity."""
        for tagging in self.get_children(Tagging):
            if tagging.tag_ptr.id == tag.id:
                self.remove_child(tagging)
                return tagging
        else:
            return None

    def get_children[N: Entity = Entity](
        self,
        type: NodeType | TraitType | type[N] | None = None,
        include_deleted: bool = False,
    ) -> Sequence[N]:
        """Gets the children of this Entity."""
        return self._graph.get_children(self, type=type)

    def get_child[N: Entity = Entity](
        self,
        type: NodeType | type[N] | TraitType,
        name: str,
        include_deleted: bool = False,
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
    ) -> Sequence[N]:
        """Gets the ancestors of this Node."""
        return self._graph.get_ancestors(self, type=type)

    def get_descendants[N: Entity = Entity](
        self,
        type: NodeType | TraitType | type[N] | None = None,
        include_deleted: bool = False,
    ) -> Sequence[N]:
        """Gets the descendants of this Node."""
        return self._graph.get_descendants(self, type=type)

    def get_roots[N: Entity = Entity](
        self,
        type: NodeType | TraitType | type[N] | None = None,
        include_deleted: bool = False,
    ) -> Sequence[N]:
        """Gets the roots of this Node."""
        return self._graph.get_roots(node_type=type)

    def get_leaves[N: Entity = Entity](
        self,
        type: NodeType | TraitType | type[N] | None = None,
        include_deleted: bool = False,
    ) -> Sequence[N]:
        """Gets the leaves of this Node."""
        return self._graph.get_leaves(node_type=type, node=self)

    def into(self, snapshot: "Snapshot") -> "Self":
        """
        Turn this Entity into its corresponding Entity in the given Snapshot.
        """
        raise NotImplementedError

    def instantiate(self) -> "Self":
        """Instantiate this Entity into a new Entity."""
        raise NotImplementedError


@builtin_node(NodeType.RECORD, is_abstract=True)
class Record(
    IsExtensible,
    IsOwnable,
    Entity,
):
    """
    A generic Record instance of a CustomEntity.
    """

    pass


@builtin_node(NodeType.RESOURCE, is_abstract=True)
class Resource(
    IsExtensible,
    IsOwnable,
    Entity,
):
    """
    A Resource represents an external asset outside of Destack.
    The lifecycle of a Resource may be managed by some Provisioner (Service).
    """

    status: ResourceStatus = builtin_property(40, default=ResourceStatus.PENDING)


@builtin_enum(EnumType.SNAPSHOT_TYPE)
class SnapshotType(Enum):
    """The type of a Snapshot."""

    PARTIAL = 1
    FULL = 10
    ROOT = 11


@builtin_enum(EnumType.SNAPSHOT_STATUS)
class SnapshotStatus(Enum):
    """The status of a Snapshot."""

    CREATING = 1
    ACTIVE = 10
    READONLY = 50


@builtin_node(NodeType.SNAPSHOT)
class Snapshot(
    IsOwnable,
    Entity,
):
    """
    A Snapshot is a point in Space-time.
    Snapshots may branch off of other Snapshots, either as a full copy or a partial override.

    To avoid breaking the universe, Snapshots cannot themselves be part of any other Snapshot.
     (Technically, Snapshots are part of themselves.)
    """

    parent: Union["Space", None] = builtin_property_parent(is_readonly=True)

    snapshot: "Snapshot" = builtin_property(
        11,
        is_readonly=True,
        is_managed=True,
        default_factory=ValueFactory.SELF,
        description="The Snapshot itself. Cannot be any other Snapshot than this Snapshot",
    )
    preceded_by: Optional["Snapshot"] = builtin_property(
        12,
        is_readonly=True,
        is_managed=True,
        description="The previous Snapshot this Snapshot is based on.",
    )

    type: SnapshotType = builtin_property(100)
    status: SnapshotStatus = builtin_property(110, default=SnapshotStatus.ACTIVE)

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

    @contextmanager
    def active(self) -> Generator[None, None, None]:
        """
        Context manager to set the active Snapshot to this Snapshot.
        """
        token = ACTIVE_SNAPSHOT.set(self)
        try:
            yield
        finally:
            ACTIVE_SNAPSHOT.reset(token)


@builtin_node(NodeType.VARIANT, is_abstract=True)
class Variant(
    IsExtensible,
    IsOwnable,
    Entity,
):
    """A Variant is an alternative version of an Entity."""

    parent: Optional["IsExtensible"] = builtin_property_parent()
    icon: "Icon | None" = builtin_property(102)


@builtin_node(NodeType.TAG)
class Tag(
    IsSourceable,
    IsExtensible,
    Entity,
):
    """A Tag to tag an Entity with (in a Tagging)."""

    icon: "Icon | None" = builtin_property(102)


@builtin_node(NodeType.TAGGING)
class Tagging(
    IsOrdered,
    Entity,
):
    """A Tagging of a Node by a Tag."""

    tag: Tag = builtin_property(110)
    if TYPE_CHECKING:
        tag_ptr: NodeReference = UNSET
