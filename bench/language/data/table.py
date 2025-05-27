from typing import TYPE_CHECKING, Any

from bench.language.core import (
    HasName,
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsInPackage,
    IsInstantiable,
    IsModal,
    IsOwnable,
    Node,
    NodeType,
    node_,
)
from bench.pb2 import TableData

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.TABLE)
class Table(
    IsInstantiable,
    IsModal,
    HasName,
    IsOwnable,
    IsBlockable,
    IsArchivable,
    IsDeletable,
    IsInPackage,
    Node[TableData],
):
    """A Table of Records."""

    # type?

    @property
    def records(self) -> Any:
        raise NotImplementedError
