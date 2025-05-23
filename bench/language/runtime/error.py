from asyncio import CancelledError
from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    BenchError,
    BuiltinEnum,
    EnumType,
    ErrorKind,
    Node,
    Struct,
    StructType,
    ValidationError,
    enum_,
    property_,
    struct_,
)
from bench.utils.string import Casing, to_casing

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false


@struct_(StructType.RUN_TRACE)
class RunTrace(Struct):
    """A stacktrace for a Run."""

    frames: list["RunFrame"] = property_(30)


@struct_(StructType.RUN_FRAME)
class RunFrame(Struct):
    """A single frame in a stacktrace."""

    pass


@enum_(EnumType.ERROR_TYPE)
class ErrorType(BuiltinEnum):
    # unretryable
    ABORTED = 2
    RUNTIME_UNAVAILABLE = 3
    RUN_IMPOSSIBLE = 4
    NOT_SUPPORTED = 5
    INVALID_VALUE = 10
    INVALID_COMPUTED = 11
    CODE_INVALID = 20
    TEXT_INVALID = 21
    INCAPABLE = 100
    REFUSED = 101
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

    kind: ErrorKind = property_(30)
    type: ErrorType = property_(31, is_repr=True)
    title: str | None = property_(32, is_repr=True)
    text: str | None = property_(33)
    nodes: list["Node"] = property_(34)
    trace: Optional[RunTrace] = property_(35)

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
        text = str(e)

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
