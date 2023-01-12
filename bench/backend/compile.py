from __future__ import annotations

import enum

import structlog

logger = structlog.get_logger(__name__)


class ExpectationStatementType(enum.Enum):
    """
    The type of expectation defines its semantics.
    """

    GENERATE = "generate"
    TRANSFORM = "transform"
    VERIFY = "verify"
