import itertools
import types
import typing
from dataclasses import dataclass
from typing import Any, Optional

import more_itertools
import structlog
from more_itertools import first, last

from bench.language.const import StructType
from bench.language.field import TypedDict
from bench.language.node import Node, p_runtime, struct, Struct, p_regular
from bench.utils.utils import get_from_env

if typing.TYPE_CHECKING:
    from bench.language import Block
    from bench.language.notice import NoticeHandler

logger = structlog.get_logger(__name__)


@dataclass
class CodeTransformation:
    original_code: str
    transformed_code: str
    method_name: str
    start_offset: int
    end_offset: int


@struct(StructType.CODE_SECTION)
class CodeSection(Struct):
    # language: ...
    # kind: ...
    code: str = p_regular(32)


@struct(StructType.CODE)
class Code(Struct):
    sections: list[CodeSection] = p_regular(
        30, require=True, array=True, default_factory=list, struct=StructType.CODE_SECTION
    )

    _is_async: Optional[bool] = p_runtime(default=None)
    _transform: Optional[CodeTransformation] = p_runtime(default=None)
    _block_references: dict[str, "Block"] | None = p_runtime(default=None)
    _cached_exports: dict[str, Any] | None = p_runtime(default=None)

    def _clear_inner(self, scope: Optional[Node] = None) -> None:
        self._transform = None
        self._block_references = None
        self._callable_wrapped = None
        self._cached_exports = None

    def _interp_inner(self, scope: Node, on_notice: "NoticeHandler") -> None:
        self._is_async = "await " in self.code
        self._block_references = {}
        self._code_export_references = {}
        for key, reference in self._parse.references.items():
            resolved = scope.lookup(reference)
            if resolved is not None:
                self._block_references[key] = resolved

    def _untrack_inner(self) -> None:
        self._callable_wrapped = None  # locals are bound to session

    async def _call_inner_async(self, *args, **kwargs):
        inputs = self._inputs_from_args(args, kwargs)
        session = self.session
        session._run_enter(self, is_async=True, inputs=inputs)
        try:
            self._prepare_callable()
            result = await self._callable_wrapped(*args, **kwargs)
        except BaseException as exception:
            session._run_exception(self, exception)
            raise
        session._run_exit(self, result if not self._export else None)
        return _to_outputs_dict(self, result)

    def _call_inner_sync(self, *args, **kwargs):
        inputs = self._inputs_from_args(args, kwargs)
        session = self.session
        session._run_enter(self, is_async=False, inputs=inputs)
        try:
            self._prepare_callable()
            result = self._callable_wrapped(*args, **kwargs)
        except BaseException as exception:
            session._run_exception(self, exception)
            raise
        session._run_exit(self, result if not self._export else None)
        return _to_outputs_dict(self, result)


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


def _to_outputs_dict(code: "HasCode", result: Any) -> TypedDict:
    if isinstance(result, TypedDict):
        return result
    elif result is None:
        return TypedDict({}, code, is_output=True)
    else:
        return TypedDict(result, code, is_output=True)


def _install_package(name: str, timeout: int = 300, try_import: str = None) -> None:
    """Helper to install a package in the current server. Not in lib because it feels wrong."""
    if try_import:
        try:
            __import__(try_import)
            return
        except ImportError:
            pass

    import subprocess
    import sys

    subprocess.run(
        [sys.executable, "-m", "pip", "install", name],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
    )
