# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_bool,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)


@dataclass(frozen=True, slots=True)
class ProtocolError:
    """Protocol errors returned in responses."""

    # error code classification
    code: ProtocolErrorCode
    # human readable error message
    message: str
    # optional structured detail string
    detail: str | None
    # whether the request can be retried safely
    retryable: bool
    # optional retry delay in milliseconds
    retry_after_ms: int | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_protocol_error(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ProtocolError:
        """Decode one ProtocolError."""
        return decode_protocol_error(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_protocol_error(self)

    @classmethod
    def from_json(cls, value: Json) -> ProtocolError:
        """Return one ProtocolError from one JSON value."""
        return from_json_protocol_error(value)


def encode_protocol_error(writer: BinaryWriter, value: ProtocolError) -> None:
    """Encode one ProtocolError."""
    encode_protocol_error_code(writer, value.code)
    writer.write_string(value.message)
    if value.detail is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.detail)
    writer.write_bool(value.retryable)
    if value.retry_after_ms is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.retry_after_ms)


def decode_protocol_error(reader: BinaryReader) -> ProtocolError:
    """Decode one ProtocolError."""
    code = decode_protocol_error_code(reader)
    message = reader.read_string()
    detail = reader.read_option(lambda: reader.read_string())
    retryable = reader.read_bool()
    retry_after_ms = reader.read_option(lambda: reader.read_number())

    return ProtocolError(
        code=code,
        message=message,
        detail=detail,
        retryable=retryable,
        retry_after_ms=retry_after_ms,
    )


def to_json_protocol_error(value: ProtocolError) -> Json:
    """Return one JSON value for one ProtocolError."""
    return {
        "code": to_json_protocol_error_code(value.code),
        "message": value.message,
        **({} if value.detail is None else {"detail": value.detail}),
        "retryable": value.retryable,
        **(
            {}
            if value.retry_after_ms is None
            else {"retryAfterMs": value.retry_after_ms}
        ),
    }


def from_json_protocol_error(value: Json) -> ProtocolError:
    """Return one ProtocolError from one JSON value."""
    object_ = json_object(value)

    return ProtocolError(
        code=from_json_protocol_error_code(json_field(object_, "code")),
        message=json_string(json_field(object_, "message")),
        detail=json_optional(object_, "detail", lambda value: json_string(value)),
        retryable=json_bool(json_field(object_, "retryable")),
        retry_after_ms=json_optional(
            object_, "retryAfterMs", lambda value: json_int(value)
        ),
    )


"""Error code classification for protocol errors."""
ProtocolErrorCode: typing.TypeAlias = (
    typing.Literal["invalidRequest"]
    | typing.Literal["invalidPayload"]
    | typing.Literal["unsupportedVersion"]
    | typing.Literal["notFound"]
    | typing.Literal["conflict"]
    | typing.Literal["busy"]
    | typing.Literal["notReady"]
    | typing.Literal["timeout"]
    | typing.Literal["canceled"]
    | typing.Literal["tooLarge"]
    | typing.Literal["unauthorized"]
    | typing.Literal["forbidden"]
    | typing.Literal["internal"]
)


def encode_protocol_error_code(writer: BinaryWriter, value: ProtocolErrorCode) -> None:
    """Encode one ProtocolErrorCode."""
    if value == "invalidRequest":
        writer.write_unsigned(0)
    elif value == "invalidPayload":
        writer.write_unsigned(1)
    elif value == "unsupportedVersion":
        writer.write_unsigned(2)
    elif value == "notFound":
        writer.write_unsigned(3)
    elif value == "conflict":
        writer.write_unsigned(4)
    elif value == "busy":
        writer.write_unsigned(5)
    elif value == "notReady":
        writer.write_unsigned(6)
    elif value == "timeout":
        writer.write_unsigned(7)
    elif value == "canceled":
        writer.write_unsigned(8)
    elif value == "tooLarge":
        writer.write_unsigned(9)
    elif value == "unauthorized":
        writer.write_unsigned(10)
    elif value == "forbidden":
        writer.write_unsigned(11)
    elif value == "internal":
        writer.write_unsigned(12)
    else:
        raise SerdeError("unknown enum variant")


def decode_protocol_error_code(reader: BinaryReader) -> ProtocolErrorCode:
    """Decode one ProtocolErrorCode."""
    variant = reader.read_number()

    if variant == 0:
        return "invalidRequest"
    elif variant == 1:
        return "invalidPayload"
    elif variant == 2:
        return "unsupportedVersion"
    elif variant == 3:
        return "notFound"
    elif variant == 4:
        return "conflict"
    elif variant == 5:
        return "busy"
    elif variant == 6:
        return "notReady"
    elif variant == 7:
        return "timeout"
    elif variant == 8:
        return "canceled"
    elif variant == 9:
        return "tooLarge"
    elif variant == 10:
        return "unauthorized"
    elif variant == 11:
        return "forbidden"
    elif variant == 12:
        return "internal"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_protocol_error_code(value: ProtocolErrorCode) -> Json:
    """Return one JSON value for one ProtocolErrorCode."""
    return value


def from_json_protocol_error_code(value: Json) -> ProtocolErrorCode:
    """Return one ProtocolErrorCode from one JSON value."""
    variant = json_string(value)

    if variant == "invalidRequest":
        return "invalidRequest"
    elif variant == "invalidPayload":
        return "invalidPayload"
    elif variant == "unsupportedVersion":
        return "unsupportedVersion"
    elif variant == "notFound":
        return "notFound"
    elif variant == "conflict":
        return "conflict"
    elif variant == "busy":
        return "busy"
    elif variant == "notReady":
        return "notReady"
    elif variant == "timeout":
        return "timeout"
    elif variant == "canceled":
        return "canceled"
    elif variant == "tooLarge":
        return "tooLarge"
    elif variant == "unauthorized":
        return "unauthorized"
    elif variant == "forbidden":
        return "forbidden"
    elif variant == "internal":
        return "internal"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "ProtocolError",
    "encode_protocol_error",
    "decode_protocol_error",
    "to_json_protocol_error",
    "from_json_protocol_error",
    "ProtocolErrorCode",
    "encode_protocol_error_code",
    "decode_protocol_error_code",
    "to_json_protocol_error_code",
    "from_json_protocol_error_code",
]
