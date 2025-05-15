from typing import TYPE_CHECKING

from bench.language.core import node_component_
from bench.pb2 import AnyNodeData

from ..view import ViewBase

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@node_component_()
class InputViewBase[NodeDataT: AnyNodeData](ViewBase[NodeDataT]):
    """An input View."""
