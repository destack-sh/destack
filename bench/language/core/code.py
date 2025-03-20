import textwrap
from typing import TYPE_CHECKING, Optional

import black
import structlog
from opentelemetry import trace

from .const import BuiltinEnum, EnumType, StructType, enum_
from .property import p_regular
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


@enum_(EnumType.CODE_LANGUAGE)
class CodeLanguage(BuiltinEnum):
    """
    The language of Code.
    """

    PYTHON = 10


@struct_(StructType.CODE)
class Code(Struct):
    """Code in some language."""

    language: Optional[CodeLanguage] = p_regular(32, require=False)
    content: Optional[str] = p_regular(40, require=False)

    def __content_str__(self) -> str:
        preview_str = self.content or ""
        if len(preview_str) > 100:
            preview_str = preview_str[:100] + "..."
        return f"'{preview_str}'"

    def __len__(self) -> int:
        return len(self.content or "")

    def __contains__(self, other: str) -> bool:
        return other in (self.content or "")

    def to_string(self) -> str:
        return self.content or ""

    @staticmethod
    def from_string(s: str, *, language: Optional[CodeLanguage] = None) -> "Code":
        if not s:
            return Code.empty()
        s = textwrap.dedent(s)
        return Code(content=s, language=language)

    @staticmethod
    def empty() -> "Code":
        return Code(content=None)


code = Code.from_string


@tracer.start_as_current_span(name="code.format")
def format_code(code: str, suppress_error: bool = False, line_length: int = 100) -> str:
    """
    Formats the code string with our standard black settings.
    TODO :Performance!: replace black with ruff in format_code
     (unfortunately ruff doesn't have a nice API for this yet, so we would need to use a subprocess?)
    """
    try:
        return black.format_str(code, mode=black.FileMode(line_length=line_length))
    except Exception as e:
        if suppress_error:
            return code
        else:
            raise SyntaxError(code) from e
