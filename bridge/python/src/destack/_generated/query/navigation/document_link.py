# generated bridge target, do not edit

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

import destack._generated.query.core.target
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class DocumentLinksRequest:
    """Request document links for a document."""

    # the queried module
    module: destack._generated.query.core.target.QueryModule

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_document_links_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DocumentLinksRequest:
        """Decode one DocumentLinksRequest."""
        return decode_document_links_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_document_links_request(self)

    @classmethod
    def from_json(cls, value: Json) -> DocumentLinksRequest:
        """Return one DocumentLinksRequest from one JSON value."""
        return from_json_document_links_request(value)


def encode_document_links_request(
    writer: BinaryWriter, value: DocumentLinksRequest
) -> None:
    """Encode one DocumentLinksRequest."""
    destack._generated.query.core.target.encode_query_module(writer, value.module)


def decode_document_links_request(reader: BinaryReader) -> DocumentLinksRequest:
    """Decode one DocumentLinksRequest."""
    module = destack._generated.query.core.target.decode_query_module(reader)

    return DocumentLinksRequest(
        module=module,
    )


def to_json_document_links_request(value: DocumentLinksRequest) -> Json:
    """Return one JSON value for one DocumentLinksRequest."""
    return {
        "module": destack._generated.query.core.target.to_json_query_module(
            value.module
        ),
    }


def from_json_document_links_request(value: Json) -> DocumentLinksRequest:
    """Return one DocumentLinksRequest from one JSON value."""
    object_ = json_object(value)

    return DocumentLinksRequest(
        module=destack._generated.query.core.target.from_json_query_module(
            json_field(object_, "module")
        ),
    )


@dataclass(frozen=True, slots=True)
class DocumentLinksResponse:
    """Response payload for document links queries."""

    # document links
    links: Sequence[DocumentLink]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_document_links_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DocumentLinksResponse:
        """Decode one DocumentLinksResponse."""
        return decode_document_links_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_document_links_response(self)

    @classmethod
    def from_json(cls, value: Json) -> DocumentLinksResponse:
        """Return one DocumentLinksResponse from one JSON value."""
        return from_json_document_links_response(value)


def encode_document_links_response(
    writer: BinaryWriter, value: DocumentLinksResponse
) -> None:
    """Encode one DocumentLinksResponse."""
    writer.write_unsigned(len(value.links))
    for item_value_links_0 in value.links:
        encode_document_link(writer, item_value_links_0)


def decode_document_links_response(reader: BinaryReader) -> DocumentLinksResponse:
    """Decode one DocumentLinksResponse."""
    links = [decode_document_link(reader) for _ in range(reader.read_number())]

    return DocumentLinksResponse(
        links=links,
    )


def to_json_document_links_response(value: DocumentLinksResponse) -> Json:
    """Return one JSON value for one DocumentLinksResponse."""
    return {
        "links": [to_json_document_link(item_0) for item_0 in value.links],
    }


def from_json_document_links_response(value: Json) -> DocumentLinksResponse:
    """Return one DocumentLinksResponse from one JSON value."""
    object_ = json_object(value)

    return DocumentLinksResponse(
        links=[
            from_json_document_link(item_0)
            for item_0 in json_array(json_field(object_, "links"))
        ],
    )


@dataclass(frozen=True, slots=True)
class DocumentLink:
    """A clickable link in a document."""

    # the range of the link in the document
    range: destack._generated.source.file.model.span.Span
    # the target of the link
    target: DocumentLinkTarget
    # tooltip text (shown on hover)
    tooltip: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_document_link(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DocumentLink:
        """Decode one DocumentLink."""
        return decode_document_link(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_document_link(self)

    @classmethod
    def from_json(cls, value: Json) -> DocumentLink:
        """Return one DocumentLink from one JSON value."""
        return from_json_document_link(value)


def encode_document_link(writer: BinaryWriter, value: DocumentLink) -> None:
    """Encode one DocumentLink."""
    destack._generated.source.file.model.span.encode_span(writer, value.range)
    encode_document_link_target(writer, value.target)
    if value.tooltip is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.tooltip)


def decode_document_link(reader: BinaryReader) -> DocumentLink:
    """Decode one DocumentLink."""
    range_ = destack._generated.source.file.model.span.decode_span(reader)
    target = decode_document_link_target(reader)
    tooltip = reader.read_option(lambda: reader.read_string())

    return DocumentLink(
        range=range_,
        target=target,
        tooltip=tooltip,
    )


def to_json_document_link(value: DocumentLink) -> Json:
    """Return one JSON value for one DocumentLink."""
    return {
        "range": destack._generated.source.file.model.span.to_json_span(value.range),
        "target": to_json_document_link_target(value.target),
        **({} if value.tooltip is None else {"tooltip": value.tooltip}),
    }


def from_json_document_link(value: Json) -> DocumentLink:
    """Return one DocumentLink from one JSON value."""
    object_ = json_object(value)

    return DocumentLink(
        range=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "range")
        ),
        target=from_json_document_link_target(json_field(object_, "target")),
        tooltip=json_optional(object_, "tooltip", lambda value: json_string(value)),
    )


@dataclass(frozen=True, slots=True)
class DocumentLinkTargetFile:
    """Link to a file (resolved import)."""

    # the file path
    path: str
    kind: typing.Literal["file"] = "file"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_document_link_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_document_link_target(self)


@dataclass(frozen=True, slots=True)
class DocumentLinkTargetUrl:
    """Link to a URL."""

    # the URL
    url: str
    kind: typing.Literal["url"] = "url"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_document_link_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_document_link_target(self)


@dataclass(frozen=True, slots=True)
class DocumentLinkTargetPosition:
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
        encode_document_link_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_document_link_target(self)


"""The target of a document link."""
DocumentLinkTarget: typing.TypeAlias = (
    DocumentLinkTargetFile | DocumentLinkTargetUrl | DocumentLinkTargetPosition
)


def encode_document_link_target(
    writer: BinaryWriter, value: DocumentLinkTarget
) -> None:
    """Encode one DocumentLinkTarget."""
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


def decode_document_link_target(reader: BinaryReader) -> DocumentLinkTarget:
    """Decode one DocumentLinkTarget."""
    variant = reader.read_number()

    if variant == 0:
        path = reader.read_string()

        return DocumentLinkTargetFile(
            path=path,
        )
    elif variant == 1:
        url = reader.read_string()

        return DocumentLinkTargetUrl(
            url=url,
        )
    elif variant == 2:
        path = reader.read_string()
        line = reader.read_number()
        column = reader.read_number()

        return DocumentLinkTargetPosition(
            path=path,
            line=line,
            column=column,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_document_link_target(value: DocumentLinkTarget) -> Json:
    """Return one JSON value for one DocumentLinkTarget."""
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


def from_json_document_link_target(value: Json) -> DocumentLinkTarget:
    """Return one DocumentLinkTarget from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "file":
        return DocumentLinkTargetFile(
            path=json_string(json_field(object_, "path")),
        )
    elif kind == "url":
        return DocumentLinkTargetUrl(
            url=json_string(json_field(object_, "url")),
        )
    elif kind == "position":
        return DocumentLinkTargetPosition(
            path=json_string(json_field(object_, "path")),
            line=json_int(json_field(object_, "line")),
            column=json_int(json_field(object_, "column")),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "DocumentLinksRequest",
    "encode_document_links_request",
    "decode_document_links_request",
    "to_json_document_links_request",
    "from_json_document_links_request",
    "DocumentLinksResponse",
    "encode_document_links_response",
    "decode_document_links_response",
    "to_json_document_links_response",
    "from_json_document_links_response",
    "DocumentLink",
    "encode_document_link",
    "decode_document_link",
    "to_json_document_link",
    "from_json_document_link",
    "DocumentLinkTarget",
    "encode_document_link_target",
    "decode_document_link_target",
    "to_json_document_link_target",
    "from_json_document_link_target",
    "DocumentLinkTargetFile",
    "DocumentLinkTargetUrl",
    "DocumentLinkTargetPosition",
]
