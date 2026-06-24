# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_object,
)

import destack._generated.program.native.abi
import destack._generated.program.native.entry
import destack._generated.program.native.image
import destack._generated.program.native.import_
import destack._generated.program.native.map


@dataclass(frozen=True, slots=True)
class Code:
    """Durable native code produced for one program."""

    # the native ABI required by this code
    abi: destack._generated.program.native.abi.Abi
    # the native image kind
    image: destack._generated.program.native.image.Image
    # native imports required by this code
    import_: destack._generated.program.native.import_.ImportTable
    # native code map for safepoints and deoptimization
    map: destack._generated.program.native.map.CodeMap
    # native entries keyed by program ids
    entry: destack._generated.program.native.entry.EntryTable

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
    destack._generated.program.native.abi.encode_abi(writer, value.abi)
    destack._generated.program.native.image.encode_image(writer, value.image)
    destack._generated.program.native.import_.encode_import_table(writer, value.import_)
    destack._generated.program.native.map.encode_code_map(writer, value.map)
    destack._generated.program.native.entry.encode_entry_table(writer, value.entry)


def decode_code(reader: BinaryReader) -> Code:
    """Decode one Code."""
    abi = destack._generated.program.native.abi.decode_abi(reader)
    image = destack._generated.program.native.image.decode_image(reader)
    import_ = destack._generated.program.native.import_.decode_import_table(reader)
    map = destack._generated.program.native.map.decode_code_map(reader)
    entry = destack._generated.program.native.entry.decode_entry_table(reader)

    return Code(
        abi=abi,
        image=image,
        import_=import_,
        map=map,
        entry=entry,
    )


def to_json_code(value: Code) -> Json:
    """Return one JSON value for one Code."""
    return {
        "abi": destack._generated.program.native.abi.to_json_abi(value.abi),
        "image": destack._generated.program.native.image.to_json_image(value.image),
        "import": destack._generated.program.native.import_.to_json_import_table(
            value.import_
        ),
        "map": destack._generated.program.native.map.to_json_code_map(value.map),
        "entry": destack._generated.program.native.entry.to_json_entry_table(
            value.entry
        ),
    }


def from_json_code(value: Json) -> Code:
    """Return one Code from one JSON value."""
    object_ = json_object(value)

    return Code(
        abi=destack._generated.program.native.abi.from_json_abi(
            json_field(object_, "abi")
        ),
        image=destack._generated.program.native.image.from_json_image(
            json_field(object_, "image")
        ),
        import_=destack._generated.program.native.import_.from_json_import_table(
            json_field(object_, "import")
        ),
        map=destack._generated.program.native.map.from_json_code_map(
            json_field(object_, "map")
        ),
        entry=destack._generated.program.native.entry.from_json_entry_table(
            json_field(object_, "entry")
        ),
    )


__all__ = [
    "Code",
    "encode_code",
    "decode_code",
    "to_json_code",
    "from_json_code",
]
