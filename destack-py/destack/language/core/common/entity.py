from typing import TYPE_CHECKING, Optional, Union

import structlog

from destack.proto import CustomEntityData, CustomEntityDefinitionData
from destack.utils.uuid import UUID

from ..builtin import (
    Entity,
    HasName,
    IsActionable,
    IsCustomNode,
    IsCustomNodeDefinition,
    IsDeletable,
    IsExtensible,
    IsOwnable,
    IsScriptable,
    IsSourceable,
    IsTaggable,
    Node,
    NodeType,
    Spatial,
    TraitType,
    builtin_node,
    property_,
    property_parent_,
)
from .relation import NodeReference

if TYPE_CHECKING:
    from destack.language import Folder

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@builtin_node(NodeType.CUSTOM_ENTITY_DEFINITION)
class CustomEntityDefinition(
    Spatial,
    Entity,
    HasName,
    IsCustomNodeDefinition,
    IsTaggable,
    IsOwnable,
    IsDeletable,
    IsScriptable,
    IsSourceable,
    IsActionable,
    Node[CustomEntityDefinitionData],
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
    Entity,
    IsExtensible,
    IsDeletable,
    IsCustomNode,
    Node[CustomEntityData],
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
    )
    if TYPE_CHECKING:
        definition_id: Optional[UUID] = None
        definition_ptr: Optional[NodeReference] = None
