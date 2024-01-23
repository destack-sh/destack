import contextvars
from contextlib import contextmanager
from typing import TYPE_CHECKING, Optional
from uuid import uuid5

from bench.language.const import UUID_NAMESPACE, VERSION
from bench.language.node import Bench, Module
from bench.utils.utils import DEBUG

if TYPE_CHECKING:
    from bench.language.session import Session

#
# Things we need to import (almost) everywhere with minimal circular conflicts.
#

_active_session: contextvars.ContextVar[Optional["Session"]] = contextvars.ContextVar(
    "active_session", default=None
)
if DEBUG:
    _no_validation: contextvars.ContextVar[bool] = contextvars.ContextVar(
        "_no_validation", default=False
    )


def active_session() -> "Session":
    session = _active_session.get()
    assert session is not None, "no active session"
    return session


@contextmanager
def _without_validation() -> None:
    if DEBUG:
        token = _no_validation.set(True)
        try:
            yield
        finally:
            _no_validation.reset(token)
    else:
        raise RuntimeError("cannot disable validation outside debug mode")


# real data will be patched in at first runtime start
# nocheckin: patch in symbolx_bench at runtime start
symbolx_bench = Bench(
    name="SymbolX", slug="symbolx", id=uuid5(UUID_NAMESPACE, f"builtin:symbolx.bench")
)
symbolx_lib = Module(parent=symbolx_bench, id=uuid5(symbolx_bench.id, VERSION))
DEFAULT_DEPENDENCIES = {symbolx_bench.slug: symbolx_lib.id}
