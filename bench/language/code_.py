import itertools
import types
import typing
from typing import Any

import more_itertools
import structlog
from more_itertools import first, last

from bench.language.const import StructType
from bench.language.node import Struct, struct
from bench.language.property import p_regular
from bench.utils.utils import get_from_env

if typing.TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)


@struct(StructType.CODE_LINE)
class CodeLine(Struct):
    content: str = p_regular(32)


@struct(StructType.CODE)
class Code(Struct):
    # language: ...
    lines: list[CodeLine] = p_regular(30, require=True, array=True, struct=StructType.CODE_LINE)

    async def _call_component_async(self, *args, **kwargs):
        raise NotImplementedError

    def _call_component_sync(self, *args, **kwargs):
        raise NotImplementedError


AsyncCodeCallable = typing.Callable[..., typing.Coroutine]
SyncCodeCallable = typing.Callable[..., Any]

STATIC_BUILTINS: dict[str, Any] = {
    # functional builtins
    "first": first,
    "last": last,
    "batched": more_itertools.batched,
    "chain": itertools.chain,
}
DYNAMIC_BUILTINS: set[str] = {"builtins", "session", "cache", "random", "self"}
ALLOW_UNTRUSTED_CODE = get_from_env("ALLOW_UNTRUSTED_CODE", False, type_cast=bool)


def _do_exec_get_globals(code: str | types.CodeType, globals: dict[str, Any]) -> dict:
    # remember the globals we started with, do not modify originals
    if not ALLOW_UNTRUSTED_CODE:
        raise RuntimeError("untrusted code execution is disabled")
    globals_local = {**globals}
    exec(code, globals_local)
    return globals_local
