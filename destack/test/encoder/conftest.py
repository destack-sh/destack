from destack import BinaryReader, BinaryWriter, Encoder, Graph, Session

# ruff: noqa: T201

_SILENCE_TYPES = (
    Session,
    Graph,
    Encoder,
    BinaryReader,
    BinaryWriter,
)


def wrap_binary_writer(writer: BinaryWriter) -> BinaryWriter:
    """Wrap a binary writer instance to log all method calls."""
    # dynamically wrap all write_* methods
    for attr_name in dir(writer):
        if attr_name.startswith("write_") and callable(getattr(writer, attr_name)):
            original_method = getattr(writer, attr_name)
            setattr(
                writer,
                attr_name,
                _make_logging_wrapper(attr_name, original_method),
            )
    return writer


def wrap_binary_reader(reader: BinaryReader) -> BinaryReader:
    """Wrap a binary reader instance to log all method calls."""
    # dynamically wrap all read_* and peek_* methods
    for attr_name in dir(reader):
        if attr_name.startswith(("read_", "peek_")) and callable(getattr(reader, attr_name)):
            original_method = getattr(reader, attr_name)
            setattr(
                reader,
                attr_name,
                _make_logging_wrapper(attr_name, original_method),
            )
    return reader


def wrap_encoder(encoder: Encoder) -> Encoder:
    """Wrap an encoder instance to log all method calls."""
    # dynamically wrap all public methods
    for attr_name in dir(encoder):
        if not attr_name.startswith("_") and callable(getattr(encoder, attr_name)):
            original_method = getattr(encoder, attr_name)
            setattr(
                encoder,
                attr_name,
                _make_logging_wrapper(attr_name, original_method),
            )
    return encoder


def _make_logging_wrapper(method_name: str, original_method):
    def wrapper(*args, **kwargs):
        print(
            f"{method_name}({', '.join(repr(arg) for arg in args if not isinstance(arg, _SILENCE_TYPES))}, {', '.join(f'{k}={v!r}' for k, v in kwargs.items() if not isinstance(v, _SILENCE_TYPES))})"
        )
        result = original_method(*args, **kwargs)
        print(f"{method_name} -> {result!r}")
        return result

    return wrapper
