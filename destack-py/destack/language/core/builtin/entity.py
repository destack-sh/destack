from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Optional,
)

from .common import ResourceStatus, RoleType
from .node import Node, NodeType, builtin_node
from .property import (
    builtin_property,
    builtin_property_parent,
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

    created_at: datetime = builtin_property(
        15,
        is_managed=True,
        is_eq=False,
        is_readonly=True,
        can_write=RoleType.SYSTEM,
    )
    created_by: Optional["IsSubject"] = builtin_property(
        16,
        default=None,
        is_managed=True,
        is_eq=False,
        is_readonly=True,
        node_space_from="self",
        node_is_extensible=False,
        can_write=RoleType.SYSTEM,
    )
    updated_at: datetime = builtin_property(
        17,
        is_managed=True,
        is_eq=False,
        can_write=RoleType.SYSTEM,
    )
    updated_by: Optional["IsSubject"] = builtin_property(
        18,
        default=None,
        is_managed=True,
        is_eq=False,
        node_space_from="self",
        node_is_extensible=False,
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
    #     node_is_extensible=False,
    #     node_space_from="self",
    #     description="The Snapshot this Entity's snapshot is based on.",
    # )
    # instance: Optional["Entity"] = property_(
    #     10,
    #     can_write=None,
    #     is_managed=True,
    #     node_is_extensible=False,
    #     description="The (root) Entity in this Entity's instance tree.",
    # )
    # template: Optional["Entity"] = property_(
    #     11,
    #     can_write=None,
    #     is_managed=True,
    #     node_is_extensible=False,
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
    A definition for a custom Entity type.
    Custom Entities are instantiated either as:
     1) their respective extensible base type (like ContainerView)
     2) plain CustomEntity instance (default if not extending any other type)
    """

    parent: Optional["Folder"] = builtin_property_parent(node_is_extensible=False)

    base_type: "NodeDefinitionReference" = builtin_property(40)
    base_traits: list["NodeDefinitionReference"] = builtin_property(41)
    is_abstract: bool = builtin_property(45, default=False)

    prototype: Optional["CustomEntity"] = builtin_property(
        50,
        description="A custom Entity's prototype is the default template new CustomEntity instances are based on.",
    )


@builtin_node(NodeType.CUSTOM_ENTITY, is_abstract=True)
class CustomEntity(
    IsSpatial,
    IsExtensible,
    IsDeletable,
    Entity,
):
    """
    A generic CustomEntity instance of a CustomEntityDefinition.
    """

    definition: "CustomEntityDefinition" = builtin_property(
        6,
        is_managed=True,
        is_readonly=True,
        description="The CustomEntityDefinition this CustomEntity is an instance of.",
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

    parent: Optional["Folder"] = builtin_property_parent(node_is_extensible=False)
    base_type: Optional["NodeDefinitionReference"] = builtin_property(40)
    base_traits: list["NodeDefinitionReference"] = builtin_property(41)
    is_abstract: bool = builtin_property(45, default=False)


@builtin_node(NodeType.RESOURCE, is_abstract=True)
class Resource(IsDeletable, IsExtensible, Entity):
    """
    A Resource represents an external asset outside of Destack.
    The lifecycle of a Resource may be managed by some Provisioner.
    """

    status: ResourceStatus = builtin_property(100, default=ResourceStatus.PENDING)
    target_status: Optional[datetime] = builtin_property(101)


@builtin_node(NodeType.METRIC, is_abstract=True)
class Metric(IsSpatial, HasName, IsSourceable, Entity):
    """An Entity that represents a Metric."""

    pass
