from typing import TYPE_CHECKING, Any

import structlog

from bench.language.core import (
    HasName,
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsInPackage,
    IsInstantiable,
    IsModal,
    IsNodeType,
    IsOwnable,
    Node,
    NodeType,
    node_,
)
from bench.pb2 import TableData

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@node_(NodeType.TABLE)
class Table(
    IsInstantiable,
    IsModal,
    IsNodeType,
    HasName,
    IsOwnable,
    IsBlockable,
    IsArchivable,
    IsDeletable,
    IsInPackage,
    Node[TableData],
):
    """
    A Table of Records, like a custom Node type with Fields as Properties.
    NOTE :Architecture: I don't love that we call custom Node definitions "Tables",
     but Tables / Records is a very natural way to describe the most common use case.
     Maybe we'll abstract out the IsCustomNode and IsCustomInstance traits or something.
    """

    # type?

    @property
    def records(self) -> Any:
        raise NotImplementedError
