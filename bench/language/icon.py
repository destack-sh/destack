from typing import Optional, TYPE_CHECKING

from bench.language.const import StructType
from bench.language.node import struct, Struct, p_internal
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import File


class IconKind(IdEnum):
    EMOJI = 1
    BUILTIN = 2
    CUSTOM = 3


class IconType(IdEnum):
    pass


@struct(StructType.ICON)
class Icon(Struct):
    kind: IconKind = p_internal(30, default=False)
    emoji: Optional[str] = p_internal(31, require=False)
    type: Optional[IconType] = p_internal(32, require=False)
    image: Optional["File"] = p_internal(33, require=False, array=False, struct=StructType.FILE)
