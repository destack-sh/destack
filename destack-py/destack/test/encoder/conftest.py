from typing import Any

from destack.language import BinaryReader, BinaryWriter, Encoder, Graph, Session

# ruff: noqa: T201

_SILENCE_TYPES = (
    Session,
    Graph,
    Encoder,
    BinaryReader,
    BinaryWriter,
)


class LoggingBinaryWriter(BinaryWriter):
    """A BinaryWriter that logs all writes to stdout."""

    def __init__(self) -> None:
        super().__init__()
        # dynamically wrap all write_* methods
        for attr_name in dir(self):
            if attr_name.startswith("write_"):
                original_method = getattr(self, attr_name)
                if callable(original_method):
                    setattr(
                        self,
                        attr_name,
                        self._make_write_logging_wrapper(attr_name, original_method),
                    )

    def _make_write_logging_wrapper(self, method_name: str, original_method):
        def wrapper(*args, **kwargs):
            print(method_name, *(a for a in args if not isinstance(a, _SILENCE_TYPES)))
            return original_method(*args, **kwargs)

        return wrapper


class LoggingBinaryReader(BinaryReader):
    """A BinaryReader that logs all reads to stdout."""

    def __init__(self, data: bytes) -> None:
        super().__init__(data)
        # dynamically wrap all read_* and peek_* methods
        for attr_name in dir(self):
            if attr_name.startswith(("read_", "peek_")):
                original_method = getattr(self, attr_name)
                if callable(original_method):
                    setattr(
                        self, attr_name, self._make_read_logging_wrapper(attr_name, original_method)
                    )

    def _make_read_logging_wrapper(self, method_name: str, original_method):
        def wrapper(*args, **kwargs):
            result = original_method(*args, **kwargs)
            print(
                method_name, *(a for a in args if not isinstance(a, _SILENCE_TYPES)), "->", result
            )
            return result

        return wrapper


def wrap_encoder(encoder: Encoder[Any]) -> Encoder[Any]:
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
