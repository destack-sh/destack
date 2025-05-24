from typing import TYPE_CHECKING

from bench.language.core import (
    BenchError,
    BuiltinEnum,
    EnumType,
    Node,
    Struct,
    StructType,
    enum_,
    property_,
    struct_,
)

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false
# TODO: proper Errors


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
    INTERRUPTION_CANCELLED = 500
    MODEL_FAILED = 501

    @property
    def is_retryable(self) -> bool:
        """Whether this error type is retryable *at runtime*"""
        return self > 500


@struct_(StructType.ERROR)
class Error(Struct, BenchError):
    """An error that occurred in the context of a Run."""

    type: ErrorType = property_(30, is_repr=True)
    title: str | None = property_(32, is_repr=True)
    text: str | None = property_(33)
    nodes: list["Node"] = property_(34)

    @property
    def is_retryable(self) -> bool:
        return self.type is None or self.type.is_retryable
