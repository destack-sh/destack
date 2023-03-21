import enum
import typing

from bench.language import XBlock
from bench.language.type import XKind, XSource


class GenerationErrorType(enum.Enum):
    INTERNAL = 0, "internal error"
    EXCEEDED_CONTEXT = 1, "ran out of tokens"
    INVALID_OUTPUT = 2, "invalid output"

    def __init__(self, code: int, message: str):
        self.code = code
        self.message = message


class GenerationError(ValueError):
    def __init__(
        self,
        _t: GenerationErrorType,
        message_detail: str | None = None,
        cause: Exception | None = None,
    ):
        super().__init__(_t.message)
        self.type = _t
        self.cause = cause
        self.message_detail = message_detail


ValueT = typing.TypeVar("ValueT", bound=typing.Any)


def xsettings(value: ValueT, source: XSource = XSource.System) -> XBlock[ValueT]:
    return XBlock(kind=XKind.Settings, source=source, value=value)


def xstatic(value: ValueT, source: XSource = XSource.Developer) -> XBlock[ValueT]:
    return XBlock(kind=XKind.Static, source=source, value=value)


def xinput(value: ValueT, source: XSource = XSource.User) -> XBlock[ValueT]:
    return XBlock(kind=XKind.Input, source=source, value=value)


def xoutput(value: ValueT, source: XSource = XSource.Model) -> XBlock[ValueT]:
    return XBlock(kind=XKind.Output, source=source, value=value)
