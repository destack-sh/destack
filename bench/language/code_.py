from typing import TYPE_CHECKING, Any

import black
import structlog

from bench.language.const import StructType
from bench.language.node import Struct, struct
from bench.language.property import p_regular

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)


@struct(StructType.CODE_LINE)
class CodeLine(Struct):
    content: str = p_regular(32)


@struct(StructType.CODE)
class Code(Struct):
    # language: ...
    lines: list[CodeLine] = p_regular(30, require=True, array=True, struct=StructType.CODE_LINE)


def code_to_string(code: Code) -> str:
    return "\n".join(line.content for line in code.lines)


def string_to_code(s: str) -> Code:
    return Code(lines=[CodeLine(content=line) for line in s.split("\n")])


def format_code(code: str, suppress_error: bool = False) -> str:
    """Formats the code string with our standard black settings."""
    try:
        return black.format_str(code, mode=black.FileMode(line_length=100))
    except Exception as e:
        if suppress_error:
            return code
        else:
            raise ValueError(code) from e


def default_globals():
    import bench.language
    from bench.language.setup import BENCH_CLASS_BY_NAME

    globals = {**vars(bench.language), **BENCH_CLASS_BY_NAME}
    return globals


def run_code_script(code: str, globals: dict[str, Any] | None = None) -> dict[str, Any]:
    """Runs the code string and extracts its definitions."""
    if globals is None:
        globals = default_globals()
    globals_local = {**globals}
    exec(code, globals_local)
    new_globals = {k: v for k, v in globals_local.items() if k not in globals}
    return new_globals
