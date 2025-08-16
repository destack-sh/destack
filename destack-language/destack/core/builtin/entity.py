from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Optional,
    Self,
    dataclass_transform,
)

from ._const import UNSET
from ._hoisted import ReferenceType, UInt64, ValueFactory
from .declaration import (
    ConstraintDeclaration,
    IndexDeclaration,
    PermissionDeclaration,
    TagDeclaration,
    declare_method,
)
from .enum import FlagEnum, OptionEnum, declare_enum, declare_option
from .node import Node, _process_node_cls
from .property import _PROPERTY_SPECIFIERS, PropertyDeclaration, declare_property
from .universe import EnumType, NodeType, ObjectKind, StructType, TraitType

if TYPE_CHECKING:
    from destack import (
        Branch,
        NamedValue,
        NodeSpatialReference,
        Script,
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


@declare_enum(EnumType.PROCESS_FLAG)
class ProcessFlag(FlagEnum):
    """
    How an Entity should be treated for processing by the system.
    """

    DEFAULT = declare_option(0)
    DELETED = declare_option(
        1,
        description="Entity is (soft) deleted.",
    )
    INACTIVE = declare_option(
        2,
        description="Entity is inactive (i.e. paused).",
    )
    INACTIVE_INPUT = declare_option(
        4,
        description="Entity is inactive to InputEvents.",
    )
    SLEEPING = declare_option(
        8,
        description="Entity is sleeping (i.e. not processing).",
    )
    SLEEPING_INPUT = declare_option(
        16,
        description="Entity is sleeping to InputEvents.",
    )


@declare_enum(EnumType.EXTENSION_FLAG)
class ExtensionFlag(FlagEnum):
    """
    How an Entity should be treated for extension by the system.
    """

    DEFAULT = declare_option(0)
    INSTANTIABLE = declare_option(1)
    EXTENSIBLE = declare_option(2)
    # is_locked, is_extensible, is_instantiable, ...
    # is_trait? is_abstract?
    # is_locked/is_final?
    # is_singleton?


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
            is_immutable=False,
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
    event_types=(NodeType.CHANGE_EVENT,),
    permissions=(PermissionDeclaration(id=20, name="create", description="Create (or Upsert)"),),
    tags=(
        TagDeclaration(id=20, name="tree", description="Tree"),
        TagDeclaration(id=21, name="custom", description="Custom"),
        TagDeclaration(id=22, name="behavior", description="Behavior"),
        TagDeclaration(id=23, name="provenance", description="Provenance"),
    ),
)
class Entity(Node):
    """
    An Entity is a named, versioned, mutable Node.
    Entities can be attached to (most) other Entities to compose richer structures.

    Updates to Entities are made through Events.
    Entities are always part of a Snapshot (in their Space).
     (Technically, Entities are just a temporary materialization of the Event stream.)

    An instance of an Entity is identified by an (id, branch_id, snapshot_id) tuple,
     where Snapshots are 'shortcuts' to certain epochs:
     (id, definition_id) @ (branch_id, snapshot_id, epoch)

    Custom and context Values are keyed by name for convenience and clarity.
    The name is normalized to a snake_case string.
    """

    metakind = ObjectKind.NODE
    __parent_property__: ClassVar[PropertyDeclaration] = UNSET

    # 1-20: identity
    materialization: Materialization = declare_property(
        10,
        is_managed=True,
        is_eq=False,
        is_hash=False,
        default=Materialization.ROOT,
        tag="identity",
    )
    definition: Optional["Entity"] = declare_property(
        11,
        is_managed=True,
        is_readonly=True,
        reference_type=ReferenceType.SPATIAL,
        description="The definition this Entity is an instance of.",
        tag="identity",
    )
    # preceded_by?
    instance: Optional["Entity"] = declare_property(
        13,
        is_readonly=True,
        is_managed=True,
        is_eq=False,
        is_hash=False,
        reference_type=ReferenceType.SPATIAL,
        description="The (root) Entity that is being instantiated.",
        tag="identity",
    )

    # 20-40: tracking
    created_epoch: UInt64 = declare_property(
        20,
        is_managed=True,
        is_hash=False,
        is_eq=False,
        default_factory=ValueFactory.REMOTE_EPOCH,
        description="The logical time this Entity was created (system time).",
        tag="tracking",
    )
    updated_epoch: UInt64 = declare_property(
        21,
        is_managed=True,
        is_hash=False,
        is_eq=False,
        default_factory=ValueFactory.REMOTE_EPOCH,
        description="The logical time this Entity was last updated (system time).",
        tag="tracking",
    )
    owner: "Entity" = declare_property(
        22,
        is_repr=True,
        description="The exclusive owner of this Entity (the root on access).",
        reference_type=ReferenceType.SPATIAL,
        default_factory=ValueFactory.ACTOR,
        tag="tracking",
    )
    controller: "Entity" = declare_property(
        23,
        is_repr=True,
        description="The exclusive controller of this Entity (the authority on Changes).",
        reference_type=ReferenceType.SPATIAL,
        default_factory=ValueFactory.ACTOR,
        tag="tracking",
    )
    process_flags: ProcessFlag = declare_property(
        30,
        is_managed=True,
        is_hash=False,
        is_eq=False,
        default=ProcessFlag.DEFAULT,
        description="The process flags of this Entity.",
        tag="tracking",
    )

    # 40-50: tree
    parent: Optional["Entity"] = declare_property(
        40,
        reference_type=ReferenceType.SPATIAL,
        description="The parent of this Entity. Most Entities can be attached to any other Entity.",
        tag="tree",
    )
    name: str = declare_property(
        41,
        is_repr=True,
        is_interned=True,
        description="The name of this Entity.",
        default_factory=ValueFactory.NAME,
        tag="tree",
    )
    key: str | None = declare_property(
        42,
        is_eq=False,
        is_interned=True,
        is_managed=True,
        description="The key of this Entity (for reconciliation and querying).",
        tag="tree",
    )
    order_key: str | None = declare_property(
        43,
        is_eq=False,
        is_interned=True,
        is_managed=True,
        description="The absolute order of this Entity (in its parent, as a order key).",
        tag="tree",
    )
    # icon: Optional["Icon"]?

    # 50-60: custom
    extension_flags: ExtensionFlag = declare_property(
        50,
        description="The extension flags of this Entity.",
        tag="custom",
    )
    custom_values: list["NamedValue"] | None = declare_property(
        51,
        description="The custom Values of this Entity, keyed by custom Property or Tag name.",
        tag="custom",
    )
    # context_values?

    # 60-70: behavior
    script: Optional["Script"] = declare_property(
        60,
        description="The Script of this Entity.",
        reference_type=ReferenceType.SPATIAL,
        tag="behavior",
    )

    # 70-80: provenance
    # provenance: Optional["Script"]?
    # key? (for reconciliation)
    # ... (from script, dynamic effect, manual function, import, ...)

    @declare_method(2)
    def to_ref(self) -> "NodeSpatialReference":
        """Gets a reference to this Node."""
        raise NotImplementedError

    @declare_method(20)
    def delete(self):
        """Delete this Entity."""
        raise NotImplementedError

    @declare_method(21)
    def restore(self):
        """Restore this deleted Entity from the trash."""
        raise NotImplementedError

    @declare_method(30)
    def get_children(
        self,
        type: NodeType | None = None,
        include_deleted: bool = False,
    ) -> list["Entity"]:
        """Gets the children of this Entity."""
        raise NotImplementedError

    @declare_method(31)
    def get_child(
        self,
        type: NodeType,
        name: str,
        include_deleted: bool = False,
    ) -> Optional["Entity"]:
        """Gets a specific child of this Node by name."""
        raise NotImplementedError

    @declare_method(32)
    def child(
        self,
        type: NodeType,
        name: str,
        include_deleted: bool = False,
    ) -> "Entity":
        """Gets a specific child of this Node by name, or raises an error if not found."""
        raise NotImplementedError

    @declare_method(33)
    def get_ancestors(
        self,
        type: NodeType | None = None,
        include_deleted: bool = False,
    ) -> list["Entity"]:
        """Gets the ancestors of this Node."""
        raise NotImplementedError

    @declare_method(34)
    def get_descendants(
        self,
        type: NodeType | None = None,
        include_deleted: bool = False,
    ) -> list["Entity"]:
        """Gets the descendants of this Node."""
        raise NotImplementedError

    @declare_method(40)
    def detach(self):
        """
        Detach this Entity from its parent.
        Does not delete the Entity, just removes it from its parent.
        Raises an error if it has no parent.
        """
        raise NotImplementedError

    @declare_method(41)
    def move_to(
        self,
        parent: Optional["Entity"],
        *,
        after: Optional["Entity"] = None,
        before: Optional["Entity"] = None,
    ):
        """
        Move this Entity to a new parent Entity.
        If the Entity is new, it will be automatically created in this Entity's Session (for convenience).
        (The same applies to all descendants.)
        """
        raise NotImplementedError

    @declare_method(42)
    def add_sibling(
        self,
        sibling: "Entity",
        *,
        after: Optional["Entity"] = None,
        before: Optional["Entity"] = None,
    ) -> Self:
        """
        Add an Entity as a sibling of this Entity.
        If the Entity is new, it will be automatically created in this Entity's Session (for convenience).
        (The same applies to all descendants.)
        """
        raise NotImplementedError

    @declare_method(43)
    def add_siblings(
        self,
        *siblings: "Entity",
        after: Optional["Entity"] = None,
        before: Optional["Entity"] = None,
    ) -> Self:
        """Add multiple Entities as siblings of this Entity."""
        raise NotImplementedError

    @declare_method(44)
    def add_child(
        self,
        child: "Entity",
        *,
        after: Optional["Entity"] = None,
        before: Optional["Entity"] = None,
    ) -> Self:
        """
        Append an Entity as a child of this Entity (and all its descendants).
        If the Entity IsOrdered, it will be positioned (relative to after/before).
        If the Entity is new, it will be automatically created in this Entity's Session (for convenience).
        (The same applies to all descendants.)
        """
        raise NotImplementedError

    @declare_method(45)
    def add_children(
        self,
        *children: "Entity",
        after: Optional["Entity"] = None,
        before: Optional["Entity"] = None,
    ) -> Self:
        """
        Append multiple Entities as children of this Entity.
        """
        raise NotImplementedError

    @declare_method(46)
    def remove_child(self, child: "Entity") -> Self:
        """
        Remove a child Entity from this Entity.
        The child will NOT be deleted, it will simply be detached.
        """
        raise NotImplementedError

    @declare_method(50)
    def add_tag(self, tag: "Tag", value: Optional["Value"] = None) -> "Value":
        """Add or get a Tag's value on this Entity."""
        raise NotImplementedError

    @declare_method(51)
    def remove_tag(self, tag: "Tag") -> Optional["Value"]:
        """Remove a Tag from this Entity."""
        raise NotImplementedError

    @declare_method(60)
    def checkout(self, branch: "Branch") -> "Self":
        """
        Turn this Entity into its corresponding Entity in the given Branch.
        """
        raise NotImplementedError

    @declare_method(61)
    def instantiate(
        self,
        *,
        partial: bool = True,
        attach: bool = False,
        **override: Any,
    ) -> "Self":
        """
        Instantiate this Entity into a new (partial or full) Entity.
        If partial is True, the new Entity will be a partial Entity with only override set.
        If attach is True, the new Entity will be attached to the current Entity's parent.
        """
        raise NotImplementedError


ENTITY_MATERIALIZATION_ID = Entity.property("materialization").id
ENTITY_MATERIALIZATION_KEY = str(ENTITY_MATERIALIZATION_ID)
