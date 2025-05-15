import typing
from typing import Optional, Union

from bench.language.core import (
    IsInstantiable,
    IsModal,
    IsNamed,
    IsOrdered,
    NodeType,
    PackageNode,
    StructType,
    node_,
    p_node_parent,
    p_regular,
)
from bench.pb2 import OptionData

if typing.TYPE_CHECKING:
    from bench.language import Choice, Field, Icon


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@node_(NodeType.OPTION)
class Option(IsInstantiable, IsModal, IsNamed, IsOrdered, PackageNode[OptionData]):
    """
    An Option in a Choice or Field.
    """

    parent: Union["Choice", "Field", None] = p_node_parent(4, NodeType.CHOICE, NodeType.FIELD)
    icon: Optional["Icon"] = p_regular(34, require=False, array=False, struct=StructType.ICON)

    @staticmethod
    def new(name: str, **kwargs) -> "Option":
        return Option(name=name, **kwargs)
