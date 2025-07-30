from typing import final

from ..builtin import NodeType, builtin_node
from .function import Function

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.METHOD, is_final=True)
@final
class Method(Function):
    """
    A Method is a small runtime-specific piece of logic.
    """
