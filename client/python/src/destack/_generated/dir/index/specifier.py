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
    json_array_length,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.dir.index.postings
import destack._generated.dir.tree.node
import destack._generated.source.file.model.file
import destack._generated.source.file.model.module
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class SpecifierIndex:
    """Module specifier rewrite index."""

    # the resolved specifiers ordered by target path
    by_target_path: Sequence[tuple[str, SpecifierEntry]]
    # the specifiers without a resolved target path
    unresolved: Sequence[SpecifierEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_specifier_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SpecifierIndex:
        """Decode one SpecifierIndex."""
        return decode_specifier_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_specifier_index(self)

    @classmethod
    def from_json(cls, value: Json) -> SpecifierIndex:
        """Return one SpecifierIndex from one JSON value."""
        return from_json_specifier_index(value)


def encode_specifier_index(writer: BinaryWriter, value: SpecifierIndex) -> None:
    """Encode one SpecifierIndex."""
    writer.write_unsigned(len(value.by_target_path))
    for item_value_by_target_path_0 in value.by_target_path:
        writer.write_string(item_value_by_target_path_0[0])
        encode_specifier_entry(writer, item_value_by_target_path_0[1])
    writer.write_unsigned(len(value.unresolved))
    for item_value_unresolved_0 in value.unresolved:
        encode_specifier_entry(writer, item_value_unresolved_0)


def decode_specifier_index(reader: BinaryReader) -> SpecifierIndex:
    """Decode one SpecifierIndex."""
    by_target_path = [
        (
            reader.read_string(),
            decode_specifier_entry(reader),
        )
        for _ in range(reader.read_number())
    ]
    unresolved = [decode_specifier_entry(reader) for _ in range(reader.read_number())]

    return SpecifierIndex(
        by_target_path=by_target_path,
        unresolved=unresolved,
    )


def to_json_specifier_index(value: SpecifierIndex) -> Json:
    """Return one JSON value for one SpecifierIndex."""
    return {
        "byTargetPath": [
            [item_0[0], to_json_specifier_entry(item_0[1])]
            for item_0 in value.by_target_path
        ],
        "unresolved": [to_json_specifier_entry(item_0) for item_0 in value.unresolved],
    }


def from_json_specifier_index(value: Json) -> SpecifierIndex:
    """Return one SpecifierIndex from one JSON value."""
    object_ = json_object(value)

    return SpecifierIndex(
        by_target_path=[
            (
                lambda items: (
                    json_string(items[0]),
                    from_json_specifier_entry(items[1]),
                )
            )(json_array_length(item_0, 2))
            for item_0 in json_array(json_field(object_, "byTargetPath"))
        ],
        unresolved=[
            from_json_specifier_entry(item_0)
            for item_0 in json_array(json_field(object_, "unresolved"))
        ],
    )


@dataclass(frozen=True, slots=True)
class SpecifierEntry:
    """One indexed module specifier."""

    # the source file
    file: destack._generated.source.file.model.file.FileId
    # the source node that owns the specifier
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the source range
    span: destack._generated.source.file.model.span.Span
    # the module specifier kind
    kind: SpecifierKind
    # the specifier text
    text: str
    # the semantic target module when resolved
    target_module: destack._generated.source.file.model.module.ModuleId | None
    # the semantic target path when known
    target_path: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_specifier_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SpecifierEntry:
        """Decode one SpecifierEntry."""
        return decode_specifier_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_specifier_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> SpecifierEntry:
        """Return one SpecifierEntry from one JSON value."""
        return from_json_specifier_entry(value)


def encode_specifier_entry(writer: BinaryWriter, value: SpecifierEntry) -> None:
    """Encode one SpecifierEntry."""
    destack._generated.source.file.model.file.encode_file_id(writer, value.file)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.source.file.model.span.encode_span(writer, value.span)
    encode_specifier_kind(writer, value.kind)
    writer.write_string(value.text)
    if value.target_module is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.target_module
        )
    if value.target_path is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.target_path)


def decode_specifier_entry(reader: BinaryReader) -> SpecifierEntry:
    """Decode one SpecifierEntry."""
    file = destack._generated.source.file.model.file.decode_file_id(reader)
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    span = destack._generated.source.file.model.span.decode_span(reader)
    kind = decode_specifier_kind(reader)
    text = reader.read_string()
    target_module = reader.read_option(
        lambda: destack._generated.source.file.model.module.decode_module_id(reader)
    )
    target_path = reader.read_option(lambda: reader.read_string())

    return SpecifierEntry(
        file=file,
        source=source,
        span=span,
        kind=kind,
        text=text,
        target_module=target_module,
        target_path=target_path,
    )


def to_json_specifier_entry(value: SpecifierEntry) -> Json:
    """Return one JSON value for one SpecifierEntry."""
    return {
        "file": destack._generated.source.file.model.file.to_json_file_id(value.file),
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "span": destack._generated.source.file.model.span.to_json_span(value.span),
        "kind": to_json_specifier_kind(value.kind),
        "text": value.text,
        **(
            {}
            if value.target_module is None
            else {
                "targetModule": destack._generated.source.file.model.module.to_json_module_id(
                    value.target_module
                )
            }
        ),
        **({} if value.target_path is None else {"targetPath": value.target_path}),
    }


def from_json_specifier_entry(value: Json) -> SpecifierEntry:
    """Return one SpecifierEntry from one JSON value."""
    object_ = json_object(value)

    return SpecifierEntry(
        file=destack._generated.source.file.model.file.from_json_file_id(
            json_field(object_, "file")
        ),
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        span=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "span")
        ),
        kind=from_json_specifier_kind(json_field(object_, "kind")),
        text=json_string(json_field(object_, "text")),
        target_module=json_optional(
            object_,
            "targetModule",
            lambda value: (
                destack._generated.source.file.model.module.from_json_module_id(value)
            ),
        ),
        target_path=json_optional(
            object_, "targetPath", lambda value: json_string(value)
        ),
    )


"""Kind of module specifier."""
SpecifierKind: typing.TypeAlias = typing.Literal["import"] | typing.Literal["export"]


def encode_specifier_kind(writer: BinaryWriter, value: SpecifierKind) -> None:
    """Encode one SpecifierKind."""
    if value == "import":
        writer.write_unsigned(0)
    elif value == "export":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_specifier_kind(reader: BinaryReader) -> SpecifierKind:
    """Decode one SpecifierKind."""
    variant = reader.read_number()

    if variant == 0:
        return "import"
    elif variant == 1:
        return "export"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_specifier_kind(value: SpecifierKind) -> Json:
    """Return one JSON value for one SpecifierKind."""
    return value


def from_json_specifier_kind(value: Json) -> SpecifierKind:
    """Return one SpecifierKind from one JSON value."""
    variant = json_string(value)

    if variant == "import":
        return "import"
    elif variant == "export":
        return "export"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class SpecifierPostings:
    """Module specifier postings by resolved target path."""

    # resolved target path postings
    paths: destack._generated.dir.index.postings.Postings
    # modules that contain unresolved specifiers
    unresolved: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_specifier_postings(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SpecifierPostings:
        """Decode one SpecifierPostings."""
        return decode_specifier_postings(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_specifier_postings(self)

    @classmethod
    def from_json(cls, value: Json) -> SpecifierPostings:
        """Return one SpecifierPostings from one JSON value."""
        return from_json_specifier_postings(value)


def encode_specifier_postings(writer: BinaryWriter, value: SpecifierPostings) -> None:
    """Encode one SpecifierPostings."""
    destack._generated.dir.index.postings.encode_postings(writer, value.paths)
    writer.write_unsigned(len(value.unresolved))
    for item_value_unresolved_0 in value.unresolved:
        writer.write_unsigned(item_value_unresolved_0)


def decode_specifier_postings(reader: BinaryReader) -> SpecifierPostings:
    """Decode one SpecifierPostings."""
    paths = destack._generated.dir.index.postings.decode_postings(reader)
    unresolved = [reader.read_number() for _ in range(reader.read_number())]

    return SpecifierPostings(
        paths=paths,
        unresolved=unresolved,
    )


def to_json_specifier_postings(value: SpecifierPostings) -> Json:
    """Return one JSON value for one SpecifierPostings."""
    return {
        "paths": destack._generated.dir.index.postings.to_json_postings(value.paths),
        "unresolved": [item_0 for item_0 in value.unresolved],
    }


def from_json_specifier_postings(value: Json) -> SpecifierPostings:
    """Return one SpecifierPostings from one JSON value."""
    object_ = json_object(value)

    return SpecifierPostings(
        paths=destack._generated.dir.index.postings.from_json_postings(
            json_field(object_, "paths")
        ),
        unresolved=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "unresolved"))
        ],
    )


__all__ = [
    "SpecifierIndex",
    "encode_specifier_index",
    "decode_specifier_index",
    "to_json_specifier_index",
    "from_json_specifier_index",
    "SpecifierEntry",
    "encode_specifier_entry",
    "decode_specifier_entry",
    "to_json_specifier_entry",
    "from_json_specifier_entry",
    "SpecifierKind",
    "encode_specifier_kind",
    "decode_specifier_kind",
    "to_json_specifier_kind",
    "from_json_specifier_kind",
    "SpecifierPostings",
    "encode_specifier_postings",
    "decode_specifier_postings",
    "to_json_specifier_postings",
    "from_json_specifier_postings",
]
