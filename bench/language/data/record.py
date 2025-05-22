from typing import TYPE_CHECKING, Optional, Union, final

import structlog

from bench.language.core import (
    IsBased,
    IsClaimable,
    IsExtensible,
    IsInPackage,
    IsModal,
    IsOrdered,
    IsOwnable,
    IsTitled,
    Node,
    NodeType,
    node_,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.pb2 import RecordData

if TYPE_CHECKING:
    from bench.language import Icon, Table

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@node_(NodeType.RECORD, is_custom=True)
class Record(
    IsBased,
    IsModal,
    IsOwnable,
    IsClaimable,
    IsOrdered,
    IsExtensible,
    IsTitled,
    IsInPackage,
    Node[RecordData],
):
    """
    A Record in a Table.
    """

    # meta
    parent: Union["Table", "Record", None] = p_node_parent()
    # type: RecordType?
    icon: Optional["Icon"] = p_regular(34)
    table: "Table" = p_system(36, description="The Table this Record is from.")

    # target: Page/Task/...? (tie Record to a Page for a Notion-like experience in some Tables)
    # ... general Record/Page/Block 'tying'? :NodeTying

    @final
    def __repr__(self):  # type: ignore
        # override the default __repr__ for records
        table = self.table
        type_name = table.code_name if table is not None else "???"
        return f"<{type_name}Record {self!s}>"
