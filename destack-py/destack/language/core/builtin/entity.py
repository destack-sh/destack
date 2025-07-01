from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Optional,
    Union,
)

from .common import ResourceStatus, RoleType
from .node import Node, NodeType, builtin_node
from .property import (
    property_,
    property_parent_,
)
from .trait import (
    HasName,
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
        IsSubject,
        NodeDefinitionReference,
        NodeReference,
    )

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.ENTITY, is_abstract=True)
class Entity(Node):
    """
    An Entity is a versioned, stateful Node.
    """

    created_at: datetime = property_(15, is_managed=True, is_eq=False, can_write=RoleType.SYSTEM)
    created_by: Optional["IsSubject"] = property_(
        16,
        default=None,
        is_managed=True,
        is_eq=False,
        node_space_from="self",
        node_is_customizable=False,
        can_write=RoleType.SYSTEM,
    )
    updated_at: datetime = property_(17, is_managed=True, is_eq=False, can_write=RoleType.SYSTEM)
    updated_by: Optional["IsSubject"] = property_(
        18,
        default=None,
        is_managed=True,
        is_eq=False,
        node_space_from="self",
        node_is_customizable=False,
        can_write=RoleType.SYSTEM,
    )
    if TYPE_CHECKING:
        created_by_ptr: Optional[NodeReference] = None
        updated_by_ptr: Optional[NodeReference] = None

    # nocheckin: support Entity branching & variants (how to handle Snapshot, which is an Entity?)
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
    IsSpatial,
    HasName,
    IsCustomizable,
    IsTaggable,
    IsOwnable,
    IsDeletable,
    IsScriptable,
    IsSourceable,
    Entity,
):
    """
    A definition for a custom Entity type (instantiated in CustomEntities).
    Custom Entities may be materialized as physical or logical tables in primary storage.
    """

    parent: Optional["Folder"] = property_parent_(node_is_customizable=False)

    prototype: Optional["CustomEntity"] = property_(
        40,
        description="A custom Entity's prototype is the default template new CustomEntity instances are based on.",
    )
    base_type: Optional["NodeDefinitionReference"] = property_(41)
    base_traits: list["NodeDefinitionReference"] = property_(42)
    is_abstract: bool = property_(45, default=False)


@builtin_node(NodeType.CUSTOM_ENTITY, is_abstract=True)
class CustomEntity(
    IsSpatial,
    IsExtensible,
    IsCustomizable,
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


@builtin_node(NodeType.CUSTOM_TRAIT_DEFINITION)
class CustomTraitDefinition(
    IsSpatial,
    HasName,
    IsSourceable,
    IsDeletable,
    IsScriptable,
    IsCustomizable,
    Entity,
):
    """
    A CustomTraitDefinition defines a kind of CustomTrait.
    """

    parent: Optional["Folder"] = property_parent_(node_is_customizable=False)

    base_type: Optional["NodeDefinitionReference"] = property_(41)
    base_traits: list["NodeDefinitionReference"] = property_(42)
    is_abstract: bool = property_(45, default=False)


@builtin_node(NodeType.RESOURCE, is_abstract=True)
class Resource(IsDeletable, Entity):
    """
    A Resource represents an external asset.
    The lifecycle of a Resource may be managed by some provisioner.
    """

    status: ResourceStatus = property_(40, default=ResourceStatus.PENDING)
    target_status: Optional[datetime] = property_(41)


@builtin_node(NodeType.METRIC, is_abstract=True)
class Metric(IsSpatial, HasName, IsSourceable, Entity):
    """An Entity that represents a Metric."""

    pass
