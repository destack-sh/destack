from typing import TYPE_CHECKING, Any, Optional, Union, cast, final

import structlog

from bench.language.core import (
    FieldType,
    IsBased,
    IsClaimable,
    IsModal,
    IsOrdered,
    IsOwnable,
    IsTitled,
    NodeType,
    PackageNode,
    StructType,
    TypeBase,
    node_,
    p_node_parent,
    p_regular,
    p_system,
    p_value_packed,
    p_value_runtime,
)
from bench.pb2 import AnyNodeData, NodeReferenceData, RecordData

if TYPE_CHECKING:
    from bench.language import CustomObject, Icon, Table

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@node_(NodeType.RECORD, unravel_value=True)
class Record(
    IsBased,
    IsModal,
    IsOwnable,
    IsClaimable,
    IsOrdered,
    IsTitled,
    PackageNode[RecordData],
):
    """
    A Record in a Table.
    """

    # meta
    parent: Union["Table", "Record", None] = p_node_parent(4, NodeType.TABLE, NodeType.RECORD)
    # type: RecordType?
    icon: Optional["Icon"] = p_regular(34, default=None, struct=StructType.ICON)
    table: "Table" = p_system(
        36, require=True, references=NodeType.TABLE, description="The Table this Record is from."
    )

    # target: Page/Task/...? (tie Record to a Page for a Notion-like experience in some Tables)
    # ... general Record 'tying'?

    # value
    value_packed: Any = p_value_packed(40)
    value: "CustomObject | None" = p_value_runtime(
        40, type=FieldType.MEMBER, typ=lambda self: cast("Record", self).value_type
    )

    @final
    def __repr__(self):  # type: ignore
        # override the default __repr__ for records
        table = self.table
        type_name = table.code_name if table is not None else "???"
        return f"<{type_name}Record {self!s}>"

    def __content_str__(self):
        if (value := self.value) is not None:
            return str(value)
        else:
            return ""

    @property
    def value_type(self) -> "TypeBase | None":
        table = self.table
        return table.to_type_maybe(of="value") if table is not None else None

    @property
    def base(self) -> "Table | None":
        return self.table

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return cast(RecordData, data).table_ptr

    @staticmethod
    def get_base_from_partial(data: dict[str, Any]) -> Optional["Table"]:
        if "database" in data:
            return data["database"]
        else:
            return None
