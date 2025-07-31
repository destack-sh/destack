from collections.abc import Sequence
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Optional,
    Self,
    cast,
    dataclass_transform,
)

from destack.language.registry import NODE_CLASS_BY_TYPE
from destack.utils.fractional import INTEGER_ZERO, get_order_key
from destack.utils.uuid import UUID

from .builtin import EnumType, NodeType, ObjectKind, StructType, TraitType
from .common import UInt128, ValueFactory
from .const import UNSET
from .declaration import (
    ConstraintDeclaration,
    IndexDeclaration,
    PermissionDeclaration,
    TagDeclaration,
    builtin_method,
)
from .enum import Enum, builtin_enum
from .node import Node, _process_node_cls
from .property import (
    _PROPERTY_SPECIFIERS,
    PropertyDeclaration,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import (
        Branch,
        NodeReference,
        Script,
        Session,
        Snapshot,
        Tag,
        Tagging,
        Value,
    )

# pyright: reportIncompatibleVariableOverride=false

type_ = type
object_set_ = object.__setattr__


@builtin_enum(EnumType.MATERIALIZATION)
class Materialization(Enum):
    """
    The materialization level of an Entity.
    """

    VIRTUAL = 1, "Virtual", "Entity matches its definition, only exists when queried"
    PARTIAL = 2, "Partial", "Entity is a partial override of its definition"
    FULL = 3, "Full", "Entity is a full copy of its definition"
    ROOT = 4, "Root", "Entity is its own root (no other definition)"


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def builtin_entity(
    # meta
    entity_type: NodeType,
    *,
    is_abstract: bool = False,
    is_final: bool = False,
    is_singleton: bool = False,
    # inheritance
    traits: tuple[TraitType, ...] = (),
    # content
    indexes: tuple["IndexDeclaration", ...] = (),
    constraints: tuple["ConstraintDeclaration", ...] = (),
    permissions: tuple["PermissionDeclaration", ...] = (),
    tags: tuple["TagDeclaration", ...] = (),
    # tree
    expected_parent_types: tuple[NodeType, ...] = (),
    expected_child_types: tuple[NodeType, ...] = (),
    expected_ancestor_types: tuple[NodeType, ...] = (),
    expected_descendant_types: tuple[NodeType, ...] = (),
    # associations
    event_types: tuple[NodeType, ...] = (),
    enum_types: tuple[EnumType, ...] = (),
    message_types: tuple[StructType, ...] = (),
):
    """Register a class as a concrete Entity for the given Node type."""

    def decorate(cls: type) -> type:
        cls = _process_node_cls(
            # meta
            cls=cls,
            node_type=entity_type,
            is_abstract=is_abstract,
            is_final=is_final,
            is_singleton=is_singleton,
            is_frozen=False,
            # inheritance
            traits=traits,
            # content
            indexes=indexes,
            constraints=constraints,
            permissions=permissions,
            tags=tags,
            # tree
            expected_parent_types=expected_parent_types,
            expected_child_types=expected_child_types,
            expected_ancestor_types=expected_ancestor_types,
            expected_descendant_types=expected_descendant_types,
            # associations
            event_types=event_types,
            enum_types=enum_types,
            message_types=message_types,
        )
        assert entity_type == NodeType.ENTITY or NodeType.ENTITY in cls.__declaration__.inherits, (
            f"Entity {cls.__name__} must inherit from Entity"
        )
        return cls

    return decorate


@builtin_entity(
    NodeType.ENTITY,
    is_abstract=True,
    event_types=(NodeType.EDIT_EVENT,),
    permissions=(PermissionDeclaration(id=20, name="create", description="Create (or Upsert)"),),
    tags=(
        TagDeclaration(id=20, name="entity", description="Entity"),
        TagDeclaration(id=21, name="source", description="Source"),
        TagDeclaration(id=30, name="visibility", description="Visibility"),
        TagDeclaration(id=31, name="style", description="style"),
        TagDeclaration(id=32, name="transform", description="Transform"),
        TagDeclaration(id=33, name="size", description="Size"),
        TagDeclaration(id=34, name="layout", description="Layout"),
    ),
)
class Entity(Node):
    """
    An Entity is a named, versioned, mutable Node.
    Entities can be attached to (most) other Entities to compose richer structures.

    Updates to Entities can only be affected through Events.
    Entities are always part of a Snapshot (in their Space).

    An instance of an Entity is identified by an (id, branch_id, snapshot_id) tuple,
     where Snapshots are 'shortcuts' to certain epochs.
     (id, definition_id) @ (branch_id, snapshot_id)
    """

    metakind = ObjectKind.NODE
    __parent_property__: ClassVar[PropertyDeclaration] = UNSET

    parent: Optional["Entity"] = builtin_property_parent(
        description="The parent of this Entity. Most Entities can be attached to any other Entity."
    )
    if TYPE_CHECKING:
        parent_ptr: Optional[NodeReference] = None

    # 1-20: identity
    materialization: Materialization = builtin_property(
        10,
        is_internal=True,
        is_eq=False,
        is_hash=False,
        is_identity=True,
        default=Materialization.ROOT,
        tags=("identity",),
    )
    definition: Optional["Entity"] = builtin_property(
        11,
        is_internal=True,
        is_readonly=True,
        is_identity=True,
        description="The definition this Entity is an instance of.",
        tags=("identity",),
    )
    branch: "Branch" = builtin_property(
        12,
        is_readonly=True,
        is_internal=True,
        is_eq=False,
        is_hash=False,
        is_identity=True,
        default_factory=ValueFactory.BRANCH,
        description="The Branch this Entity is part of.",
        tags=("identity",),
    )
    snapshot: "Snapshot" = builtin_property(
        13,
        is_readonly=True,
        is_internal=True,
        is_eq=False,
        is_hash=False,
        is_identity=True,
        default_factory=ValueFactory.SNAPSHOT,
        description="The Snapshot this Entity is part of.",
        tags=("identity",),
    )
    preceded_by: Optional[Self] = builtin_property(
        14,
        is_readonly=True,
        is_internal=True,
        is_eq=False,
        is_hash=False,
        is_identity=True,
        description="""\
The previous Entity this Entity is based on (from the base Branch, if any).
This invariant must hold: `Entity.preceded_by.branch == Entity.branch.preceded_by`.
""",
        tags=("identity",),
    )
    instance: Optional["Entity"] = builtin_property(
        15,
        is_readonly=True,
        is_internal=True,
        is_eq=False,
        is_hash=False,
        is_identity=True,
        description="The (root) Entity that is being instantiated.",
        tags=("identity",),
    )
    if TYPE_CHECKING:
        definition_ptr: Optional["NodeReference"] = None
        branch_ptr: NodeReference = UNSET
        snapshot_ptr: NodeReference = UNSET
        preceded_by_ptr: Optional[NodeReference] = None
        instance_ptr: Optional[NodeReference] = None

    # 20-40: Entity tracking
    created_at: datetime = builtin_property(
        20,
        is_internal=True,
        is_eq=False,
        is_readonly=True,
        default_factory=ValueFactory.NOW,
        description="The time this Entity was created (system time).",
        tags=("tracking",),
    )
    created_epoch: UInt128 = builtin_property(
        21,
        is_internal=True,
        is_eq=False,
        is_hash=False,
        default_factory=ValueFactory.REMOTE_EPOCH,
        description="The logical time this Entity was created (system time).",
        tags=("tracking",),
    )
    created_by: "Entity" = builtin_property(
        22,
        is_internal=True,
        is_eq=False,
        is_readonly=True,
        default_factory=ValueFactory.ACTOR,
        description="The Actor that created this Entity.",
        tags=("tracking",),
    )
    updated_at: datetime = builtin_property(
        23,
        is_internal=True,
        is_eq=False,
        default_factory=ValueFactory.NOW,
        description="The time this Entity was last updated (system time).",
        tags=("tracking",),
    )
    updated_epoch: UInt128 = builtin_property(
        24,
        is_internal=True,
        is_eq=False,
        is_hash=False,
        default_factory=ValueFactory.REMOTE_EPOCH,
        description="The logical time this Entity was last updated (system time).",
        tags=("tracking",),
    )
    updated_by: "Entity" = builtin_property(
        25,
        is_internal=True,
        is_eq=False,
        default_factory=ValueFactory.ACTOR,
        description="The Actor that last updated this Entity.",
        tags=("tracking",),
    )
    deleted_at: Optional[datetime] = builtin_property(
        26,
        is_internal=True,
        is_eq=False,
        description="""\
The time this Entity was deleted (system time).
Only set if the Entity is currently 'deleted'.
Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
""",
        tags=("tracking",),
    )
    owned_by: Optional["Entity"] = builtin_property(
        30,
        is_repr=True,
        tags=("tracking",),
    )
    # controlled_by, ...
    if TYPE_CHECKING:
        created_by_ptr: NodeReference = UNSET
        updated_by_ptr: NodeReference = UNSET
        owned_by_ptr: Optional[NodeReference] = None

    # 40-60: Entity basics
    name: str = builtin_property(
        40,
        is_repr=True,
        default_factory=ValueFactory.NAME,
        tags=("entity",),
    )
    order_key: str = builtin_property(
        41,
        is_eq=False,
        is_internal=True,
        default=INTEGER_ZERO,
        description="The absolute order key of this Entity in its parent.",
        tags=("entity",),
    )
    # key: str | None = builtin_property(
    #     42,
    #     description="The key to uniquely identify this Entity in reconciliation. If not set, name is used.",
    #     tags=("source",),
    # )
    custom_values: dict[str, "Value"] | None = builtin_property(
        45,
        description="The custom Values of this Entity, keyed by custom Property name.",
        tags=("entity",),
    )
    script: Optional["Script"] = builtin_property(
        46,
        description="The Script of this Entity.",
        tags=("entity",),
    )
    is_extensible: bool | None = builtin_property(
        50,
        is_internal=True,
        is_readonly=True,
        description="Whether this Entity can be instanced.",
        tags=("entity",),
    )
    # base_type?
    # traits?
    # is_trait? is_abstract?
    # is_locked/is_final?
    # is_singleton?

    # 80-100: provenance
    source: Optional["Script"] = builtin_property(
        80,
        is_internal=True,
        description="The Script that defines this Node.",
        tags=("source",),
    )
    # ... (from script, dynamic effect, manual function, import, ...)

    def set(self, key: str, value: Any):
        """Set a Property on this Node (direct SET operations)."""
        prop = self.__properties_by_alias__.get(key)
        if prop is not None and not self._is_new:
            self._session.update_set_property(self, prop, value)
        object_set_(self, key, value)

    if not TYPE_CHECKING:
        __setattr__ = set

    @builtin_method(10)
    @property
    def is_custom(self) -> bool:
        """Whether this Node is a custom Node."""
        return self.definition is not None

    @builtin_method(11)
    @property
    def is_partial(self) -> bool:
        """Whether this Entity is a partial Entity."""
        return False  # only set in EntityPartial

    @builtin_method(20)
    def delete(self):
        """Delete this Entity."""
        assert not self.deleted_at, f"{self!r} is already deleted"
        self._session.delete(self)

    @builtin_method(21)
    def restore(self):
        """Restore this deleted Entity from the trash."""
        assert self.deleted_at, f"{self!r} is not deleted"
        self._session.restore(self)

    @builtin_method(30)
    def get_children[N: Entity = Entity](
        self,
        type: NodeType | type[N] | None = None,
        include_deleted: bool = False,
    ) -> Sequence[N]:
        """Gets the children of this Entity."""
        if isinstance(type, type_):
            type = type.metatype
        children = self._session.graph.get_children(
            self,
            self.space_ptr.id,
            self.branch_ptr.id,
            self.snapshot_ptr.id,
            type,
            include_deleted,
        )
        return cast(Sequence[N], children)

    @builtin_method(31)
    def get_child[N: Entity = Entity](
        self,
        type: NodeType | type[N],
        name: str,
        include_deleted: bool = False,
    ) -> N | None:
        """Gets a specific child of this Node by name."""
        if isinstance(type, type_):
            type = type.metatype
        for child in self._session.graph.get_children(
            self,
            self.space_ptr.id,
            self.branch_ptr.id,
            self.snapshot_ptr.id,
            type,
            include_deleted,
        ):
            if getattr(child, "name", None) == name:
                return cast(N, child)
        return None

    @builtin_method(32)
    def child[N: Entity = Entity](
        self,
        type: NodeType | type[N],
        name: str,
        include_deleted: bool = False,
    ) -> N:
        """Gets a specific child of this Node by name, or raises an error if not found."""
        child = self.get_child(type, name, include_deleted)
        if child is None:
            raise LookupError(f"no child {name} of {self!r}")
        return cast(N, child)

    @builtin_method(33)
    def get_ancestors[N: Entity = Entity](
        self,
        type: NodeType | type[N] | None = None,
        include_deleted: bool = False,
    ) -> Sequence[N]:
        """Gets the ancestors of this Node."""
        if isinstance(type, type_):
            type = type.metatype
        ancestors = self._session.graph.get_ancestors(
            self,
            self.space_ptr.id,
            self.branch_ptr.id,
            self.snapshot_ptr.id,
            type,
            include_deleted,
        )
        return cast(Sequence[N], ancestors)

    @builtin_method(34)
    def get_descendants[N: Entity = Entity](
        self,
        type: NodeType | type[N] | None = None,
        include_deleted: bool = False,
    ) -> Sequence[N]:
        """Gets the descendants of this Node."""
        if isinstance(type, type_):
            type = type.metatype
        descendants = self._session.graph.get_descendants(
            self,
            self.space_ptr.id,
            self.branch_ptr.id,
            self.snapshot_ptr.id,
            type,
            include_deleted,
        )
        return cast(Sequence[N], descendants)

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
                for node_type in child.__definition__.inherits
                if TraitType.OWNED in NODE_CLASS_BY_TYPE[node_type].__definition__.traits
            ),
            child.metatype,
        )
        if _existing_nodes is None:
            _existing_nodes = cast(
                Sequence[Entity],
                self._session.graph.get_children(
                    self, self.space_ptr.id, self.branch_ptr.id, self.snapshot_ptr.id, order_base
                ),
            )
        if _existing_nodes:
            if after is None:
                after = _existing_nodes[-1]
            after_order_key = after.order_key
            if (
                after_order_key is not None
                and before is not None
                and before.order_key > after_order_key
            ):
                before_order_key = before.order_key
            else:
                before_order_key = None
            assert isinstance(child, Entity), f"{child!r} is not an Entity"
            order_key = get_order_key(after_order_key, before_order_key)
            child.set("order_key", order_key)

    @builtin_method(40)
    def detach(self):
        """
        Detach this Entity from its parent.
        Does not delete the Entity, just removes it from its parent.
        Raises an error if it has no parent.
        """
        if self.parent_ptr is None:
            raise ValueError(f"{self!r} has no parent to detach from")
        self.move_to(parent=None)

    @builtin_method(41)
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

        # prepare graph & nodes
        session = self._session
        nodes: tuple[Entity, ...] = (
            self,
            *self._session.graph.get_descendants(
                self, self.space_ptr.id, self.branch_ptr.id, self.snapshot_ptr.id
            ),
        )

        # assign parent & order
        if parent is not None:
            self.parent_ptr = parent.to_ref()
            if TraitType.ORDERED in self.__definition__.traits:
                parent._assign_order(self, after, before)

        # create new nodes
        if self._is_new and parent is not None and not parent._is_new:
            for node in nodes:
                assert isinstance(node, Entity), f"{node!r} of {parent!r} is not an Entity"
                session.create(node)

    @builtin_method(42)
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

    @builtin_method(43)
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

    @builtin_method(44)
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

    @builtin_method(45)
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

    @builtin_method(46)
    def remove_child(self, child: "Entity") -> Self:
        """
        Remove a child Entity from this Entity.
        The child will NOT be deleted, it will simply be detached.
        """
        child.detach()
        return self

    @builtin_method(50)
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

    @builtin_method(51)
    def remove_tag(self, tag: "Tag") -> "Tagging | None":
        """Remove a Tag from this Entity."""
        for tagging in self.get_children(Tagging):
            if tagging.tag_ptr.id == tag.id:
                self.remove_child(tagging)
                return tagging
        else:
            return None

    @builtin_method(60)
    def into(self, branch: "Branch") -> "Self":
        """
        Turn this Entity into its corresponding Entity in the given Branch.
        """
        raise NotImplementedError

    @builtin_method(61)
    def instantiate(self, *, partial: bool = True, attach: bool = False, **override: Any) -> "Self":
        """
        Instantiate this Entity into a new (partial or full) Entity.
        If partial is True, the new Entity will be a partial Entity with only override set.
        If attach is True, the new Entity will be attached to the current Entity's parent.
        """
        raise NotImplementedError

    @classmethod
    def partial(cls) -> "EntityPartial": ...


ENTITY_MATERIALIZATION_ID = Entity.property("materialization").id
ENTITY_MATERIALIZATION_KEY = str(ENTITY_MATERIALIZATION_ID)


class EntityPartial:
    """
    A partial Entity is an Entity that is not fully materialized,
     but pretends to be a full Entity by deferring to a (chain of) other Entities.
    """

    __slots__ = (
        "_override",
        "_session",
        "branch_ptr",
        "definition_ptr",
        "id",
        "instance_ptr",
        "materialization",
        "metatype",
        "node_cls",
        "preceded_by_ptr",
        "snapshot_ptr",
        "space_ptr",
    )

    def __init__(
        self,
        metatype: NodeType,
        id: UUID,
        space_ptr: "NodeReference",
        materialization: Materialization,
        definition_ptr: "NodeReference | None",
        branch_ptr: "NodeReference",
        snapshot_ptr: "NodeReference",
        preceded_by_ptr: "NodeReference | None",
        instance_ptr: "NodeReference | None",
        _session: "Session",
        _override: dict[str, Any] | None,
    ):
        assert materialization < Materialization.FULL, (
            f"partial Entity must be < Materialization.FULL: {materialization} (id={id})"
        )
        self.metatype = metatype
        self.node_cls = NODE_CLASS_BY_TYPE.get(metatype)
        assert self.node_cls is not None, f"no node class for {metatype}"
        self.id = id
        self.space_ptr = space_ptr
        self.materialization = materialization
        self.definition_ptr = definition_ptr
        self.branch_ptr = branch_ptr
        self.snapshot_ptr = snapshot_ptr
        self.preceded_by_ptr = preceded_by_ptr
        self.instance_ptr = instance_ptr
        self._override = _override
        self._session = _session

    @property
    def definition(self) -> "Entity | None":
        """The definition of this Entity."""
        if self.definition_ptr is None:
            return None
        return self._session.graph.get(
            self.definition_ptr.id,
            self.space_ptr.id,
            self.branch_ptr.id,
            self.snapshot_ptr.id,
        )

    @property
    def is_partial(self) -> bool:
        """Whether this Entity is a partial Entity."""
        return True

    def is_set(self, key: str) -> bool:
        """Whether a Property is set on this Entity partial."""
        return self._override is not None and key in self._override

    def set(self, key: str, value: Any):
        """Set a Property on this Entity partial."""
        raise NotImplementedError

    def unset(self, key: str):
        """Unset a Property on this Entity partial."""
        raise NotImplementedError
