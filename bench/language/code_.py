import typing

import structlog

from bench.language.const import StructType
from bench.language.node import Struct, struct
from bench.language.property import p_regular

if typing.TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)


@struct(StructType.CODE_LINE)
class CodeLine(Struct):
    content: str = p_regular(32)


@struct(StructType.CODE)
class Code(Struct):
    # language: ...
    lines: list[CodeLine] = p_regular(30, require=True, array=True, struct=StructType.CODE_LINE)
