from bench.language.const import StructType
from bench.language.node import struct, Struct, struct_property
from bench.proto.core import ProtoStrEnum


class SpaceDockItemType(ProtoStrEnum):
    SEARCH = "SEARCH", 1
    CHAT = "CHAT", 2
    ASSIST = "ASSIST", 3
    HELP = "HELP", 4


@struct(StructType.SPACE_DOCK_ITEM)
class SpaceDockItem(Struct):
    # type: SpaceDockItemType = struct_property(
    #     30, require=True, validate=enum_validator(SpaceDockItemType)
    # )
    hidden: bool = struct_property(31, default=False)


@struct(StructType.SPACE_DOCK)
class SpaceDock(Struct):
    items: list[SpaceDockItem] = struct_property(
        30, require=True, array=True, default_factory=list, struct=StructType.SPACE_DOCK_ITEM
    )


class BlockPageViewMode(ProtoStrEnum):
    pass


class BlockPageView:
    pass


class WindowView:
    pass


class WindowGroupView:
    pass


class TabbedView:
    pass
