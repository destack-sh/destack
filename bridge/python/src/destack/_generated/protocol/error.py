# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes


@dataclass(frozen=True, slots=True)
class ProtocolError:
    """Protocol errors returned in responses."""

    """Error code classification."""
    code: ProtocolErrorCode
    """Human readable error message."""
    message: str
    """Optional structured detail string."""
    detail: str | None
    """Whether the request can be retried safely."""
    retryable: bool
    """Optional retry delay in milliseconds."""
    retry_after_ms: int | None


def encode_protocol_error(writer: Writer, value: ProtocolError) -> None:
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


def decode_protocol_error(reader: Reader) -> ProtocolError:
    field_0 = decode_protocol_error_code(reader)
    field_1 = reader.read_string()
    field_2 = reader.read_option(lambda: reader.read_string())
    field_3 = reader.read_bool()
    field_4 = reader.read_option(lambda: reader.read_number())

    return ProtocolError(
        code=field_0,
        message=field_1,
        detail=field_2,
        retryable=field_3,
        retry_after_ms=field_4,
    )


"""Error code classification for protocol errors."""
ProtocolErrorCode: TypeAlias = (
    Literal["invalidRequest"]
    | Literal["invalidPayload"]
    | Literal["unsupportedVersion"]
    | Literal["notFound"]
    | Literal["conflict"]
    | Literal["busy"]
    | Literal["notReady"]
    | Literal["timeout"]
    | Literal["canceled"]
    | Literal["tooLarge"]
    | Literal["unauthorized"]
    | Literal["forbidden"]
    | Literal["internal"]
)


def encode_protocol_error_code(writer: Writer, value: ProtocolErrorCode) -> None:
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


def decode_protocol_error_code(reader: Reader) -> ProtocolErrorCode:
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


__all__ = [
    "ProtocolError",
    "encode_protocol_error",
    "decode_protocol_error",
    "ProtocolErrorCode",
    "encode_protocol_error_code",
    "decode_protocol_error_code",
]
