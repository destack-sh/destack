from typing import TYPE_CHECKING, Optional, Union

import structlog
from fastuuid import UUID

from destack.pb2 import CustomEntityData, CustomEntityDefinitionData

from ..builtin import (
    HasName,
    IsActionable,
    IsCustomNode,
    IsCustomNodeDefinition,
    IsDeletable,
    IsEntity,
    IsExtensible,
    IsOwnable,
    IsScriptable,
    IsTaggable,
    Node,
    NodeType,
    TraitType,
    node_,
    property_,
    property_parent_,
)
from .relation import NodeReference

if TYPE_CHECKING:
    from destack.language import Folder

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@node_(NodeType.CUSTOM_ENTITY_DEFINITION)
class CustomEntityDefinition(
    HasName,
    IsEntity,
    IsTaggable,
    IsCustomNodeDefinition,
    IsOwnable,
    IsDeletable,
    IsScriptable,
    IsActionable,
    Node[CustomEntityDefinitionData],
):
    """
    A definition for a custom Entity type (instantiated in CustomEntities).
    Custom Entities may be materialized as physical or logical tables in primary storage.
    """

    # type?
    parent: Optional["Folder"] = property_parent_(node_is_customizable=False)
    traits: list[TraitType] = property_(40)


@node_(NodeType.CUSTOM_ENTITY)
class CustomEntity(
    IsTaggable,
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
        17,
        description="The CustomEntityDefinition this CustomEntity is an instance of.",
    )
    if TYPE_CHECKING:
        definition_id: Optional[UUID] = None
        definition_ptr: Optional[NodeReference] = None
