import base64
from datetime import datetime, timedelta
from typing import Any
from uuid import UUID

_CODE_GLOBALS: dict[str, Any] | None = None


def get_code_globals():
    global _CODE_GLOBALS
    if _CODE_GLOBALS is None:
        import bench.language
        from bench.language.setup import BENCH_CLASS_BY_NAME

        # all bench types
        _CODE_GLOBALS = {**vars(bench.language), **BENCH_CLASS_BY_NAME}
        # and some general stuff
        for t in (datetime, timedelta, UUID, base64):
            _CODE_GLOBALS[t.__name__] = t
    return _CODE_GLOBALS


def run_code_script(code: str, extra_globals: dict[str, Any] | None = None) -> dict[str, Any]:
    """Runs the code string and extracts its definitions."""
    globals_initial = {**get_code_globals(), **(extra_globals or {})}
    globals_local = {**globals_initial}
    exec(code, globals_local)
    new_globals = {k: v for k, v in globals_local.items() if k not in globals_initial}
    return new_globals


def run_code_eval(code: str, extra_globals: dict[str, Any] | None = None) -> Any:
    """Runs the code string and extracts its result."""
    globals_local = {**get_code_globals(), **(extra_globals or {})}
    return eval(code, globals_local)


def run_code_exec(code: str, extra_globals: dict[str, Any] | None = None) -> None:
    """Runs the code string."""
    globals_local = {**get_code_globals(), **(extra_globals or {})}
    exec(code, globals_local)
