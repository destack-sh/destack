from asyncio import CancelledError
from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    BenchError,
    EnumType,
    ErrorKind,
    Node,
    Struct,
    StructType,
    Text,
    TextLine,
    ValidationError,
    enum_,
    p_internal,
    p_regular,
    struct_,
)
from bench.utils.func import IdEnum
from bench.utils.string import Casing, to_casing

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false


@struct_(StructType.RUN_TRACE)
class RunTrace(Struct):
    """A stacktrace for a Run."""

    frames: list["RunFrame"] = p_regular(30, array=True, struct=StructType.RUN_FRAME)


@struct_(StructType.RUN_FRAME)
class RunFrame(Struct):
    """A single frame in a stacktrace."""

    pass


@enum_(EnumType.ERROR_TYPE)
class ErrorType(IdEnum):
    # unretryable
    ABORTED = 2
    RUNTIME_UNAVAILABLE = 3
    RUN_IMPOSSIBLE = 4
    NOT_SUPPORTED = 5
    INVALID_VALUE = 10
    INVALID_COMPUTED = 11
    CODE_INVALID = 20
    TEXT_INVALID = 21
    MODEL_INCAPABLE = 100
    MODEL_REFUSED = 101
    NON_RETRYABLE = 499
    # retryable
    MODEL_FAILED = 500
    INVALID_CONTINUATION = 502
    INVALID_CALL = 503
    INVALID_PLAN = 504
    INTERRUPTION_CANCELLED = 510
    RETRYABLE = 999

    @property
    def is_retryable(self) -> bool:
        """Whether this error type is retryable *at runtime*"""
        return self > 500


@struct_(StructType.ERROR)
class Error(Struct, BenchError):
    """An error that occurred in the context of a Run."""

    kind: ErrorKind = p_internal(30)
    type: ErrorType = p_internal(31, default=None)
    title: Optional[str] = p_internal(32, default=None)
    text: Optional["Text"] = p_internal(33, default=None, struct=StructType.TEXT)
    nodes: list["Node"] = p_internal(34, array=True, require=False, references="any")
    trace: Optional[RunTrace] = p_internal(
        35, require=False, array=False, struct=StructType.RUN_TRACE
    )

    def __content_str__(self) -> str:
        parts = [self.kind.bench_name]
        if self.type is not None:
            parts.append(self.type.bench_name)
        parts.append(self.title or "<no title>")
        return ", ".join(parts)

    @property
    def is_retryable(self) -> bool:
        return self.type is None or self.type.is_retryable

    @staticmethod
    def from_exception(kind: ErrorKind, e: BaseException) -> "Error":
        # NOTE :Incomplete: get run error trace/frames/node/...
        # pass on inner error if there is one
        if isinstance(getattr(e, "error", None), Error):
            return getattr(e, "error")  # manual error

        # title/text
        title = getattr(e, "title", None) or to_casing(
            e.__class__.__name__, Casing.CAMEL, allow_whitespace=True
        )
        if isinstance(getattr(e, "text", None), Text):
            text = getattr(e, "text")
        elif isinstance(e, SyntaxError):
            header_line = TextLine.plain(f"Syntax error at line {e.lineno}, column {e.offset}:")
            code_lines = Text.code(e.args[0])
            text = Text(lines=[header_line, *code_lines.lines])
        else:
            text = Text.plain(str(e))

        # kind/type
        if hasattr(e, "run_error_type"):
            typ = getattr(e, "run_error_type")
            assert isinstance(typ, ErrorType), f"unexpected {typ!r} from {e!r}"
        elif kind == ErrorKind.RUNTIME:
            if isinstance(e, CancelledError):
                typ = ErrorType.ABORTED
            elif isinstance(e, (TypeError, ValueError, ValidationError)):
                typ = ErrorType.INVALID_VALUE
            else:
                typ = ErrorType.NON_RETRYABLE
        else:
            typ = ErrorType.NON_RETRYABLE
        return Error(kind=kind, type=typ, title=title, text=text)
