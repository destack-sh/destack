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
    IsOwnable,
    Node,
    NodeType,
    TraitType,
    node_,
    property_,
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
    """

    # type?
    traits: list[TraitType] = property_(40, description="Dynamic traits of the Table.")

    @property
    def records(self) -> Any:
        raise NotImplementedError
