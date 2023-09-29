import contextvars
import os
from typing import Optional, TYPE_CHECKING
from uuid import uuid5

from bench.language.const import BENCH_UUID_NAMESPACE
from bench.language.module import Module

if TYPE_CHECKING:
    from bench.language.session import Session

active_session: contextvars.ContextVar[Optional["Session"]] = contextvars.ContextVar(
    "active_session", default=None
)


#
# Common base for builtin libraries for reference outside libs.
# (libs requires more imports than just the basics)
#


def _make_builtin_lib_module(name: str) -> Module:
    # :BuiltinLibs
    ck = uuid5(BENCH_UUID_NAMESPACE, f"builtin:{name}")
    id = uuid5(ck, os.environ["VERSION"])
    return Module(name=name, ck=ck, id=id)


symbolx_lib = _make_builtin_lib_module("symbolx.lib")
openai_lib = _make_builtin_lib_module("openai.lib")
anthropic_lib = _make_builtin_lib_module("anthropic.lib")
