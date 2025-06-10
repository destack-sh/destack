from typing import TYPE_CHECKING, Optional, Union

import structlog
from fastuuid import UUID

from destack.pb2 import CustomEntityData, CustomEntityDefinitionData

from ..builtin import (
    HasName,
    IsCustomNode,
    IsCustomNodeDefinition,
    IsDeletable,
    IsEntity,
    IsEnvironmental,
    IsExtensible,
    IsInFolder,
    IsOwnable,
    IsScriptable,
    Node,
    NodeType,
    TraitType,
    node_,
    property_,
    property_parent_,
)
from .relation import NodeReference

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@node_(NodeType.CUSTOM_ENTITY_DEFINITION)
class CustomEntityDefinition(
    HasName,
    IsEntity,
    IsEnvironmental,
    IsCustomNodeDefinition,
    IsOwnable,
    IsDeletable,
    IsScriptable,
    IsInFolder,
    Node[CustomEntityDefinitionData],
):
    """
    A definition for a custom Entity type (instantiated in CustomEntities).
    Custom Entities may be materialized as physical or logical tables in primary storage.
    """

    # type?
    traits: list[TraitType] = property_(40)


@node_(NodeType.CUSTOM_ENTITY)
class CustomEntity(
    IsEnvironmental,
    IsExtensible,
    IsInFolder,
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
        17,
        description="The CustomEntityDefinition this CustomEntity is an instance of.",
    )
    if TYPE_CHECKING:
        definition_id: Optional[UUID] = None
        definition_ptr: Optional[NodeReference] = None
