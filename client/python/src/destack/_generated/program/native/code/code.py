# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_int,
    json_object,
)

import destack._generated.core.string
import destack._generated.mir.table.target
import destack._generated.program.native.code.entry
import destack._generated.program.native.code.image
import destack._generated.program.native.code.import_
import destack._generated.program.native.code.map


@dataclass(frozen=True, slots=True)
class Code:
    """Durable native code produced for one program."""

    # destack native ABI version required by this code
    abi_version: int
    # target triple or equivalent target identity
    target: destack._generated.core.string.StringId
    # target ABI layout expected by this code
    target_layout: destack._generated.mir.table.target.TargetLayout
    # the native image
    image: destack._generated.program.native.code.image.Image
    # native imports required by this code
    imports: destack._generated.program.native.code.import_.ImportTable
    # native code map for safepoints and deoptimization
    map: destack._generated.program.native.code.map.CodeMap
    # native entries keyed by program ids
    entries: destack._generated.program.native.code.entry.EntryTable

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_code(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Code:
        """Decode one Code."""
        return decode_code(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_code(self)

    @classmethod
    def from_json(cls, value: Json) -> Code:
        """Return one Code from one JSON value."""
        return from_json_code(value)


def encode_code(writer: BinaryWriter, value: Code) -> None:
    """Encode one Code."""
    writer.write_unsigned(value.abi_version)
    destack._generated.core.string.encode_string_id(writer, value.target)
    destack._generated.mir.table.target.encode_target_layout(
        writer, value.target_layout
    )
    destack._generated.program.native.code.image.encode_image(writer, value.image)
    destack._generated.program.native.code.import_.encode_import_table(
        writer, value.imports
    )
    destack._generated.program.native.code.map.encode_code_map(writer, value.map)
    destack._generated.program.native.code.entry.encode_entry_table(
        writer, value.entries
    )


def decode_code(reader: BinaryReader) -> Code:
    """Decode one Code."""
    abi_version = reader.read_number()
    target = destack._generated.core.string.decode_string_id(reader)
    target_layout = destack._generated.mir.table.target.decode_target_layout(reader)
    image = destack._generated.program.native.code.image.decode_image(reader)
    imports = destack._generated.program.native.code.import_.decode_import_table(reader)
    map = destack._generated.program.native.code.map.decode_code_map(reader)
    entries = destack._generated.program.native.code.entry.decode_entry_table(reader)

    return Code(
        abi_version=abi_version,
        target=target,
        target_layout=target_layout,
        image=image,
        imports=imports,
        map=map,
        entries=entries,
    )


def to_json_code(value: Code) -> Json:
    """Return one JSON value for one Code."""
    return {
        "abiVersion": value.abi_version,
        "target": destack._generated.core.string.to_json_string_id(value.target),
        "targetLayout": destack._generated.mir.table.target.to_json_target_layout(
            value.target_layout
        ),
        "image": destack._generated.program.native.code.image.to_json_image(
            value.image
        ),
        "imports": destack._generated.program.native.code.import_.to_json_import_table(
            value.imports
        ),
        "map": destack._generated.program.native.code.map.to_json_code_map(value.map),
        "entries": destack._generated.program.native.code.entry.to_json_entry_table(
            value.entries
        ),
    }


def from_json_code(value: Json) -> Code:
    """Return one Code from one JSON value."""
    object_ = json_object(value)

    return Code(
        abi_version=json_int(json_field(object_, "abiVersion")),
        target=destack._generated.core.string.from_json_string_id(
            json_field(object_, "target")
        ),
        target_layout=destack._generated.mir.table.target.from_json_target_layout(
            json_field(object_, "targetLayout")
        ),
        image=destack._generated.program.native.code.image.from_json_image(
            json_field(object_, "image")
        ),
        imports=destack._generated.program.native.code.import_.from_json_import_table(
            json_field(object_, "imports")
        ),
        map=destack._generated.program.native.code.map.from_json_code_map(
            json_field(object_, "map")
        ),
        entries=destack._generated.program.native.code.entry.from_json_entry_table(
            json_field(object_, "entries")
        ),
    )


__all__ = [
    "Code",
    "encode_code",
    "decode_code",
    "to_json_code",
    "from_json_code",
]
