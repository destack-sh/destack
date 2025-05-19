import black


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
