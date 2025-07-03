from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Optional,
    Self,
    Union,
)

from .common import Materialization, ResourceStatus, RoleType
from .node import Node, NodeType, builtin_node
from .property import (
    builtin_property,
    builtin_property_parent,
)
from .trait import (
    IsArchivable,
    IsCustomizable,
    IsDeletable,
    IsExtensible,
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


@builtin_node(NodeType.ENTITY, is_abstract=True)
class Entity(Node):
    """
    An Entity is a versioned, stateful Node.
    """

    # 10-20: entity materialization
    # nocheckin: support Entity branching & variants (how to handle Snapshot, which is an Entity?)
    # primary key: (id, snapshot_id)
    materialization: Materialization = builtin_property(
        10,
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
        description="The template this Entity instance is based on (from another instance).",
    )
    instance_root: Optional["Entity"] = builtin_property(
        14,
        is_readonly=True,
        is_managed=True,
        description="The (root) Entity in this Entity's instance tree.",
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


@builtin_node(NodeType.SNAPSHOT)
class Snapshot(
    IsSpatial,
    IsOwnable,
    IsDeletable,
    Entity,
):
    """A Snapshot is a point in Space time."""

    parent: Union["Space", None] = builtin_property_parent()

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
