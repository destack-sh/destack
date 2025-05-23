from typing import TYPE_CHECKING, Union

import structlog

from bench.language.core import (
    IsArchivable,
    IsBased,
    IsClaimable,
    IsDeletable,
    IsExtensible,
    IsIcon,
    IsInPackage,
    IsLocal,
    IsModal,
    IsNamed,
    IsOrdered,
    IsOwnable,
    IsTitled,
    Node,
    NodeType,
    node_,
    property_,
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
    IsBased,
    IsModal,
    IsOwnable,
    IsClaimable,
    IsOrdered,
    IsExtensible,
    IsNamed,
    IsTitled,
    IsIcon,
    IsDeletable,
    IsArchivable,
    IsInPackage,
    Node[RecordData],
):
    """
    A Record in a Table.
    """

    # meta
    parent: Union["Table", "Record", None] = property_parent_()
    # type: RecordType?
    table: "Table" = property_(36, description="The Table this Record is from.")

    # target: Page/Task/...? (tie Record to a Page for a Notion-like experience in some Tables)
    # ... general Record/Page/Block 'tying'? :NodeTying
