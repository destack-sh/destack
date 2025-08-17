from destack import BinaryDecoder, BinaryEncoder, Encoder, Graph, Session

# ruff: noqa: T201

_SILENCE_TYPES = (
    Session,
    Graph,
    Encoder,
    BinaryDecoder,
    BinaryEncoder,
)


def wrap_binary_encoder(encoder: BinaryEncoder) -> BinaryEncoder:
    """Wrap a binary encoder instance to log all method calls."""
    # dynamically wrap all write_* methods
    for attr_name in dir(encoder):
        if attr_name.startswith("write_") and callable(getattr(encoder, attr_name)):
            original_method = getattr(encoder, attr_name)
            setattr(
                encoder,
                attr_name,
                _make_logging_wrapper(attr_name, original_method),
            )
    return encoder


def wrap_binary_decoder(decoder: BinaryDecoder) -> BinaryDecoder:
    """Wrap a binary decoder instance to log all method calls."""
    # dynamically wrap all read_* and peek_* methods
    for attr_name in dir(decoder):
        if attr_name.startswith(("read_", "peek_")) and callable(getattr(decoder, attr_name)):
            original_method = getattr(decoder, attr_name)
            setattr(
                decoder,
                attr_name,
                _make_logging_wrapper(attr_name, original_method),
            )
    return decoder


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
