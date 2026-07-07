# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_object,
)

import destack._generated.core.section


@dataclass(frozen=True, slots=True)
class ProgramInfo:
    """Reflected view of one program."""

    # reflected modules in this program
    modules: destack._generated.core.section.SectionSlice
    # reflected types keyed by program type id
    types: destack._generated.core.section.SectionSlice
    # reflected functions keyed by program function id
    functions: destack._generated.core.section.SectionSlice
    # reflected runtime bindings
    bindings: destack._generated.core.section.SectionSlice
    # reflected frame layouts keyed by frame layout id
    frames: destack._generated.core.section.SectionSlice
    # reflected globals keyed by program global id
    globals: destack._generated.core.section.SectionSlice
    # reflected entrypoints
    entries: destack._generated.core.section.SectionSlice
    # flattened child type operands used by variable-arity type payloads
    type_operands: destack._generated.core.section.SectionSlice
    # flattened object-like fields
    fields: destack._generated.core.section.SectionSlice
    # flattened tuple elements
    tuple_elements: destack._generated.core.section.SectionSlice
    # flattened reflected members
    members: destack._generated.core.section.SectionSlice
    # flattened reflected index signatures
    index_signatures: destack._generated.core.section.SectionSlice
    # flattened reflected function parameters
    function_parameters: destack._generated.core.section.SectionSlice
    # flattened reflected binding strings
    binding_strings: destack._generated.core.section.SectionSlice
    # flattened reflected variant cases
    variant_cases: destack._generated.core.section.SectionSlice
    # flattened reflected frame slots
    frame_slots: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ProgramInfo:
        """Decode one ProgramInfo."""
        return decode_program_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_info(self)

    @classmethod
    def from_json(cls, value: Json) -> ProgramInfo:
        """Return one ProgramInfo from one JSON value."""
        return from_json_program_info(value)


def encode_program_info(writer: BinaryWriter, value: ProgramInfo) -> None:
    """Encode one ProgramInfo."""
    destack._generated.core.section.encode_section_slice(writer, value.modules)
    destack._generated.core.section.encode_section_slice(writer, value.types)
    destack._generated.core.section.encode_section_slice(writer, value.functions)
    destack._generated.core.section.encode_section_slice(writer, value.bindings)
    destack._generated.core.section.encode_section_slice(writer, value.frames)
    destack._generated.core.section.encode_section_slice(writer, value.globals)
    destack._generated.core.section.encode_section_slice(writer, value.entries)
    destack._generated.core.section.encode_section_slice(writer, value.type_operands)
    destack._generated.core.section.encode_section_slice(writer, value.fields)
    destack._generated.core.section.encode_section_slice(writer, value.tuple_elements)
    destack._generated.core.section.encode_section_slice(writer, value.members)
    destack._generated.core.section.encode_section_slice(writer, value.index_signatures)
    destack._generated.core.section.encode_section_slice(
        writer, value.function_parameters
    )
    destack._generated.core.section.encode_section_slice(writer, value.binding_strings)
    destack._generated.core.section.encode_section_slice(writer, value.variant_cases)
    destack._generated.core.section.encode_section_slice(writer, value.frame_slots)


def decode_program_info(reader: BinaryReader) -> ProgramInfo:
    """Decode one ProgramInfo."""
    modules = destack._generated.core.section.decode_section_slice(reader)
    types = destack._generated.core.section.decode_section_slice(reader)
    functions = destack._generated.core.section.decode_section_slice(reader)
    bindings = destack._generated.core.section.decode_section_slice(reader)
    frames = destack._generated.core.section.decode_section_slice(reader)
    globals = destack._generated.core.section.decode_section_slice(reader)
    entries = destack._generated.core.section.decode_section_slice(reader)
    type_operands = destack._generated.core.section.decode_section_slice(reader)
    fields = destack._generated.core.section.decode_section_slice(reader)
    tuple_elements = destack._generated.core.section.decode_section_slice(reader)
    members = destack._generated.core.section.decode_section_slice(reader)
    index_signatures = destack._generated.core.section.decode_section_slice(reader)
    function_parameters = destack._generated.core.section.decode_section_slice(reader)
    binding_strings = destack._generated.core.section.decode_section_slice(reader)
    variant_cases = destack._generated.core.section.decode_section_slice(reader)
    frame_slots = destack._generated.core.section.decode_section_slice(reader)

    return ProgramInfo(
        modules=modules,
        types=types,
        functions=functions,
        bindings=bindings,
        frames=frames,
        globals=globals,
        entries=entries,
        type_operands=type_operands,
        fields=fields,
        tuple_elements=tuple_elements,
        members=members,
        index_signatures=index_signatures,
        function_parameters=function_parameters,
        binding_strings=binding_strings,
        variant_cases=variant_cases,
        frame_slots=frame_slots,
    )


def to_json_program_info(value: ProgramInfo) -> Json:
    """Return one JSON value for one ProgramInfo."""
    return {
        "modules": destack._generated.core.section.to_json_section_slice(value.modules),
        "types": destack._generated.core.section.to_json_section_slice(value.types),
        "functions": destack._generated.core.section.to_json_section_slice(
            value.functions
        ),
        "bindings": destack._generated.core.section.to_json_section_slice(
            value.bindings
        ),
        "frames": destack._generated.core.section.to_json_section_slice(value.frames),
        "globals": destack._generated.core.section.to_json_section_slice(value.globals),
        "entries": destack._generated.core.section.to_json_section_slice(value.entries),
        "typeOperands": destack._generated.core.section.to_json_section_slice(
            value.type_operands
        ),
        "fields": destack._generated.core.section.to_json_section_slice(value.fields),
        "tupleElements": destack._generated.core.section.to_json_section_slice(
            value.tuple_elements
        ),
        "members": destack._generated.core.section.to_json_section_slice(value.members),
        "indexSignatures": destack._generated.core.section.to_json_section_slice(
            value.index_signatures
        ),
        "functionParameters": destack._generated.core.section.to_json_section_slice(
            value.function_parameters
        ),
        "bindingStrings": destack._generated.core.section.to_json_section_slice(
            value.binding_strings
        ),
        "variantCases": destack._generated.core.section.to_json_section_slice(
            value.variant_cases
        ),
        "frameSlots": destack._generated.core.section.to_json_section_slice(
            value.frame_slots
        ),
    }


def from_json_program_info(value: Json) -> ProgramInfo:
    """Return one ProgramInfo from one JSON value."""
    object_ = json_object(value)

    return ProgramInfo(
        modules=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "modules")
        ),
        types=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "types")
        ),
        functions=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "functions")
        ),
        bindings=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "bindings")
        ),
        frames=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "frames")
        ),
        globals=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "globals")
        ),
        entries=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "entries")
        ),
        type_operands=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "typeOperands")
        ),
        fields=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "fields")
        ),
        tuple_elements=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tupleElements")
        ),
        members=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "members")
        ),
        index_signatures=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "indexSignatures")
        ),
        function_parameters=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "functionParameters")
        ),
        binding_strings=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "bindingStrings")
        ),
        variant_cases=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "variantCases")
        ),
        frame_slots=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "frameSlots")
        ),
    )


__all__ = [
    "ProgramInfo",
    "encode_program_info",
    "decode_program_info",
    "to_json_program_info",
    "from_json_program_info",
]
