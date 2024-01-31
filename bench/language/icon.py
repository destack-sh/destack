from typing import Optional, TYPE_CHECKING

from bench.language.const import StructType
from bench.language.node import struct, Struct, struct_internal
from bench.proto.core import ProtoStrEnum

if TYPE_CHECKING:
    from bench.language import File


class IconType(ProtoStrEnum):
    pass


@struct(StructType.ICON)
class Icon(Struct):
    # type: IconType = struct_property(30, require=True, validate=enum_validator(IconKey))
    is_custom: bool = struct_internal(31, default=False)
    image: Optional["File"] = struct_internal(
        32, require=False, array=False, struct=StructType.FILE
    )
