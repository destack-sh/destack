import linecache
import time

_time_spent_in_exec = 0


def exec_(code: str, globals: dict, locals: dict, filename: str, log: bool = False) -> None:
    """
    Executes the code, but with a name and in the cache.
    """
    global _time_spent_in_exec
    start = time.time()
    if log:
        print("=" * 80)  # noqa: T201
        print(filename)  # noqa: T201
        print("=" * 80)  # noqa: T201
        print(code)  # noqa: T201
        print("=" * 80)  # noqa: T201
    assert filename not in linecache.cache, f"filename {filename} already in cache"
    linecache.cache[filename] = (
        len(code),  # size (ignored)
        None,  # mtime  (ignored)
        code.splitlines(True),  # list with trailing '\n'
        filename,
    )
    exec(code, globals, locals)
    _time_spent_in_exec += time.time() - start
