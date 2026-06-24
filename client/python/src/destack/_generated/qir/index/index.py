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

import destack._generated.qir.index.annotation
import destack._generated.qir.index.call
import destack._generated.qir.index.definition
import destack._generated.qir.index.import_
import destack._generated.qir.index.member
import destack._generated.qir.index.reference
import destack._generated.qir.index.specifier
import destack._generated.qir.index.symbol


@dataclass(frozen=True, slots=True)
class QueryIndex:
    """Durable query index payload for one artifact scope."""

    # searchable symbol declarations
    symbols: destack._generated.qir.index.symbol.SymbolIndex
    # searchable source members
    members: destack._generated.qir.index.member.MemberIndex
    # importable module exports
    imports: destack._generated.qir.index.import_.ImportIndex
    # reference target membership by module
    references: destack._generated.qir.index.reference.ReferenceIndex
    # call graph edges
    calls: destack._generated.qir.index.call.CallIndex
    # definition relations and extension declarations
    definitions: destack._generated.qir.index.definition.DefinitionIndex
    # import specifier rewrite candidates
    specifiers: destack._generated.qir.index.specifier.SpecifierIndex
    # annotation and decorator entries
    annotations: destack._generated.qir.index.annotation.AnnotationIndex

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> QueryIndex:
        """Decode one QueryIndex."""
        return decode_query_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_index(self)

    @classmethod
    def from_json(cls, value: Json) -> QueryIndex:
        """Return one QueryIndex from one JSON value."""
        return from_json_query_index(value)


def encode_query_index(writer: BinaryWriter, value: QueryIndex) -> None:
    """Encode one QueryIndex."""
    destack._generated.qir.index.symbol.encode_symbol_index(writer, value.symbols)
    destack._generated.qir.index.member.encode_member_index(writer, value.members)
    destack._generated.qir.index.import_.encode_import_index(writer, value.imports)
    destack._generated.qir.index.reference.encode_reference_index(
        writer, value.references
    )
    destack._generated.qir.index.call.encode_call_index(writer, value.calls)
    destack._generated.qir.index.definition.encode_definition_index(
        writer, value.definitions
    )
    destack._generated.qir.index.specifier.encode_specifier_index(
        writer, value.specifiers
    )
    destack._generated.qir.index.annotation.encode_annotation_index(
        writer, value.annotations
    )


def decode_query_index(reader: BinaryReader) -> QueryIndex:
    """Decode one QueryIndex."""
    symbols = destack._generated.qir.index.symbol.decode_symbol_index(reader)
    members = destack._generated.qir.index.member.decode_member_index(reader)
    imports = destack._generated.qir.index.import_.decode_import_index(reader)
    references = destack._generated.qir.index.reference.decode_reference_index(reader)
    calls = destack._generated.qir.index.call.decode_call_index(reader)
    definitions = destack._generated.qir.index.definition.decode_definition_index(
        reader
    )
    specifiers = destack._generated.qir.index.specifier.decode_specifier_index(reader)
    annotations = destack._generated.qir.index.annotation.decode_annotation_index(
        reader
    )

    return QueryIndex(
        symbols=symbols,
        members=members,
        imports=imports,
        references=references,
        calls=calls,
        definitions=definitions,
        specifiers=specifiers,
        annotations=annotations,
    )


def to_json_query_index(value: QueryIndex) -> Json:
    """Return one JSON value for one QueryIndex."""
    return {
        "symbols": destack._generated.qir.index.symbol.to_json_symbol_index(
            value.symbols
        ),
        "members": destack._generated.qir.index.member.to_json_member_index(
            value.members
        ),
        "imports": destack._generated.qir.index.import_.to_json_import_index(
            value.imports
        ),
        "references": destack._generated.qir.index.reference.to_json_reference_index(
            value.references
        ),
        "calls": destack._generated.qir.index.call.to_json_call_index(value.calls),
        "definitions": destack._generated.qir.index.definition.to_json_definition_index(
            value.definitions
        ),
        "specifiers": destack._generated.qir.index.specifier.to_json_specifier_index(
            value.specifiers
        ),
        "annotations": destack._generated.qir.index.annotation.to_json_annotation_index(
            value.annotations
        ),
    }


def from_json_query_index(value: Json) -> QueryIndex:
    """Return one QueryIndex from one JSON value."""
    object_ = json_object(value)

    return QueryIndex(
        symbols=destack._generated.qir.index.symbol.from_json_symbol_index(
            json_field(object_, "symbols")
        ),
        members=destack._generated.qir.index.member.from_json_member_index(
            json_field(object_, "members")
        ),
        imports=destack._generated.qir.index.import_.from_json_import_index(
            json_field(object_, "imports")
        ),
        references=destack._generated.qir.index.reference.from_json_reference_index(
            json_field(object_, "references")
        ),
        calls=destack._generated.qir.index.call.from_json_call_index(
            json_field(object_, "calls")
        ),
        definitions=destack._generated.qir.index.definition.from_json_definition_index(
            json_field(object_, "definitions")
        ),
        specifiers=destack._generated.qir.index.specifier.from_json_specifier_index(
            json_field(object_, "specifiers")
        ),
        annotations=destack._generated.qir.index.annotation.from_json_annotation_index(
            json_field(object_, "annotations")
        ),
    )


__all__ = [
    "QueryIndex",
    "encode_query_index",
    "decode_query_index",
    "to_json_query_index",
    "from_json_query_index",
]
