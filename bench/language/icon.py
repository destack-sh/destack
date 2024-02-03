from typing import Optional, TYPE_CHECKING

from bench.language.const import StructType
from bench.language.node import struct, Struct, p_internal
from bench.utils.func import IdStrEnum

if TYPE_CHECKING:
    from bench.language import File


class IconType(IdStrEnum):
    pass


@struct(StructType.ICON)
class Icon(Struct):
    # type: ...
    is_custom: bool = p_internal(31, default=False)
    image: Optional["File"] = p_internal(32, require=False, array=False, struct=StructType.FILE)
