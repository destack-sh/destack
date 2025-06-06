from typing import TYPE_CHECKING, Any, Optional, Union

import structlog
from fastuuid import UUID

from bench.pb2 import CustomNodeDefinitionData, CustomNodeInstanceData

from ..builtin import (
    HasEnvironment,
    HasName,
    IsBlockable,
    IsDeletable,
    IsExtensible,
    IsInPackage,
    IsOwnable,
    IsScriptable,
    Node,
    NodeReference,
    NodeType,
    TraitType,
    node_,
    property_,
    property_parent_,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@node_(NodeType.CUSTOM_NODE_DEFINITION)
class CustomNodeDefinition(
    HasEnvironment,
    HasName,
    IsOwnable,
    IsBlockable,
    IsDeletable,
    IsScriptable,
    IsInPackage,
    Node[CustomNodeDefinitionData],
):
    """
    A definition for a custom Node type (instantiated in CustomNodeInstances).
    """

    # type?
    traits: list[TraitType] = property_(40, description="Dynamic traits.")

    @property
    def records(self) -> Any:
        raise NotImplementedError


@node_(NodeType.CUSTOM_NODE_INSTANCE)
class CustomNodeInstance(
    HasEnvironment,
    IsExtensible,
    IsInPackage,
    IsDeletable,
    Node[CustomNodeInstanceData],
):
    """
    An instance of a CustomNodeDefinition.
    """

    parent: Union["CustomNodeDefinition", "CustomNodeInstance", None] = property_parent_()
    definition: "CustomNodeDefinition" = property_(
        40,
        description="The CustomNodeDefinition this CustomNode is an instance of.",
        node_bench_from="self",
    )
    if TYPE_CHECKING:
        definition_id: Optional[UUID] = None
        definition_ptr: Optional[NodeReference] = None
