import asyncio
import contextvars
import functools
import os
from contextlib import contextmanager
from typing import TYPE_CHECKING, Optional
from uuid import uuid5

from bench.language.const import BENCH_UUID_NAMESPACE
from bench.language.module import Module
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


def _auto_async_to_sync(func=None):
    """Automatically convert async functions to sync within a session if called in sync context."""

    def decorate(func):
        # check that the func is async
        if not asyncio.iscoroutinefunction(func):
            raise TypeError(f"{func} is not a coroutine function")

        @functools.wraps(func)
        def wrapped(*args, **kwargs):
            # are we in an aysnc context?
            session = _active_session.get()
            try:
                asyncio.get_running_loop()
                is_in_loop = True
            except RuntimeError:
                is_in_loop = False
            if (
                is_in_loop
                and session is None
                or session.current_run is None
                or session.current_run._is_async
            ):
                return func(*args, **kwargs)
            else:
                return session.async_to_sync(func)(*args, **kwargs)

        return wrapped

    if func is None:
        return decorate
    else:
        return decorate(func)


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


def _make_builtin_lib_module(name: str) -> Module:
    # :BuiltinLibs
    ck = uuid5(BENCH_UUID_NAMESPACE, f"builtin:{name}")
    id = uuid5(ck, os.environ["VERSION"])
    return Module(name=name, ck=ck, id=id, _project_id=ck)


symbolx_lib = _make_builtin_lib_module("symbolx.lib")
openai_lib = _make_builtin_lib_module("openai.lib")
anthropic_lib = _make_builtin_lib_module("anthropic.lib")
deepgram_lib = _make_builtin_lib_module("deepgram.lib")
