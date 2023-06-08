from __future__ import annotations

import enum
import traceback
import typing
from typing import Any, Optional

from bench.language.const import LiteralValue
from bench.language.type import Symbol
from bench.runtime.common.type import PyFrameData
from bench.utils.utils import get_from_env, to_pyidentifier

if typing.TYPE_CHECKING:
    from bench.runtime.worker.instance import AsyncCodeInstance, CodeInstance, SyncCodeInstance

ALLOW_UNTRUSTED_CODE = get_from_env("ALLOW_UNTRUSTED_CODE", False, type_cast=bool)


def do_execute_arbitrary_code(code: str, globals: dict[str, Any]) -> dict:
    # remember the globals we started with, do not modify originals
    if not ALLOW_UNTRUSTED_CODE:
        raise RuntimeError("untrusted code execution is disabled")
    globals_local = {**globals}
    globals_local_keys_initial = {*globals_local.keys()}
    exec(code, globals_local)
    new_globals = {
        k: v
        for k, v in globals_local.items()
        if k not in globals_local_keys_initial and k not in ("__builtins__", "__annotations__")
    }
    return new_globals


class RunErrorType(enum.Enum):
    INTERNAL = 0, "Internal error"
    PARSE = 1, "Parse error"
    VALIDATION = 2, "Validation error"
    RUNTIME = 3, "Runtime code error"
    UNTRUSTED = 4, "Untrusted code error"

    def __new__(cls, value, description):
        obj = object.__new__(cls)
        obj._value_ = value
        obj.description = description
        return obj


class RunError(Exception):
    def __init__(
        self,
        _t: RunErrorType,
        symbol: typing.Optional[Symbol],
        cause: typing.Optional[Exception] = None,
    ):
        self.type = _t
        self.symbol = symbol
        self.cause = cause
        super().__init__(self.type.description)

    def get_traceback(self, from_code: CodeInstance) -> Optional[list[PyFrameData]]:
        if self.cause is None:
            return None
        stack_summary = traceback.StackSummary.extract(
            traceback.walk_tb(self.cause.__traceback__), capture_locals=True
        )
        stack = PyFrameData.from_stack(stack_summary)
        return PyFrameData.clean(stack, from_code)


def run_sync(
    code: SyncCodeInstance, arguments: dict[str, LiteralValue] | None = None
) -> LiteralValue:
    """Runs the code instance synchronously. Not to be used in production."""
    # transform keys to valid python identifiers
    arguments = {to_pyidentifier(k): v for k, v in (arguments or {}).items()}
    try:
        if code.is_async:
            raise RuntimeError(f"cannot run async code synchronously: {code}")
        return code(**arguments)
    except Exception as e:
        raise RunError(RunErrorType.RUNTIME, code, cause=e) from e


async def run(
    code: AsyncCodeInstance | SyncCodeInstance,
    arguments: dict[str, LiteralValue] | None = None,
    is_trusted: bool = False,
) -> LiteralValue:
    if not is_trusted and not ALLOW_UNTRUSTED_CODE:
        raise RunError(RunErrorType.UNTRUSTED, code)
    # transform keys to valid python identifiers
    arguments = {to_pyidentifier(k): v for k, v in (arguments or {}).items()}
    try:
        await code.session.prepare()
        code.session.open()
        if not code.is_async:
            code = code.to_async()
        ret = await code(**arguments)
        await code.session.aclose()
        return ret
    except Exception as e:
        raise RunError(RunErrorType.RUNTIME, code, cause=e) from e
