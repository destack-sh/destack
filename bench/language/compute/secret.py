from typing import TYPE_CHECKING, Any, Union, cast

from bench.language.core import (
    FieldType,
    NodeType,
    StructType,
    node_,
    p_node_parent,
    p_regular,
    p_value_packed,
    p_value_runtime,
)
from bench.pb2 import SecretData

from ..core.resource import DynamicResource

if TYPE_CHECKING:
    from bench.language import Bench, Type

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SECRET)
class Secret(DynamicResource[SecretData]):
    """A secret value."""

    parent: Union["Bench", None] = p_node_parent(4, NodeType.BENCH, is_system=True)

    # content
    value_type: "Type" = p_regular(60, struct=StructType.TYPE)
    value_packed: Any = p_value_packed(61, secret=True)
    value = p_value_runtime(
        61, type=FieldType.MEMBER, typ=lambda self: cast(Secret, self).value_type
    )
