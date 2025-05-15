from typing import TYPE_CHECKING, Optional

from bench.language.core import node_component_, p_regular
from bench.pb2 import AnyNodeData

from ..view import ViewBase

if TYPE_CHECKING:
    from bench.language import Align

# pyright: reportIncompatibleVariableOverride=false


@node_component_()
class ContentViewBase[NodeDataT: AnyNodeData](ViewBase[NodeDataT]):
    """A content View."""

    # layout
    align: Optional["Align"] = p_regular(53, require=False)

    # appearance
    visible: Optional[bool] = p_regular(60, require=False)
    opacity: Optional[float] = p_regular(61, require=False)
