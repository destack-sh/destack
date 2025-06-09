import linecache

import black


def exec_(code: str, globals: dict, locals: dict, filename: str) -> None:
    """
    Executes the code, but with a name and in the cache.
    """
    code_co = compile(code, filename, "exec")
    assert filename not in linecache.cache, f"filename {filename} already in cache"
    linecache.cache[filename] = (
        len(code),  # size (ignored)
        None,  # mtime  (ignored)
        code.splitlines(True),  # list with trailing '\n'
        filename,
    )
    exec(code_co, globals, locals)


def format_code(code: str, suppress_error: bool = False, line_length: int = 100) -> str:
    """
    Formats the code string with our standard black settings.
    TODO :Performance!: replace black with ruff in format_code :BadCodeFormatting
     (unfortunately ruff doesn't have a nice API for this yet, so we would need to use a subprocess?)
    """
    try:
        return black.format_str(code, mode=black.FileMode(line_length=line_length))
    except Exception as e:
        if suppress_error:
            return code
        else:
            raise SyntaxError(code) from e
