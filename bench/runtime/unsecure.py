from typing import Any


def do_execute_arbitrary_code(code: str, globals: dict[str, Any]) -> dict:
    # remember the globals we started with, do not modify originals
    globals_local = {**globals}
    globals_local_keys_initial = {*globals_local.keys()}
    exec(code, globals_local)
    new_globals = {
        k: v
        for k, v in globals_local.items()
        if k not in globals_local_keys_initial and k not in ("__builtins__", "__annotations__")
    }
    return new_globals
