import linecache
import time

_time_spent_in_exec = 0


def exec_(code: str, globals: dict, locals: dict, filename: str, log: bool = False) -> None:
    """
    Executes the code, but with a name and in the cache.
    """
    global _time_spent_in_exec
    start = time.time()
    code_co = compile(code, filename, "exec")
    assert filename not in linecache.cache, f"filename {filename} already in cache"
    linecache.cache[filename] = (
        len(code),  # size (ignored)
        None,  # mtime  (ignored)
        code.splitlines(True),  # list with trailing '\n'
        filename,
    )
    if log:
        print("=" * 80)  # noqa: T201
        print(filename)  # noqa: T201
        print("=" * 80)  # noqa: T201
        print(code)  # noqa: T201
        print("=" * 80)  # noqa: T201
    exec(code_co, globals, locals)
    _time_spent_in_exec += time.time() - start


def format_code(code: str, suppress_error: bool = False, line_length: int = 100) -> str:
    """
    Formats the code string with our standard black settings.
    TODO :Performance!: replace black with ruff in format_code
     (unfortunately ruff doesn't have a nice API for this yet, so we would need to use a subprocess?)
    """
    try:
        import black

        return black.format_str(code, mode=black.FileMode(line_length=line_length))
    except Exception as e:
        if suppress_error:
            return code
        else:
            raise SyntaxError(code) from e
