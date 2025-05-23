import textwrap
from typing import TYPE_CHECKING, Optional

import structlog
from opentelemetry import trace

from .const import BuiltinEnum, EnumType, StructType, enum_
from .property import property_
from .struct import Struct, struct_

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@enum_(EnumType.CODE_TYPE)
class CodeType(BuiltinEnum):
    """
    The (implicit) 'type' of Code.
    We don't set this explicitly in Code because it depends on where the Code is used.
    """

    SNIPPET = 1  # inline expressions and procedures anywhere
    SCRIPT = 2  # define Python-level commons 'scripts'
    FUNCTION = 3  # Python functions


@struct_(StructType.CODE)
class Code(Struct):
    """Code in some language."""

    language: Optional[str] = property_(32, is_repr=True)
    content: Optional[str] = property_(40)

    def __len__(self) -> int:
        return len(self.content or "")

    def __contains__(self, other: str) -> bool:
        return other in (self.content or "")

    def to_string(self) -> str:
        return self.content or ""

    @staticmethod
    def from_string(s: str, *, language: Optional[str] = None) -> "Code":
        if not s:
            return Code.empty()
        s = textwrap.dedent(s)
        return Code(content=s, language=language)

    @staticmethod
    def empty() -> "Code":
        return Code(content=None)


code = Code.from_string
