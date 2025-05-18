from typing import TYPE_CHECKING

from bench.language.core import VariableProperty, node_component_, p_regular
from bench.pb2 import AnyNodeData

from ..view import ViewBase

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@node_component_()
class InputViewBase[NodeDataT: AnyNodeData](ViewBase[NodeDataT]):
    """An input View."""

    # appearance
    is_visible: VariableProperty[bool] = p_regular(60)
    opacity: VariableProperty[float] = p_regular(61)
