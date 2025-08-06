from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Optional,
    Self,
    dataclass_transform,
)

from destack.registry import NODE_CLASS_BY_TYPE

from ..utility import INTEGER_ZERO, UUID
from .const import UNSET
from .declaration import (
    ConstraintDeclaration,
    IndexDeclaration,
    PermissionDeclaration,
    TagDeclaration,
    declare_method,
)
from .enum import OptionEnum, declare_enum, declare_option
from .hoisted import UInt128, ValueFactory
from .node import Node, _process_node_cls
from .property import (
    _PROPERTY_SPECIFIERS,
    PropertyDeclaration,
    declare_property,
    declare_property_parent,
)
from .universe import EnumType, NodeType, ObjectKind, StructType, TraitType

if TYPE_CHECKING:
    from destack import (
        Branch,
        Icon,
        NodeReference,
        Script,
        Session,
        Snapshot,
        Tag,
        Value,
    )


type_ = type
object_set_ = object.__setattr__


@declare_enum(EnumType.MATERIALIZATION)
class Materialization(OptionEnum):
    """
    The materialization level of an Entity.
    """

    VIRTUAL = declare_option(
        1, description="Entity matches its definition, only exists when queried"
    )
    PARTIAL = declare_option(2, description="Entity is a partial override of its definition")
    FULL = declare_option(3, description="Entity is a full copy of its definition")
    ROOT = declare_option(4, description="Entity is its own root (no other definition)")


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def declare_entity(
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
    base_struct_type: StructType | None = None,
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
            base_struct_type=base_struct_type,
        )
        assert entity_type == NodeType.ENTITY or NodeType.ENTITY in cls.__declaration__.inherits, (
            f"Entity {cls.__name__} must inherit from Entity"
        )
        return cls

    return decorate


@declare_entity(
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

    Updates to Entities are made through Events.
    Entities are always part of a Snapshot (in their Space).
     (Technically, Entities are just a temporary materialization of the Event graph.)

    An instance of an Entity is identified by an (id, branch_id, snapshot_id) tuple,
     where Snapshots are 'shortcuts' to certain epochs:
     (id, definition_id) @ (branch_id, snapshot_id, epoch)

    Custom and context Values are keyed by name for convenience and clarity.
    The name is normalized to a snake_case string.
    """

    metakind = ObjectKind.NODE
    __parent_property__: ClassVar[PropertyDeclaration] = UNSET

    parent: Optional["Entity"] = declare_property_parent(
        description="The parent of this Entity. Most Entities can be attached to any other Entity."
    )

    # 1-20: identity
    materialization: Materialization = declare_property(
        10,
        is_internal=True,
        is_eq=False,
        is_hash=False,
        is_identity=True,
        default=Materialization.ROOT,
        tags=("identity",),
    )
    definition: Optional["Entity"] = declare_property(
        11,
        is_internal=True,
        is_readonly=True,
        is_identity=True,
        description="The definition this Entity is an instance of.",
        tags=("identity",),
    )
    branch: "Branch" = declare_property(
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
    snapshot: "Snapshot" = declare_property(
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
    preceded_by: Optional[Self] = declare_property(
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
    instance: Optional["Entity"] = declare_property(
        15,
        is_readonly=True,
        is_internal=True,
        is_eq=False,
        is_hash=False,
        is_identity=True,
        description="The (root) Entity that is being instantiated.",
        tags=("identity",),
    )

    # 20-40: Entity tracking
    created_at: datetime = declare_property(
        20,
        is_internal=True,
        is_hash=False,
        is_eq=False,
        is_readonly=True,
        default_factory=ValueFactory.NOW,
        description="The time this Entity was created (system time).",
        tags=("tracking",),
    )
    created_epoch: UInt128 = declare_property(
        21,
        is_internal=True,
        is_hash=False,
        is_eq=False,
        default_factory=ValueFactory.REMOTE_EPOCH,
        description="The logical time this Entity was created (system time).",
        tags=("tracking",),
    )
    created_by: "Entity" = declare_property(
        22,
        is_internal=True,
        is_hash=False,
        is_eq=False,
        is_readonly=True,
        default_factory=ValueFactory.ACTOR,
        description="The Actor that created this Entity.",
        tags=("tracking",),
    )
    updated_at: datetime = declare_property(
        23,
        is_internal=True,
        is_hash=False,
        is_eq=False,
        default_factory=ValueFactory.NOW,
        description="The time this Entity was last updated (system time).",
        tags=("tracking",),
    )
    updated_epoch: UInt128 = declare_property(
        24,
        is_internal=True,
        is_hash=False,
        is_eq=False,
        default_factory=ValueFactory.REMOTE_EPOCH,
        description="The logical time this Entity was last updated (system time).",
        tags=("tracking",),
    )
    updated_by: "Entity" = declare_property(
        25,
        is_internal=True,
        is_hash=False,
        is_eq=False,
        default_factory=ValueFactory.ACTOR,
        description="The Actor that last updated this Entity.",
        tags=("tracking",),
    )
    deleted_at: Optional[datetime] = declare_property(
        26,
        is_internal=True,
        is_hash=False,
        is_eq=False,
        description="""\
The time this Entity was last deleted (system time, if it's is currently deleted).
Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
""",
        tags=("tracking",),
    )

    # 40-80: Entity core
    # 40-50: Entity meta
    name: str = declare_property(
        40,
        is_repr=True,
        default_factory=ValueFactory.NAME,
        tags=("entity",),
    )
    icon: "Icon | None" = declare_property(
        41,
        description="The icon of this Entity.",
        tags=("entity",),
    )
    order_key: str = declare_property(
        42,
        is_eq=False,
        is_internal=True,
        default=INTEGER_ZERO,
        description="The absolute order key of this Entity in its parent.",
        tags=("entity",),
    )
    # is_extensible, is_instantiable, ...
    # base_type?
    # traits?
    # is_trait? is_abstract?
    # is_locked/is_final?
    # is_singleton?

    # 50-60: Entity state
    owned_by: "Entity" = declare_property(
        50,
        is_repr=True,
        description="The exclusive owner of this Entity.",
        default_factory=ValueFactory.ACTOR,
        tags=("tracking",),
    )
    custom_values: dict[str, "Value"] | None = declare_property(
        55,
        description="The custom Values of this Entity, keyed by custom Property or Tag name.",
        tags=("entity",),
    )
    context_values: dict[str, "Value"] | None = declare_property(
        56,
        description="The context Values provided by this Entity, keyed by context Property name.",
        tags=("entity",),
    )

    # 60-70: Entity behavior
    script: Optional["Script"] = declare_property(
        60,
        description="The Script of this Entity.",
        tags=("entity",),
    )
    # key? (for reconciliation)

    # 80-100: provenance
    source: Optional["Script"] = declare_property(
        80,
        is_internal=True,
        description="The Script that defines this Node.",
        tags=("source",),
    )
    # ... (from script, dynamic effect, manual function, import, ...)

    @declare_method(2)
    def to_ref(self) -> "NodeReference":
        """Gets a reference to this Node."""
        ...

    @declare_method(3)
    def set(self, key: str, value: Any):
        """Set a Property on this Node (direct SET operations)."""
        ...

    if not TYPE_CHECKING:
        __setattr__ = set

    @declare_method(10)
    @property
    def is_custom(self) -> bool:
        """Whether this Node is a custom Node."""
        ...

    @declare_method(11)
    @property
    def is_partial(self) -> bool:
        """Whether this Entity is a partial Entity."""
        ...

    @declare_method(20)
    def delete(self):
        """Delete this Entity."""
        ...

    @declare_method(21)
    def restore(self):
        """Restore this deleted Entity from the trash."""
        ...

    @declare_method(30)
    def get_children(
        self,
        type: NodeType | None = None,
        include_deleted: bool = False,
    ) -> list["Entity"]:
        """Gets the children of this Entity."""
        ...

    @declare_method(31)
    def get_child(
        self,
        type: NodeType,
        name: str,
        include_deleted: bool = False,
    ) -> "Entity | None":
        """Gets a specific child of this Node by name."""
        ...

    @declare_method(32)
    def child(
        self,
        type: NodeType,
        name: str,
        include_deleted: bool = False,
    ) -> "Entity":
        """Gets a specific child of this Node by name, or raises an error if not found."""
        ...

    @declare_method(33)
    def get_ancestors(
        self,
        type: NodeType | None = None,
        include_deleted: bool = False,
    ) -> list["Entity"]:
        """Gets the ancestors of this Node."""
        ...

    @declare_method(34)
    def get_descendants(
        self,
        type: NodeType | None = None,
        include_deleted: bool = False,
    ) -> list["Entity"]:
        """Gets the descendants of this Node."""
        ...

    @declare_method(40)
    def detach(self):
        """
        Detach this Entity from its parent.
        Does not delete the Entity, just removes it from its parent.
        Raises an error if it has no parent.
        """
        ...

    @declare_method(41)
    def move_to(
        self,
        parent: "Entity | None",
        *,
        after: "Entity | None" = None,
        before: "Entity | None" = None,
    ):
        """
        Move this Entity to a new parent Entity.
        If the Entity is new, it will be automatically created in this Entity's Session (for convenience).
        (The same applies to all descendants.)
        """
        ...

    @declare_method(42)
    def add_sibling(
        self,
        sibling: "Entity",
        *,
        after: "Entity | None" = None,
        before: "Entity | None" = None,
    ) -> Self:
        """
        Add an Entity as a sibling of this Entity.
        If the Entity is new, it will be automatically created in this Entity's Session (for convenience).
        (The same applies to all descendants.)
        """
        ...

    @declare_method(43)
    def add_siblings(
        self,
        *siblings: "Entity",
        after: "Entity | None" = None,
        before: "Entity | None" = None,
    ) -> Self:
        """Add multiple Entities as siblings of this Entity."""
        ...

    @declare_method(44)
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
        ...

    @declare_method(45)
    def add_children(
        self,
        *children: "Entity",
        after: "Entity | None" = None,
        before: "Entity | None" = None,
    ) -> Self:
        """
        Append multiple Entities as children of this Entity.
        """
        ...

    @declare_method(46)
    def remove_child(self, child: "Entity") -> Self:
        """
        Remove a child Entity from this Entity.
        The child will NOT be deleted, it will simply be detached.
        """
        ...

    @declare_method(50)
    def add_tag(self, tag: "Tag", value: "Value | None" = None) -> "Value":
        """Add or get a Tag's value on this Entity."""
        ...

    @declare_method(51)
    def remove_tag(self, tag: "Tag") -> "Value | None":
        """Remove a Tag from this Entity."""
        ...

    @declare_method(60)
    def into(self, branch: "Branch") -> "Self":
        """
        Turn this Entity into its corresponding Entity in the given Branch.
        """
        ...

    @declare_method(61)
    def instantiate(self, *, partial: bool = True, attach: bool = False, **override: Any) -> "Self":
        """
        Instantiate this Entity into a new (partial or full) Entity.
        If partial is True, the new Entity will be a partial Entity with only override set.
        If attach is True, the new Entity will be attached to the current Entity's parent.
        """
        ...

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
        ...

    def unset(self, key: str):
        """Unset a Property on this Entity partial."""
        ...
