import contextvars
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
    return Module(name=name, ck=ck, id=id)


symbolx_lib = _make_builtin_lib_module("symbolx.lib")
openai_lib = _make_builtin_lib_module("openai.lib")
anthropic_lib = _make_builtin_lib_module("anthropic.lib")
