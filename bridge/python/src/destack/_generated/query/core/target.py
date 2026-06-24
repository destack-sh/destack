# generated bridge target, do not edit

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
class QueryModule:
    """One module in one query profile."""

    # the queried module
    module_id: destack._generated.source.file.model.module.ModuleId
    # the queried profile
    profile_id: destack._generated.source.file.model.profile.ProfileId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_module(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> QueryModule:
        """Decode one QueryModule."""
        return decode_query_module(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_module(self)

    @classmethod
    def from_json(cls, value: Json) -> QueryModule:
        """Return one QueryModule from one JSON value."""
        return from_json_query_module(value)


def encode_query_module(writer: BinaryWriter, value: QueryModule) -> None:
    """Encode one QueryModule."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    destack._generated.source.file.model.profile.encode_profile_id(
        writer, value.profile_id
    )


def decode_query_module(reader: BinaryReader) -> QueryModule:
    """Decode one QueryModule."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    profile_id = destack._generated.source.file.model.profile.decode_profile_id(reader)

    return QueryModule(
        module_id=module_id,
        profile_id=profile_id,
    )


def to_json_query_module(value: QueryModule) -> Json:
    """Return one JSON value for one QueryModule."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "profileId": destack._generated.source.file.model.profile.to_json_profile_id(
            value.profile_id
        ),
    }


def from_json_query_module(value: Json) -> QueryModule:
    """Return one QueryModule from one JSON value."""
    object_ = json_object(value)

    return QueryModule(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        profile_id=destack._generated.source.file.model.profile.from_json_profile_id(
            json_field(object_, "profileId")
        ),
    )


@dataclass(frozen=True, slots=True)
class QueryPosition:
    """One byte position in a module source file."""

    # the queried module profile
    module: QueryModule
    # the source file
    file_id: destack._generated.source.file.model.file.FileId
    # the byte offset in the source file
    offset: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_position(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> QueryPosition:
        """Decode one QueryPosition."""
        return decode_query_position(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_position(self)

    @classmethod
    def from_json(cls, value: Json) -> QueryPosition:
        """Return one QueryPosition from one JSON value."""
        return from_json_query_position(value)


def encode_query_position(writer: BinaryWriter, value: QueryPosition) -> None:
    """Encode one QueryPosition."""
    encode_query_module(writer, value.module)
    destack._generated.source.file.model.file.encode_file_id(writer, value.file_id)
    writer.write_unsigned(value.offset)


def decode_query_position(reader: BinaryReader) -> QueryPosition:
    """Decode one QueryPosition."""
    module = decode_query_module(reader)
    file_id = destack._generated.source.file.model.file.decode_file_id(reader)
    offset = reader.read_number()

    return QueryPosition(
        module=module,
        file_id=file_id,
        offset=offset,
    )


def to_json_query_position(value: QueryPosition) -> Json:
    """Return one JSON value for one QueryPosition."""
    return {
        "module": to_json_query_module(value.module),
        "fileId": destack._generated.source.file.model.file.to_json_file_id(
            value.file_id
        ),
        "offset": value.offset,
    }


def from_json_query_position(value: Json) -> QueryPosition:
    """Return one QueryPosition from one JSON value."""
    object_ = json_object(value)

    return QueryPosition(
        module=from_json_query_module(json_field(object_, "module")),
        file_id=destack._generated.source.file.model.file.from_json_file_id(
            json_field(object_, "fileId")
        ),
        offset=json_int(json_field(object_, "offset")),
    )


@dataclass(frozen=True, slots=True)
class QueryRange:
    """One source range in a module."""

    # the queried module profile
    module: QueryModule
    # the source range
    span: destack._generated.source.file.model.span.Span

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_range(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> QueryRange:
        """Decode one QueryRange."""
        return decode_query_range(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_range(self)

    @classmethod
    def from_json(cls, value: Json) -> QueryRange:
        """Return one QueryRange from one JSON value."""
        return from_json_query_range(value)


def encode_query_range(writer: BinaryWriter, value: QueryRange) -> None:
    """Encode one QueryRange."""
    encode_query_module(writer, value.module)
    destack._generated.source.file.model.span.encode_span(writer, value.span)


def decode_query_range(reader: BinaryReader) -> QueryRange:
    """Decode one QueryRange."""
    module = decode_query_module(reader)
    span = destack._generated.source.file.model.span.decode_span(reader)

    return QueryRange(
        module=module,
        span=span,
    )


def to_json_query_range(value: QueryRange) -> Json:
    """Return one JSON value for one QueryRange."""
    return {
        "module": to_json_query_module(value.module),
        "span": destack._generated.source.file.model.span.to_json_span(value.span),
    }


def from_json_query_range(value: Json) -> QueryRange:
    """Return one QueryRange from one JSON value."""
    object_ = json_object(value)

    return QueryRange(
        module=from_json_query_module(json_field(object_, "module")),
        span=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "span")
        ),
    )


@dataclass(frozen=True, slots=True)
class QueryTarget:
    """One source-backed query target."""

    # the target module profile
    module: QueryModule
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
        encode_query_target(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> QueryTarget:
        """Decode one QueryTarget."""
        return decode_query_target(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_target(self)

    @classmethod
    def from_json(cls, value: Json) -> QueryTarget:
        """Return one QueryTarget from one JSON value."""
        return from_json_query_target(value)


def encode_query_target(writer: BinaryWriter, value: QueryTarget) -> None:
    """Encode one QueryTarget."""
    encode_query_module(writer, value.module)
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


def decode_query_target(reader: BinaryReader) -> QueryTarget:
    """Decode one QueryTarget."""
    module = decode_query_module(reader)
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

    return QueryTarget(
        module=module,
        span=span,
        selection_span=selection_span,
        symbol_id=symbol_id,
        node_id=node_id,
    )


def to_json_query_target(value: QueryTarget) -> Json:
    """Return one JSON value for one QueryTarget."""
    return {
        "module": to_json_query_module(value.module),
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


def from_json_query_target(value: Json) -> QueryTarget:
    """Return one QueryTarget from one JSON value."""
    object_ = json_object(value)

    return QueryTarget(
        module=from_json_query_module(json_field(object_, "module")),
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
class QueryText:
    """Query text in display formats understood by clients."""

    # plain text
    plain: str | None
    # markdown text
    markdown: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_text(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> QueryText:
        """Decode one QueryText."""
        return decode_query_text(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_text(self)

    @classmethod
    def from_json(cls, value: Json) -> QueryText:
        """Return one QueryText from one JSON value."""
        return from_json_query_text(value)


def encode_query_text(writer: BinaryWriter, value: QueryText) -> None:
    """Encode one QueryText."""
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


def decode_query_text(reader: BinaryReader) -> QueryText:
    """Decode one QueryText."""
    plain = reader.read_option(lambda: reader.read_string())
    markdown = reader.read_option(lambda: reader.read_string())

    return QueryText(
        plain=plain,
        markdown=markdown,
    )


def to_json_query_text(value: QueryText) -> Json:
    """Return one JSON value for one QueryText."""
    return {
        **({} if value.plain is None else {"plain": value.plain}),
        **({} if value.markdown is None else {"markdown": value.markdown}),
    }


def from_json_query_text(value: Json) -> QueryText:
    """Return one QueryText from one JSON value."""
    object_ = json_object(value)

    return QueryText(
        plain=json_optional(object_, "plain", lambda value: json_string(value)),
        markdown=json_optional(object_, "markdown", lambda value: json_string(value)),
    )


__all__ = [
    "QueryModule",
    "encode_query_module",
    "decode_query_module",
    "to_json_query_module",
    "from_json_query_module",
    "QueryPosition",
    "encode_query_position",
    "decode_query_position",
    "to_json_query_position",
    "from_json_query_position",
    "QueryRange",
    "encode_query_range",
    "decode_query_range",
    "to_json_query_range",
    "from_json_query_range",
    "QueryTarget",
    "encode_query_target",
    "decode_query_target",
    "to_json_query_target",
    "from_json_query_target",
    "QueryText",
    "encode_query_text",
    "decode_query_text",
    "to_json_query_text",
    "from_json_query_text",
]
