import contextvars
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Any,
    Optional,
    cast,
)

from destack.utils.env import get_from_env
from destack.utils.frozen import frozendict
from destack.utils.uuid import UUID, uuid4

from .common import Cloud, Region

if TYPE_CHECKING:
    from destack.language import Session


class _Unset:
    def __repr__(self):
        return "<UNSET!>"

    def __str__(self):
        return "<UNSET!>"


# forever constants
VERSION = "2025.07.01.0"
FLOAT_EPSILON = 1e-6
BEGINNING_OF_TIME = datetime.fromisoformat("1970-01-01T00:00:00+00:00")

# builtin destackes :Builtins
DESTACK_SLUG = "destack"
DESTACK_ID = UUID("11111111-1111-1111-1111-000000000000")

# runtime constants
NONCE = uuid4()
UNSET = cast(Any, _Unset())
EMPTY_LIST: list = []
EMPTY_SET: frozenset = frozenset()
EMPTY_DICT: dict[Any, Any] = frozendict()

# runtime context
IS_IN_USER_CODE = contextvars.ContextVar("is_in_user_code", default=False)
ACTIVE_SESSION: contextvars.ContextVar[Optional["Session"]] = contextvars.ContextVar(
    "active_session", default=None
)


def active_session() -> "Session":
    """Gets the currently active Session (error if none)."""
    session = ACTIVE_SESSION.get()
    assert session is not None, "no active session"
    return session


def get_active_session() -> Optional["Session"]:
    """Gets the currently active Session (if any)."""
    return ACTIVE_SESSION.get()


CLOUD = get_from_env("CLOUD", typ=Cloud, description="Cloud we're running in")
REGION = get_from_env("REGION", typ=Region, description="Region we're running in")
TRACING = get_from_env("TRACING", typ=bool, description="Enable tracing")


class DestackError(Exception):
    """Common base class for any regular errors."""

    pass
