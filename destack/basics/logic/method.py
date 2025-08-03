from typing import final

from destack.core import NodeType, declare_entity

from .function import Function

# pyright: reportIncompatibleVariableOverride=false


@declare_entity(NodeType.METHOD, is_final=True)
@final
class Method(Function):
    """
    A Method is a small runtime-specific piece of logic.
    """
