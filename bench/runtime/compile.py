from __future__ import annotations

import enum
from typing import Optional

import structlog

from bench.language import Statement

logger = structlog.get_logger(__name__)


class CompileErrorType(enum.Enum):
    INTERNAL = 0, "Internal error"
    RUN = 1, "Error running user code"

    def __new__(cls, value, description):
        obj = object.__new__(cls)
        obj._value_ = value
        obj.description = description
        return obj


class CompileError(ValueError):
    def __init__(
        self,
        _t: CompileErrorType,
        statement: Optional[Statement],
        cause: Optional[Exception] = None,
    ):
        self.type = _t
        self.statement = statement
        self.cause = cause
        super().__init__(self.type.description)


class ExpectationStatementType(enum.Enum):
    """
    The type of expectation defines its semantics.
    """

    GENERATE = "generate"
    TRANSFORM = "transform"
    VERIFY = "verify"
