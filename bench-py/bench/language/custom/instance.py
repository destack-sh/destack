from typing import TYPE_CHECKING, Optional, Union

import structlog
from fastuuid import UUID

from bench.language.core import (
    IsDeletable,
    IsExtensible,
    IsInPackage,
    IsModal,
    Node,
    NodeReference,
    NodeType,
    node_,
    property_,
    property_parent_,
)
from bench.pb2 import CustomNodeInstanceData

if TYPE_CHECKING:
    from bench.language import CustomNodeDefinition

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@node_(NodeType.CUSTOM_NODE_INSTANCE)
class CustomNodeInstance(
    IsModal,
    IsExtensible,
    IsInPackage,
    IsDeletable,
    Node[CustomNodeInstanceData],
):
    """
    An instance of a CustomNodeDefinition.
    """

    # meta
    parent: Union["CustomNodeDefinition", "CustomNodeInstance", None] = property_parent_()
    # type: RecordType?

    definition: "CustomNodeDefinition" = property_(
        40,
        description="The CustomNodeDefinition this CustomNode is an instance of.",
        node_bench_from="self",
    )
    if TYPE_CHECKING:
        definition_id: Optional[UUID] = None
        definition_ptr: Optional[NodeReference] = None
    # ... general Record/Page/Block 'tying'? :NodeTying
