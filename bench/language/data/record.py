from typing import TYPE_CHECKING, Union

import structlog

from bench.language.core import (
    IsDeletable,
    IsExtensible,
    IsInPackage,
    IsLocal,
    IsModal,
    IsNodeInstance,
    Node,
    NodeType,
    node_,
    property_parent_,
)
from bench.pb2 import RecordData

if TYPE_CHECKING:
    from bench.language import Table

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@node_(NodeType.RECORD)
class Record(
    IsLocal,
    IsNodeInstance,
    IsModal,
    IsExtensible,
    IsInPackage,
    IsDeletable,
    Node[RecordData],
):
    """
    A Record in a Table, basically an instance of a custom Node.
    """

    # meta
    parent: Union["Table", "Record", None] = property_parent_()
    # type: RecordType?

    # target: Page/Task/...? (tie Record to a Page for a Notion-like experience in some Tables)
    # ... general Record/Page/Block 'tying'? :NodeTying
