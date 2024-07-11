from typing import Any, cast

from bench.language.bench import Bench
from bench.language.const import NodeType, StructType
from bench.language.field import TypeInfo
from bench.language.node import BenchNode, InlineStruct, node_, struct_
from bench.language.property import p_node_parent, p_regular, p_value_packed, p_value_runtime
from bench.language.validation import TITLE_CONSTRAINT
from bench.language.value import HasValues
from bench.proto.wire import SecretData

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SECRET)
class Secret(BenchNode[SecretData], HasValues):
    """A secret value."""

    parent: Bench | None = p_node_parent(4, NodeType.BENCH, is_system=True)

    # meta
    title: str = p_regular(33, constraint=TITLE_CONSTRAINT)

    # content
    value_type: TypeInfo = p_regular(40, struct=StructType.TYPE_INFO)
    value_packed: Any = p_value_packed(41, secret=True)
    value = p_value_runtime(41, typ=lambda self: cast(Secret, self).value_type)


@struct_(StructType.SECRET_REFERENCE, inline=True)
class SecretReference(InlineStruct):
    """
    A reference to a Secret.
    Like a NodeReference with secret-specific metadata.
    """

    title: str = p_regular(33, constraint=TITLE_CONSTRAINT)
    value_type: TypeInfo | None = p_regular(41, struct=StructType.TYPE_INFO)
    secret: Secret = p_regular(40, require=True, array=False, references=NodeType.SECRET)
