from typing import TYPE_CHECKING

from bench.language.core import MaybeVariable, node_component_, p_regular
from bench.pb2 import AnyNodeData

from ..view import ViewBase

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@node_component_()
class InputViewBase[NodeDataT: AnyNodeData](ViewBase[NodeDataT]):
    """An input View."""

    # appearance
    is_visible: MaybeVariable[bool] = p_regular(60)
    opacity: MaybeVariable[float] = p_regular(61)
