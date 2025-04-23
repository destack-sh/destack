import typing
from typing import Optional, Union

from bench.language.core import (
    NAME_CONSTRAINT,
    IsInstantiable,
    IsModal,
    NodeType,
    PackageNode,
    StructType,
    node_,
    p_internal,
    p_node_parent,
    p_regular,
)
from bench.pb2 import OptionData
from bench.utils.fractional import INTEGER_ZERO

if typing.TYPE_CHECKING:
    from bench.language import Choice, Field, Icon, Text


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@node_(NodeType.OPTION)
class Option(IsInstantiable, IsModal, PackageNode[OptionData]):
    """
    An Option in a Choice or Field.
    """

    parent: Union["Choice", "Field", None] = p_node_parent(4, NodeType.CHOICE, NodeType.FIELD)
    name: str | None = p_regular(31, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(32, default=INTEGER_ZERO)
    text: Optional["Text"] = p_regular(
        33, default=None, require=False, array=False, struct=StructType.TEXT
    )
    icon: Optional["Icon"] = p_regular(34, require=False, array=False, struct=StructType.ICON)

    @staticmethod
    def new(name: str, **kwargs) -> "Option":
        return Option(name=name, **kwargs)
