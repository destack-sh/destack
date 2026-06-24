# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.repository.config.condition


@dataclass(frozen=True, slots=True)
class Export:
    """Public package material declaration."""

    # exported material kind
    kind: ExportKind
    # package relative material path
    path: str
    # condition predicate required for this export
    when: destack._generated.repository.config.condition.ConditionRef | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_export(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Export:
        """Decode one Export."""
        return decode_export(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_export(self)

    @classmethod
    def from_json(cls, value: Json) -> Export:
        """Return one Export from one JSON value."""
        return from_json_export(value)


def encode_export(writer: BinaryWriter, value: Export) -> None:
    """Encode one Export."""
    encode_export_kind(writer, value.kind)
    writer.write_string(value.path)
    if value.when is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.repository.config.condition.encode_condition_ref(
            writer, value.when
        )


def decode_export(reader: BinaryReader) -> Export:
    """Decode one Export."""
    kind = decode_export_kind(reader)
    path = reader.read_string()
    when = reader.read_option(
        lambda: destack._generated.repository.config.condition.decode_condition_ref(
            reader
        )
    )

    return Export(
        kind=kind,
        path=path,
        when=when,
    )


def to_json_export(value: Export) -> Json:
    """Return one JSON value for one Export."""
    return {
        "kind": to_json_export_kind(value.kind),
        "path": value.path,
        **(
            {}
            if value.when is None
            else {
                "when": destack._generated.repository.config.condition.to_json_condition_ref(
                    value.when
                )
            }
        ),
    }


def from_json_export(value: Json) -> Export:
    """Return one Export from one JSON value."""
    object_ = json_object(value)

    return Export(
        kind=from_json_export_kind(json_field(object_, "kind")),
        path=json_string(json_field(object_, "path")),
        when=json_optional(
            object_,
            "when",
            lambda value: (
                destack._generated.repository.config.condition.from_json_condition_ref(
                    value
                )
            ),
        ),
    )


"""Public package material kind."""
ExportKind: typing.TypeAlias = (
    typing.Literal["module"]
    | typing.Literal["asset"]
    | typing.Literal["template"]
    | typing.Literal["reflect"]
    | typing.Literal["simulation"]
    | typing.Literal["service"]
    | typing.Literal["app"]
)


def encode_export_kind(writer: BinaryWriter, value: ExportKind) -> None:
    """Encode one ExportKind."""
    if value == "module":
        writer.write_unsigned(0)
    elif value == "asset":
        writer.write_unsigned(1)
    elif value == "template":
        writer.write_unsigned(2)
    elif value == "reflect":
        writer.write_unsigned(3)
    elif value == "simulation":
        writer.write_unsigned(4)
    elif value == "service":
        writer.write_unsigned(5)
    elif value == "app":
        writer.write_unsigned(6)
    else:
        raise SerdeError("unknown enum variant")


def decode_export_kind(reader: BinaryReader) -> ExportKind:
    """Decode one ExportKind."""
    variant = reader.read_number()

    if variant == 0:
        return "module"
    elif variant == 1:
        return "asset"
    elif variant == 2:
        return "template"
    elif variant == 3:
        return "reflect"
    elif variant == 4:
        return "simulation"
    elif variant == 5:
        return "service"
    elif variant == 6:
        return "app"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_export_kind(value: ExportKind) -> Json:
    """Return one JSON value for one ExportKind."""
    return value


def from_json_export_kind(value: Json) -> ExportKind:
    """Return one ExportKind from one JSON value."""
    variant = json_string(value)

    if variant == "module":
        return "module"
    elif variant == "asset":
        return "asset"
    elif variant == "template":
        return "template"
    elif variant == "reflect":
        return "reflect"
    elif variant == "simulation":
        return "simulation"
    elif variant == "service":
        return "service"
    elif variant == "app":
        return "app"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "Export",
    "encode_export",
    "decode_export",
    "to_json_export",
    "from_json_export",
    "ExportKind",
    "encode_export_kind",
    "decode_export_kind",
    "to_json_export_kind",
    "from_json_export_kind",
]
