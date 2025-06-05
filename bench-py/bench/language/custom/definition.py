from typing import TYPE_CHECKING, Any

import structlog

from bench.language.core import (
    HasEnvironment,
    HasName,
    IsBlockable,
    IsDeletable,
    IsInPackage,
    IsOwnable,
    Node,
    NodeType,
    TraitType,
    node_,
    property_,
)
from bench.pb2 import CustomNodeDefinitionData

if TYPE_CHECKING:
    from bench.language import CustomNodeDefinition

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@node_(NodeType.CUSTOM_NODE_DEFINITION)
class CustomNodeDefinition(
    HasEnvironment,
    HasName,
    IsOwnable,
    IsBlockable,
    IsDeletable,
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
