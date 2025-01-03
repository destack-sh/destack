from typing import TYPE_CHECKING, Any, Union, cast

from bench.language.core import (
    NodeType,
    ObjectKind,
    StructType,
    node_,
    p_node_parent,
    p_regular,
    p_value_packed,
    p_value_runtime,
)
from bench.language.resource import DynamicResource
from bench.language.source import TypeInfo
from bench.proto.wire import SecretData

if TYPE_CHECKING:
    from bench.language import Bench

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SECRET)
class Secret(DynamicResource[SecretData]):
    """A secret value."""

    parent: Union["Bench", None] = p_node_parent(4, NodeType.BENCH, is_system=True)

    # content
    value_type: TypeInfo = p_regular(50, struct=StructType.TYPE_INFO)
    value_packed: Any = p_value_packed(51, secret=True)
    value = p_value_runtime(
        51, kind=ObjectKind.MEMBER, typ=lambda self: cast(Secret, self).value_type
    )
