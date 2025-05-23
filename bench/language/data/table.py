from typing import TYPE_CHECKING, Any

from bench.language.core import (
    IsArchivable,
    IsBlockable,
    IsClaimable,
    IsDeletable,
    IsInstantiable,
    IsModal,
    IsNamed,
    IsOwnable,
    Node,
    NodeType,
    node_,
)
from bench.pb2 import TableData

if TYPE_CHECKING:
    from bench.language import Field

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.TABLE)
class Table(
    IsInstantiable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsClaimable,
    IsBlockable,
    IsArchivable,
    IsDeletable,
    Node[TableData],
):
    """A Table of Records."""

    # type?

    @property
    def records(self) -> Any:
        raise NotImplementedError

    @staticmethod
    def new(name: str, *fields: "Field", **kwargs) -> "Table":
        table = Table(name=name, **kwargs)
        for field in fields:
            table.add_child(field)
        return table
