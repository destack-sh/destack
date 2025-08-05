import contextvars
from typing import (
    TYPE_CHECKING,
    Any,
    Optional,
    cast,
)

from ..utility import frozendict, uuid4

if TYPE_CHECKING:
    from destack import (
        Session,
    )


class _Unset:
    def __repr__(self):
        return "<UNSET!>"

    def __str__(self):
        return "<UNSET!>"


# forever constants
VERSION = "2025.08.01.22"
EPSILON = 1e-6
EPSILON_EXPONENT = 6
METAKIND_PROPERTY_ID = 0
METAKIND_PROPERTY_KEY = str(METAKIND_PROPERTY_ID)
METATYPE_PROPERTY_ID = 1
METATYPE_PROPERTY_KEY = str(METATYPE_PROPERTY_ID)

# runtime constants
NONCE = uuid4()
UNSET = cast(Any, _Unset())
EMPTY_LIST: list = []
EMPTY_SET: frozenset = frozenset()
EMPTY_DICT: dict[Any, Any] = frozendict()

# runtime context
ACTIVE_SESSION: contextvars.ContextVar[Optional["Session"]] = contextvars.ContextVar(
    "active_session", default=None
)


def get_active_session() -> Optional["Session"]:
    """Gets the currently active Session (if any)."""
    return ACTIVE_SESSION.get()


def active_session() -> "Session":
    """Gets the currently active Session (error if none)."""
    session = ACTIVE_SESSION.get()
    assert session is not None, "no active session"
    return session


class DestackError(Exception):
    """Common base class for any regular errors."""

    pass
