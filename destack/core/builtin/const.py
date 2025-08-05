import contextvars
from typing import TYPE_CHECKING, Any, Optional, cast

from ..utility import frozendict, uuid7

if TYPE_CHECKING:
    from destack import Session


class _Unset:
    def __repr__(self):
        return "<UNSET!>"

    def __str__(self):
        return "<UNSET!>"


VERSION = "2025.08.01.22"
EPSILON = 1e-6
EPSILON_EXPONENT = 6

METAKIND_PROPERTY_ID = 0
METAKIND_PROPERTY_KEY = str(METAKIND_PROPERTY_ID)
METATYPE_PROPERTY_ID = 1
METATYPE_PROPERTY_KEY = str(METATYPE_PROPERTY_ID)

NONCE = uuid7()
UNSET = cast(Any, _Unset())
EMPTY_LIST: list = []
EMPTY_SET: frozenset = frozenset()
EMPTY_DICT: dict[Any, Any] = frozendict()

ACTIVE_SESSION: contextvars.ContextVar[Optional["Session"]] = contextvars.ContextVar(
    "active_session", default=None
)
