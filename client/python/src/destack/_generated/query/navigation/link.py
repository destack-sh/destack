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
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.query.protocol.target
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class LinksRequest:
    """Request links for a module."""

    # the queried module
    module: destack._generated.query.protocol.target.Module

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_links_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LinksRequest:
        """Decode one LinksRequest."""
        return decode_links_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_links_request(self)

    @classmethod
    def from_json(cls, value: Json) -> LinksRequest:
        """Return one LinksRequest from one JSON value."""
        return from_json_links_request(value)


def encode_links_request(writer: BinaryWriter, value: LinksRequest) -> None:
    """Encode one LinksRequest."""
    destack._generated.query.protocol.target.encode_module(writer, value.module)


def decode_links_request(reader: BinaryReader) -> LinksRequest:
    """Decode one LinksRequest."""
    module = destack._generated.query.protocol.target.decode_module(reader)

    return LinksRequest(
        module=module,
    )


def to_json_links_request(value: LinksRequest) -> Json:
    """Return one JSON value for one LinksRequest."""
    return {
        "module": destack._generated.query.protocol.target.to_json_module(value.module),
    }


def from_json_links_request(value: Json) -> LinksRequest:
    """Return one LinksRequest from one JSON value."""
    object_ = json_object(value)

    return LinksRequest(
        module=destack._generated.query.protocol.target.from_json_module(
            json_field(object_, "module")
        ),
    )


@dataclass(frozen=True, slots=True)
class LinksResponse:
    """Response payload for links queries."""

    # links
    links: Sequence[Link]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_links_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LinksResponse:
        """Decode one LinksResponse."""
        return decode_links_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_links_response(self)

    @classmethod
    def from_json(cls, value: Json) -> LinksResponse:
        """Return one LinksResponse from one JSON value."""
        return from_json_links_response(value)


def encode_links_response(writer: BinaryWriter, value: LinksResponse) -> None:
    """Encode one LinksResponse."""
    writer.write_unsigned(len(value.links))
    for item_value_links_0 in value.links:
        encode_link(writer, item_value_links_0)


def decode_links_response(reader: BinaryReader) -> LinksResponse:
    """Decode one LinksResponse."""
    links = [decode_link(reader) for _ in range(reader.read_number())]

    return LinksResponse(
        links=links,
    )


def to_json_links_response(value: LinksResponse) -> Json:
    """Return one JSON value for one LinksResponse."""
    return {
        "links": [to_json_link(item_0) for item_0 in value.links],
    }


def from_json_links_response(value: Json) -> LinksResponse:
    """Return one LinksResponse from one JSON value."""
    object_ = json_object(value)

    return LinksResponse(
        links=[
            from_json_link(item_0)
            for item_0 in json_array(json_field(object_, "links"))
        ],
    )


@dataclass(frozen=True, slots=True)
class Link:
    """A clickable link in a module."""

    # the range of the link in the module
    range: destack._generated.source.file.model.span.Span
    # the target of the link
    target: LinkTarget
    # tooltip text (shown on hover)
    tooltip: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_link(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Link:
        """Decode one Link."""
        return decode_link(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_link(self)

    @classmethod
    def from_json(cls, value: Json) -> Link:
        """Return one Link from one JSON value."""
        return from_json_link(value)


def encode_link(writer: BinaryWriter, value: Link) -> None:
    """Encode one Link."""
    destack._generated.source.file.model.span.encode_span(writer, value.range)
    encode_link_target(writer, value.target)
    if value.tooltip is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.tooltip)


def decode_link(reader: BinaryReader) -> Link:
    """Decode one Link."""
    range_ = destack._generated.source.file.model.span.decode_span(reader)
    target = decode_link_target(reader)
    tooltip = reader.read_option(lambda: reader.read_string())

    return Link(
        range=range_,
        target=target,
        tooltip=tooltip,
    )


def to_json_link(value: Link) -> Json:
    """Return one JSON value for one Link."""
    return {
        "range": destack._generated.source.file.model.span.to_json_span(value.range),
        "target": to_json_link_target(value.target),
        **({} if value.tooltip is None else {"tooltip": value.tooltip}),
    }


def from_json_link(value: Json) -> Link:
    """Return one Link from one JSON value."""
    object_ = json_object(value)

    return Link(
        range=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "range")
        ),
        target=from_json_link_target(json_field(object_, "target")),
        tooltip=json_optional(object_, "tooltip", lambda value: json_string(value)),
    )


@dataclass(frozen=True, slots=True)
class LinkTargetFile:
    """Link to a file (resolved import)."""

    # the file path
    path: str
    kind: typing.Literal["file"] = "file"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_link_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_link_target(self)


@dataclass(frozen=True, slots=True)
class LinkTargetUrl:
    """Link to a URL."""

    # the URL
    url: str
    kind: typing.Literal["url"] = "url"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_link_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_link_target(self)


@dataclass(frozen=True, slots=True)
class LinkTargetPosition:
    """Link to a position in a file."""

    # the file path
    path: str
    # line number (0-indexed)
    line: int
    # column number (0-indexed)
    column: int
    kind: typing.Literal["position"] = "position"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_link_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_link_target(self)


"""The target of a link."""
LinkTarget: typing.TypeAlias = LinkTargetFile | LinkTargetUrl | LinkTargetPosition


def encode_link_target(writer: BinaryWriter, value: LinkTarget) -> None:
    """Encode one LinkTarget."""
    if value.kind == "file":
        writer.write_unsigned(0)
        writer.write_string(value.path)
    elif value.kind == "url":
        writer.write_unsigned(1)
        writer.write_string(value.url)
    elif value.kind == "position":
        writer.write_unsigned(2)
        writer.write_string(value.path)
        writer.write_unsigned(value.line)
        writer.write_unsigned(value.column)
    else:
        raise SerdeError("unknown enum variant")


def decode_link_target(reader: BinaryReader) -> LinkTarget:
    """Decode one LinkTarget."""
    variant = reader.read_number()

    if variant == 0:
        path = reader.read_string()

        return LinkTargetFile(
            path=path,
        )
    elif variant == 1:
        url = reader.read_string()

        return LinkTargetUrl(
            url=url,
        )
    elif variant == 2:
        path = reader.read_string()
        line = reader.read_number()
        column = reader.read_number()

        return LinkTargetPosition(
            path=path,
            line=line,
            column=column,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_link_target(value: LinkTarget) -> Json:
    """Return one JSON value for one LinkTarget."""
    if value.kind == "file":
        return {
            "kind": "file",
            "path": value.path,
        }
    elif value.kind == "url":
        return {
            "kind": "url",
            "url": value.url,
        }
    elif value.kind == "position":
        return {
            "kind": "position",
            "path": value.path,
            "line": value.line,
            "column": value.column,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_link_target(value: Json) -> LinkTarget:
    """Return one LinkTarget from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "file":
        return LinkTargetFile(
            path=json_string(json_field(object_, "path")),
        )
    elif kind == "url":
        return LinkTargetUrl(
            url=json_string(json_field(object_, "url")),
        )
    elif kind == "position":
        return LinkTargetPosition(
            path=json_string(json_field(object_, "path")),
            line=json_int(json_field(object_, "line")),
            column=json_int(json_field(object_, "column")),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "LinksRequest",
    "encode_links_request",
    "decode_links_request",
    "to_json_links_request",
    "from_json_links_request",
    "LinksResponse",
    "encode_links_response",
    "decode_links_response",
    "to_json_links_response",
    "from_json_links_response",
    "Link",
    "encode_link",
    "decode_link",
    "to_json_link",
    "from_json_link",
    "LinkTarget",
    "encode_link_target",
    "decode_link_target",
    "to_json_link_target",
    "from_json_link_target",
    "LinkTargetFile",
    "LinkTargetUrl",
    "LinkTargetPosition",
]
