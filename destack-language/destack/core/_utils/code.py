import linecache


def execute_arbitrary_code(
    code: str,
    globals: dict,
    locals: dict,
    filename: str,
    _debug_log: bool = False,
) -> None:
    """
    Executes the code, but with a name and in the cache.
    """
    if _debug_log:
        print("=" * 80)  # noqa: T201
        print(filename)  # noqa: T201
        print("-" * 80)  # noqa: T201
        print(code)  # noqa: T201
        print("-" * 80)  # noqa: T201
    assert filename not in linecache.cache, f"filename {filename} already in cache"
    linecache.cache[filename] = (
        len(code),  # size (ignored)
        None,  # mtime  (ignored)
        code.splitlines(True),  # list with trailing '\n'
        filename,
    )
    exec(code, globals, locals)
