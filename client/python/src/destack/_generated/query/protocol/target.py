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
    json_optional,
    json_string,
)

import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.source.file.model.file
import destack._generated.source.file.model.module
import destack._generated.source.file.model.profile
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class Module:
    """One module in one query profile."""

    # the queried module
    module_id: destack._generated.source.file.model.module.ModuleId
    # the queried profile
    profile_id: destack._generated.source.file.model.profile.ProfileId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_module(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Module:
        """Decode one Module."""
        return decode_module(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_module(self)

    @classmethod
    def from_json(cls, value: Json) -> Module:
        """Return one Module from one JSON value."""
        return from_json_module(value)


def encode_module(writer: BinaryWriter, value: Module) -> None:
    """Encode one Module."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    destack._generated.source.file.model.profile.encode_profile_id(
        writer, value.profile_id
    )


def decode_module(reader: BinaryReader) -> Module:
    """Decode one Module."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    profile_id = destack._generated.source.file.model.profile.decode_profile_id(reader)

    return Module(
        module_id=module_id,
        profile_id=profile_id,
    )


def to_json_module(value: Module) -> Json:
    """Return one JSON value for one Module."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "profileId": destack._generated.source.file.model.profile.to_json_profile_id(
            value.profile_id
        ),
    }


def from_json_module(value: Json) -> Module:
    """Return one Module from one JSON value."""
    object_ = json_object(value)

    return Module(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        profile_id=destack._generated.source.file.model.profile.from_json_profile_id(
            json_field(object_, "profileId")
        ),
    )


@dataclass(frozen=True, slots=True)
class Position:
    """One byte position in a module source file."""

    # the queried module profile
    module: Module
    # the source file
    file_id: destack._generated.source.file.model.file.FileId
    # the byte offset in the source file
    offset: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_position(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Position:
        """Decode one Position."""
        return decode_position(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_position(self)

    @classmethod
    def from_json(cls, value: Json) -> Position:
        """Return one Position from one JSON value."""
        return from_json_position(value)


def encode_position(writer: BinaryWriter, value: Position) -> None:
    """Encode one Position."""
    encode_module(writer, value.module)
    destack._generated.source.file.model.file.encode_file_id(writer, value.file_id)
    writer.write_unsigned(value.offset)


def decode_position(reader: BinaryReader) -> Position:
    """Decode one Position."""
    module = decode_module(reader)
    file_id = destack._generated.source.file.model.file.decode_file_id(reader)
    offset = reader.read_number()

    return Position(
        module=module,
        file_id=file_id,
        offset=offset,
    )


def to_json_position(value: Position) -> Json:
    """Return one JSON value for one Position."""
    return {
        "module": to_json_module(value.module),
        "fileId": destack._generated.source.file.model.file.to_json_file_id(
            value.file_id
        ),
        "offset": value.offset,
    }


def from_json_position(value: Json) -> Position:
    """Return one Position from one JSON value."""
    object_ = json_object(value)

    return Position(
        module=from_json_module(json_field(object_, "module")),
        file_id=destack._generated.source.file.model.file.from_json_file_id(
            json_field(object_, "fileId")
        ),
        offset=json_int(json_field(object_, "offset")),
    )


@dataclass(frozen=True, slots=True)
class Range:
    """One source range in a module."""

    # the queried module profile
    module: Module
    # the source range
    span: destack._generated.source.file.model.span.Span

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_range(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Range:
        """Decode one Range."""
        return decode_range(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_range(self)

    @classmethod
    def from_json(cls, value: Json) -> Range:
        """Return one Range from one JSON value."""
        return from_json_range(value)


def encode_range(writer: BinaryWriter, value: Range) -> None:
    """Encode one Range."""
    encode_module(writer, value.module)
    destack._generated.source.file.model.span.encode_span(writer, value.span)


def decode_range(reader: BinaryReader) -> Range:
    """Decode one Range."""
    module = decode_module(reader)
    span = destack._generated.source.file.model.span.decode_span(reader)

    return Range(
        module=module,
        span=span,
    )


def to_json_range(value: Range) -> Json:
    """Return one JSON value for one Range."""
    return {
        "module": to_json_module(value.module),
        "span": destack._generated.source.file.model.span.to_json_span(value.span),
    }


def from_json_range(value: Json) -> Range:
    """Return one Range from one JSON value."""
    object_ = json_object(value)

    return Range(
        module=from_json_module(json_field(object_, "module")),
        span=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "span")
        ),
    )


@dataclass(frozen=True, slots=True)
class Target:
    """One source-backed target."""

    # the target module profile
    module: Module
    # the full source range
    span: destack._generated.source.file.model.span.Span
    # the primary selection range
    selection_span: destack._generated.source.file.model.span.Span | None
    # the target symbol when known
    symbol_id: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the target node when known
    node_id: destack._generated.dir.tree.node.GlobalNodeIdAny | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Target:
        """Decode one Target."""
        return decode_target(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target(self)

    @classmethod
    def from_json(cls, value: Json) -> Target:
        """Return one Target from one JSON value."""
        return from_json_target(value)


def encode_target(writer: BinaryWriter, value: Target) -> None:
    """Encode one Target."""
    encode_module(writer, value.module)
    destack._generated.source.file.model.span.encode_span(writer, value.span)
    if value.selection_span is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.span.encode_span(
            writer, value.selection_span
        )
    if value.symbol_id is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.symbol_id
        )
    if value.node_id is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, value.node_id
        )


def decode_target(reader: BinaryReader) -> Target:
    """Decode one Target."""
    module = decode_module(reader)
    span = destack._generated.source.file.model.span.decode_span(reader)
    selection_span = reader.read_option(
        lambda: destack._generated.source.file.model.span.decode_span(reader)
    )
    symbol_id = reader.read_option(
        lambda: destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    )
    node_id = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    )

    return Target(
        module=module,
        span=span,
        selection_span=selection_span,
        symbol_id=symbol_id,
        node_id=node_id,
    )


def to_json_target(value: Target) -> Json:
    """Return one JSON value for one Target."""
    return {
        "module": to_json_module(value.module),
        "span": destack._generated.source.file.model.span.to_json_span(value.span),
        **(
            {}
            if value.selection_span is None
            else {
                "selectionSpan": destack._generated.source.file.model.span.to_json_span(
                    value.selection_span
                )
            }
        ),
        **(
            {}
            if value.symbol_id is None
            else {
                "symbolId": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                    value.symbol_id
                )
            }
        ),
        **(
            {}
            if value.node_id is None
            else {
                "nodeId": destack._generated.dir.tree.node.to_json_global_node_id_any(
                    value.node_id
                )
            }
        ),
    }


def from_json_target(value: Json) -> Target:
    """Return one Target from one JSON value."""
    object_ = json_object(value)

    return Target(
        module=from_json_module(json_field(object_, "module")),
        span=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "span")
        ),
        selection_span=json_optional(
            object_,
            "selectionSpan",
            lambda value: destack._generated.source.file.model.span.from_json_span(
                value
            ),
        ),
        symbol_id=json_optional(
            object_,
            "symbolId",
            lambda value: (
                destack._generated.dir.symbol.symbol.from_json_global_symbol_id(value)
            ),
        ),
        node_id=json_optional(
            object_,
            "nodeId",
            lambda value: destack._generated.dir.tree.node.from_json_global_node_id_any(
                value
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class Text:
    """Text in display formats understood by clients."""

    # plain text
    plain: str | None
    # markdown text
    markdown: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_text(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Text:
        """Decode one Text."""
        return decode_text(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_text(self)

    @classmethod
    def from_json(cls, value: Json) -> Text:
        """Return one Text from one JSON value."""
        return from_json_text(value)


def encode_text(writer: BinaryWriter, value: Text) -> None:
    """Encode one Text."""
    if value.plain is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.plain)
    if value.markdown is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.markdown)


def decode_text(reader: BinaryReader) -> Text:
    """Decode one Text."""
    plain = reader.read_option(lambda: reader.read_string())
    markdown = reader.read_option(lambda: reader.read_string())

    return Text(
        plain=plain,
        markdown=markdown,
    )


def to_json_text(value: Text) -> Json:
    """Return one JSON value for one Text."""
    return {
        **({} if value.plain is None else {"plain": value.plain}),
        **({} if value.markdown is None else {"markdown": value.markdown}),
    }


def from_json_text(value: Json) -> Text:
    """Return one Text from one JSON value."""
    object_ = json_object(value)

    return Text(
        plain=json_optional(object_, "plain", lambda value: json_string(value)),
        markdown=json_optional(object_, "markdown", lambda value: json_string(value)),
    )


__all__ = [
    "Module",
    "encode_module",
    "decode_module",
    "to_json_module",
    "from_json_module",
    "Position",
    "encode_position",
    "decode_position",
    "to_json_position",
    "from_json_position",
    "Range",
    "encode_range",
    "decode_range",
    "to_json_range",
    "from_json_range",
    "Target",
    "encode_target",
    "decode_target",
    "to_json_target",
    "from_json_target",
    "Text",
    "encode_text",
    "decode_text",
    "to_json_text",
    "from_json_text",
]
