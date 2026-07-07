# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_field,
    json_object,
    json_string,
)

import destack._generated.dir.index.call
import destack._generated.dir.index.decorator
import destack._generated.dir.index.export
import destack._generated.dir.index.extension
import destack._generated.dir.index.heritage
import destack._generated.dir.index.member
import destack._generated.dir.index.reference
import destack._generated.dir.index.specifier
import destack._generated.dir.index.symbol
import destack._generated.source.file.model.module

"""One observable projection of a module index artifact."""
ModuleIndexProjection: typing.TypeAlias = (
    typing.Literal["symbols"]
    | typing.Literal["exports"]
    | typing.Literal["members"]
    | typing.Literal["references"]
    | typing.Literal["calls"]
    | typing.Literal["heritage"]
    | typing.Literal["extensions"]
    | typing.Literal["specifiers"]
    | typing.Literal["decorators"]
)


def encode_module_index_projection(
    writer: BinaryWriter, value: ModuleIndexProjection
) -> None:
    """Encode one ModuleIndexProjection."""
    if value == "symbols":
        writer.write_unsigned(0)
    elif value == "exports":
        writer.write_unsigned(1)
    elif value == "members":
        writer.write_unsigned(2)
    elif value == "references":
        writer.write_unsigned(3)
    elif value == "calls":
        writer.write_unsigned(4)
    elif value == "heritage":
        writer.write_unsigned(5)
    elif value == "extensions":
        writer.write_unsigned(6)
    elif value == "specifiers":
        writer.write_unsigned(7)
    elif value == "decorators":
        writer.write_unsigned(8)
    else:
        raise SerdeError("unknown enum variant")


def decode_module_index_projection(reader: BinaryReader) -> ModuleIndexProjection:
    """Decode one ModuleIndexProjection."""
    variant = reader.read_number()

    if variant == 0:
        return "symbols"
    elif variant == 1:
        return "exports"
    elif variant == 2:
        return "members"
    elif variant == 3:
        return "references"
    elif variant == 4:
        return "calls"
    elif variant == 5:
        return "heritage"
    elif variant == 6:
        return "extensions"
    elif variant == 7:
        return "specifiers"
    elif variant == 8:
        return "decorators"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_module_index_projection(value: ModuleIndexProjection) -> Json:
    """Return one JSON value for one ModuleIndexProjection."""
    return value


def from_json_module_index_projection(value: Json) -> ModuleIndexProjection:
    """Return one ModuleIndexProjection from one JSON value."""
    variant = json_string(value)

    if variant == "symbols":
        return "symbols"
    elif variant == "exports":
        return "exports"
    elif variant == "members":
        return "members"
    elif variant == "references":
        return "references"
    elif variant == "calls":
        return "calls"
    elif variant == "heritage":
        return "heritage"
    elif variant == "extensions":
        return "extensions"
    elif variant == "specifiers":
        return "specifiers"
    elif variant == "decorators":
        return "decorators"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class ModuleIndex:
    """Indexed checked DIR facts for one module profile."""

    # indexed declared symbols
    symbols: destack._generated.dir.index.symbol.SymbolIndex
    # indexed exports
    exports: destack._generated.dir.index.export.ExportIndex
    # indexed checked members
    members: destack._generated.dir.index.member.MemberIndex
    # indexed reference memberships
    references: destack._generated.dir.index.reference.ReferenceIndex
    # indexed call edges
    calls: destack._generated.dir.index.call.CallIndex
    # indexed nominal heritage edges
    heritage: destack._generated.dir.index.heritage.HeritageIndex
    # indexed checked extensions
    extensions: destack._generated.dir.index.extension.ExtensionIndex
    # indexed module specifiers
    specifiers: destack._generated.dir.index.specifier.SpecifierIndex
    # indexed decorators
    decorators: destack._generated.dir.index.decorator.DecoratorIndex

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_module_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ModuleIndex:
        """Decode one ModuleIndex."""
        return decode_module_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_module_index(self)

    @classmethod
    def from_json(cls, value: Json) -> ModuleIndex:
        """Return one ModuleIndex from one JSON value."""
        return from_json_module_index(value)


def encode_module_index(writer: BinaryWriter, value: ModuleIndex) -> None:
    """Encode one ModuleIndex."""
    destack._generated.dir.index.symbol.encode_symbol_index(writer, value.symbols)
    destack._generated.dir.index.export.encode_export_index(writer, value.exports)
    destack._generated.dir.index.member.encode_member_index(writer, value.members)
    destack._generated.dir.index.reference.encode_reference_index(
        writer, value.references
    )
    destack._generated.dir.index.call.encode_call_index(writer, value.calls)
    destack._generated.dir.index.heritage.encode_heritage_index(writer, value.heritage)
    destack._generated.dir.index.extension.encode_extension_index(
        writer, value.extensions
    )
    destack._generated.dir.index.specifier.encode_specifier_index(
        writer, value.specifiers
    )
    destack._generated.dir.index.decorator.encode_decorator_index(
        writer, value.decorators
    )


def decode_module_index(reader: BinaryReader) -> ModuleIndex:
    """Decode one ModuleIndex."""
    symbols = destack._generated.dir.index.symbol.decode_symbol_index(reader)
    exports = destack._generated.dir.index.export.decode_export_index(reader)
    members = destack._generated.dir.index.member.decode_member_index(reader)
    references = destack._generated.dir.index.reference.decode_reference_index(reader)
    calls = destack._generated.dir.index.call.decode_call_index(reader)
    heritage = destack._generated.dir.index.heritage.decode_heritage_index(reader)
    extensions = destack._generated.dir.index.extension.decode_extension_index(reader)
    specifiers = destack._generated.dir.index.specifier.decode_specifier_index(reader)
    decorators = destack._generated.dir.index.decorator.decode_decorator_index(reader)

    return ModuleIndex(
        symbols=symbols,
        exports=exports,
        members=members,
        references=references,
        calls=calls,
        heritage=heritage,
        extensions=extensions,
        specifiers=specifiers,
        decorators=decorators,
    )


def to_json_module_index(value: ModuleIndex) -> Json:
    """Return one JSON value for one ModuleIndex."""
    return {
        "symbols": destack._generated.dir.index.symbol.to_json_symbol_index(
            value.symbols
        ),
        "exports": destack._generated.dir.index.export.to_json_export_index(
            value.exports
        ),
        "members": destack._generated.dir.index.member.to_json_member_index(
            value.members
        ),
        "references": destack._generated.dir.index.reference.to_json_reference_index(
            value.references
        ),
        "calls": destack._generated.dir.index.call.to_json_call_index(value.calls),
        "heritage": destack._generated.dir.index.heritage.to_json_heritage_index(
            value.heritage
        ),
        "extensions": destack._generated.dir.index.extension.to_json_extension_index(
            value.extensions
        ),
        "specifiers": destack._generated.dir.index.specifier.to_json_specifier_index(
            value.specifiers
        ),
        "decorators": destack._generated.dir.index.decorator.to_json_decorator_index(
            value.decorators
        ),
    }


def from_json_module_index(value: Json) -> ModuleIndex:
    """Return one ModuleIndex from one JSON value."""
    object_ = json_object(value)

    return ModuleIndex(
        symbols=destack._generated.dir.index.symbol.from_json_symbol_index(
            json_field(object_, "symbols")
        ),
        exports=destack._generated.dir.index.export.from_json_export_index(
            json_field(object_, "exports")
        ),
        members=destack._generated.dir.index.member.from_json_member_index(
            json_field(object_, "members")
        ),
        references=destack._generated.dir.index.reference.from_json_reference_index(
            json_field(object_, "references")
        ),
        calls=destack._generated.dir.index.call.from_json_call_index(
            json_field(object_, "calls")
        ),
        heritage=destack._generated.dir.index.heritage.from_json_heritage_index(
            json_field(object_, "heritage")
        ),
        extensions=destack._generated.dir.index.extension.from_json_extension_index(
            json_field(object_, "extensions")
        ),
        specifiers=destack._generated.dir.index.specifier.from_json_specifier_index(
            json_field(object_, "specifiers")
        ),
        decorators=destack._generated.dir.index.decorator.from_json_decorator_index(
            json_field(object_, "decorators")
        ),
    )


@dataclass(frozen=True, slots=True)
class ProgramIndex:
    """Indexed checked DIR module set for one program profile."""

    # the indexed modules in stable ordinal order
    modules: Sequence[destack._generated.source.file.model.module.ModuleId]
    # symbol postings
    symbols: destack._generated.dir.index.symbol.SymbolPostings
    # export postings
    exports: destack._generated.dir.index.export.ExportPostings
    # member postings
    members: destack._generated.dir.index.member.MemberPostings
    # reference postings
    references: destack._generated.dir.index.reference.ReferencePostings
    # call postings
    calls: destack._generated.dir.index.call.CallPostings
    # heritage postings
    heritage: destack._generated.dir.index.heritage.HeritagePostings
    # extension postings
    extensions: destack._generated.dir.index.extension.ExtensionPostings
    # module specifier postings
    specifiers: destack._generated.dir.index.specifier.SpecifierPostings
    # decorator postings
    decorators: destack._generated.dir.index.decorator.DecoratorPostings

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ProgramIndex:
        """Decode one ProgramIndex."""
        return decode_program_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_index(self)

    @classmethod
    def from_json(cls, value: Json) -> ProgramIndex:
        """Return one ProgramIndex from one JSON value."""
        return from_json_program_index(value)


def encode_program_index(writer: BinaryWriter, value: ProgramIndex) -> None:
    """Encode one ProgramIndex."""
    writer.write_unsigned(len(value.modules))
    for item_value_modules_0 in value.modules:
        destack._generated.source.file.model.module.encode_module_id(
            writer, item_value_modules_0
        )
    destack._generated.dir.index.symbol.encode_symbol_postings(writer, value.symbols)
    destack._generated.dir.index.export.encode_export_postings(writer, value.exports)
    destack._generated.dir.index.member.encode_member_postings(writer, value.members)
    destack._generated.dir.index.reference.encode_reference_postings(
        writer, value.references
    )
    destack._generated.dir.index.call.encode_call_postings(writer, value.calls)
    destack._generated.dir.index.heritage.encode_heritage_postings(
        writer, value.heritage
    )
    destack._generated.dir.index.extension.encode_extension_postings(
        writer, value.extensions
    )
    destack._generated.dir.index.specifier.encode_specifier_postings(
        writer, value.specifiers
    )
    destack._generated.dir.index.decorator.encode_decorator_postings(
        writer, value.decorators
    )


def decode_program_index(reader: BinaryReader) -> ProgramIndex:
    """Decode one ProgramIndex."""
    modules = [
        destack._generated.source.file.model.module.decode_module_id(reader)
        for _ in range(reader.read_number())
    ]
    symbols = destack._generated.dir.index.symbol.decode_symbol_postings(reader)
    exports = destack._generated.dir.index.export.decode_export_postings(reader)
    members = destack._generated.dir.index.member.decode_member_postings(reader)
    references = destack._generated.dir.index.reference.decode_reference_postings(
        reader
    )
    calls = destack._generated.dir.index.call.decode_call_postings(reader)
    heritage = destack._generated.dir.index.heritage.decode_heritage_postings(reader)
    extensions = destack._generated.dir.index.extension.decode_extension_postings(
        reader
    )
    specifiers = destack._generated.dir.index.specifier.decode_specifier_postings(
        reader
    )
    decorators = destack._generated.dir.index.decorator.decode_decorator_postings(
        reader
    )

    return ProgramIndex(
        modules=modules,
        symbols=symbols,
        exports=exports,
        members=members,
        references=references,
        calls=calls,
        heritage=heritage,
        extensions=extensions,
        specifiers=specifiers,
        decorators=decorators,
    )


def to_json_program_index(value: ProgramIndex) -> Json:
    """Return one JSON value for one ProgramIndex."""
    return {
        "modules": [
            destack._generated.source.file.model.module.to_json_module_id(item_0)
            for item_0 in value.modules
        ],
        "symbols": destack._generated.dir.index.symbol.to_json_symbol_postings(
            value.symbols
        ),
        "exports": destack._generated.dir.index.export.to_json_export_postings(
            value.exports
        ),
        "members": destack._generated.dir.index.member.to_json_member_postings(
            value.members
        ),
        "references": destack._generated.dir.index.reference.to_json_reference_postings(
            value.references
        ),
        "calls": destack._generated.dir.index.call.to_json_call_postings(value.calls),
        "heritage": destack._generated.dir.index.heritage.to_json_heritage_postings(
            value.heritage
        ),
        "extensions": destack._generated.dir.index.extension.to_json_extension_postings(
            value.extensions
        ),
        "specifiers": destack._generated.dir.index.specifier.to_json_specifier_postings(
            value.specifiers
        ),
        "decorators": destack._generated.dir.index.decorator.to_json_decorator_postings(
            value.decorators
        ),
    }


def from_json_program_index(value: Json) -> ProgramIndex:
    """Return one ProgramIndex from one JSON value."""
    object_ = json_object(value)

    return ProgramIndex(
        modules=[
            destack._generated.source.file.model.module.from_json_module_id(item_0)
            for item_0 in json_array(json_field(object_, "modules"))
        ],
        symbols=destack._generated.dir.index.symbol.from_json_symbol_postings(
            json_field(object_, "symbols")
        ),
        exports=destack._generated.dir.index.export.from_json_export_postings(
            json_field(object_, "exports")
        ),
        members=destack._generated.dir.index.member.from_json_member_postings(
            json_field(object_, "members")
        ),
        references=destack._generated.dir.index.reference.from_json_reference_postings(
            json_field(object_, "references")
        ),
        calls=destack._generated.dir.index.call.from_json_call_postings(
            json_field(object_, "calls")
        ),
        heritage=destack._generated.dir.index.heritage.from_json_heritage_postings(
            json_field(object_, "heritage")
        ),
        extensions=destack._generated.dir.index.extension.from_json_extension_postings(
            json_field(object_, "extensions")
        ),
        specifiers=destack._generated.dir.index.specifier.from_json_specifier_postings(
            json_field(object_, "specifiers")
        ),
        decorators=destack._generated.dir.index.decorator.from_json_decorator_postings(
            json_field(object_, "decorators")
        ),
    )


__all__ = [
    "ModuleIndexProjection",
    "encode_module_index_projection",
    "decode_module_index_projection",
    "to_json_module_index_projection",
    "from_json_module_index_projection",
    "ModuleIndex",
    "encode_module_index",
    "decode_module_index",
    "to_json_module_index",
    "from_json_module_index",
    "ProgramIndex",
    "encode_program_index",
    "decode_program_index",
    "to_json_program_index",
    "from_json_program_index",
]
