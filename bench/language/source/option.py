import typing
from typing import Any, Optional, Union, cast
from uuid import UUID

from bench.language.core import (
    NAME_CONSTRAINT,
    IsBased,
    Node,
    NodeType,
    SourceNode,
    StructType,
    _IntoQuery,
    node_,
    p_internal,
    p_node_parent,
    p_regular,
)
from bench.pb2 import AnyNodeData, NodeReferenceData, OptionData
from bench.utils.fractional import INTEGER_ZERO

if typing.TYPE_CHECKING:
    from bench.language import Choice, Icon, Text


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@node_(NodeType.OPTION)
class Option(SourceNode[OptionData], IsBased, _IntoQuery):
    """
    An Option in a Choice or something.
    """

    parent: Union["Choice", None] = p_node_parent(4, NodeType.CHOICE)
    name: str | None = p_regular(31, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(32, default=INTEGER_ZERO)
    text: Optional["Text"] = p_regular(
        33, default=None, require=False, array=False, struct=StructType.TEXT
    )
    icon: Optional["Icon"] = p_regular(34, require=False, array=False, struct=StructType.ICON)

    @property
    def base(self) -> Optional[Node]:
        return self.parent

    @property
    def base_ck(self) -> Optional[UUID]:
        return self.base.ck if self.base is not None else None

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return (cast(OptionData, data)).parent_ptr

    @staticmethod
    def get_base_from_partial(data: dict[str, Any]) -> Optional[Node]:
        if "parent" in data:
            return data["parent"]
        else:
            return None

    @staticmethod
    def new(name: str, **kwargs) -> "Option":
        return Option(name=name, **kwargs)
