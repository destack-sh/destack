from typing import TYPE_CHECKING, Any, Optional, cast, final

import structlog

from bench.language.core import (
    BlockType,
    FieldType,
    HasNodeBase,
    NodeType,
    StateNode,
    StructType,
    TypeBase,
    constraint,
    node_,
    p_internal,
    p_regular,
    p_system,
    p_value_packed,
    p_value_runtime,
)
from bench.pb2 import AnyNodeData, NodeReferenceData, RecordData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import CustomObject, Database, Icon, Text

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@node_(
    NodeType.RECORD,
    passthrough_get=("value",),
    passthrough_set=("value",),
    stored_value_unraveled=True,
)
class Record(StateNode[RecordData], HasNodeBase):
    """
    A Record from a Database. May reference other Records (except for :ManyToManyRecords).
    """

    # type: RecordType?
    name: Optional[str] = p_regular(32, default=None)
    order_key: str | None = p_internal(33, default=INTEGER_ZERO)
    icon: Optional["Icon"] = p_regular(34, default=None, struct=StructType.ICON)
    text: Optional["Text"] = p_regular(
        35, default=None, require=False, array=False, struct=StructType.TEXT
    )
    database: "Database" = p_system(
        36,
        require=True,
        references=NodeType.BLOCK,
        constraint=constraint(node_subtypes=[BlockType.DATABASE]),
        description="The Database this Record is from.",
    )

    # value
    value_packed: Any = p_value_packed(40)
    value: "CustomObject | None" = p_value_runtime(
        40, type=FieldType.MEMBER, typ=lambda self: cast("Record", self).value_type
    )

    @final
    def __repr__(self):  # type: ignore
        # override the default __repr__ for records
        block = self.database
        type_name = block.code_name if block is not None else "???"
        return f"<{type_name}Record {self!s}>"

    def __content_str__(self):
        if (value := self.value) is not None:
            return str(value)
        else:
            return ""

    @property
    def value_type(self) -> "TypeBase | None":
        database = self.database
        return database.to_type_maybe() if database is not None else None

    @property
    def base(self) -> "Database | None":
        return self.database

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return cast(RecordData, data).database_ptr

    @staticmethod
    def get_base_from_partial(data: dict[str, Any]) -> Optional["Database"]:
        if "database" in data:
            return data["database"]
        else:
            return None
