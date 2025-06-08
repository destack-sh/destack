from typing import TYPE_CHECKING, Optional, Union

import structlog
from fastuuid import UUID

from bench.pb2 import CustomEntityData, CustomEntityDefinitionData

from ..builtin import (
    HasName,
    IsBlockable,
    IsCustomNode,
    IsCustomNodeDefinition,
    IsDeletable,
    IsEnvironmental,
    IsExtensible,
    IsInPackage,
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
    IsEnvironmental,
    HasName,
    IsCustomNodeDefinition,
    IsOwnable,
    IsBlockable,
    IsDeletable,
    IsScriptable,
    IsInPackage,
    Node[CustomEntityDefinitionData],
):
    """
    A definition for a generic Entity type (instantiated in CustomEntities).
    Custom Entities may be materialized as physical or logical tables in primary storage.
    """

    # type?
    traits: list[TraitType] = property_(40, description="Dynamic traits.")


@node_(NodeType.CUSTOM_ENTITY)
class CustomEntity(
    IsEnvironmental,
    IsExtensible,
    IsInPackage,
    IsDeletable,
    IsCustomNode,
    Node[CustomEntityData],
):
    """
    An Entity is an instance of a CustomEntityDefinition.
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
