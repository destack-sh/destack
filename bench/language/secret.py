from typing import TYPE_CHECKING, Any, Union, cast

from bench.language.bench import AnonymousResourceNode
from bench.language.const import NodeType, ObjectKind, StructType
from bench.language.field import TypeInfo
from bench.language.node import (
    node_,
)
from bench.language.property import (
    p_node_parent,
    p_regular,
    p_value_packed,
    p_value_runtime,
)
from bench.language.validation import TITLE_CONSTRAINT
from bench.proto.wire import SecretData

if TYPE_CHECKING:
    from bench.language import Vault

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SECRET)
class Secret(AnonymousResourceNode[SecretData]):
    """A secret value."""

    parent: Union["Vault", None] = p_node_parent(4, NodeType.VAULT, is_system=True)

    # meta
    title: str = p_regular(33, constraint=TITLE_CONSTRAINT)

    # content
    value_type: TypeInfo = p_regular(40, struct=StructType.TYPE_INFO)
    value_packed: Any = p_value_packed(41, secret=True)
    value = p_value_runtime(
        41, kind=ObjectKind.MEMBER, typ=lambda self: cast(Secret, self).value_type
    )
