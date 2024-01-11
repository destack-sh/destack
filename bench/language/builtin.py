import asyncio
import contextvars
import functools
from contextlib import contextmanager
from typing import TYPE_CHECKING, Optional
from uuid import uuid5

from asgiref.sync import async_to_sync

from bench.language.const import UUID_NAMESPACE, VERSION, BenchStatus
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


def _should_validate() -> bool:
    if DEBUG:
        return not _no_validation.get()
    else:
        return True


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


def _make_builtin_bench(name: str) -> tuple[Bench, Module]:
    # :BuiltinLibs
    bench_id = uuid5(UUID_NAMESPACE, f"builtin:{name}")
    bench = Bench(name=name, slug=name, id=bench_id, status=BenchStatus.AVAILABLE)
    module_id = uuid5(bench_id, VERSION)
    module = Module(parent=bench, id=module_id)
    return bench, module


# real data will be patched in at first runtime start
# nocheckin: patch in symbolx_bench at runtime
symbolx_bench, symbolx_lib = _make_builtin_bench("symbolx.bench")
DEFAULT_DEPENDENCIES = {symbolx_bench.slug: symbolx_lib.id}
