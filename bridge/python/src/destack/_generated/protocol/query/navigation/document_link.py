# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.core.target
import destack._generated.protocol.source.file.model.span

if TYPE_CHECKING:
    from destack._generated.protocol.query.core.target import (
        QueryModule,
    )

    from destack._generated.protocol.source.file.model.span import (
        Span,
    )


@dataclass(frozen=True, slots=True)
class DocumentLinksRequest:
    """Request document links for a document."""

    """The queried module."""
    module: QueryModule


def encode_document_links_request(writer: Writer, value: DocumentLinksRequest) -> None:
    destack._generated.protocol.query.core.target.encode_query_module(
        writer, value.module
    )


def decode_document_links_request(reader: Reader) -> DocumentLinksRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_module(reader)

    return DocumentLinksRequest(
        module=field_0,
    )


@dataclass(frozen=True, slots=True)
class DocumentLinksResponse:
    """Response payload for document links queries."""

    """Document links."""
    links: Sequence[DocumentLink]


def encode_document_links_response(
    writer: Writer, value: DocumentLinksResponse
) -> None:
    writer.write_unsigned(len(value.links))
    for item_0 in value.links:
        encode_document_link(writer, item_0)


def decode_document_links_response(reader: Reader) -> DocumentLinksResponse:
    field_0 = [decode_document_link(reader) for _ in range(reader.read_number())]

    return DocumentLinksResponse(
        links=field_0,
    )


@dataclass(frozen=True, slots=True)
class DocumentLink:
    """A clickable link in a document."""

    """The range of the link in the document."""
    range: Span
    """The target of the link."""
    target: DocumentLinkTarget
    """Tooltip text (shown on hover)."""
    tooltip: str | None


def encode_document_link(writer: Writer, value: DocumentLink) -> None:
    destack._generated.protocol.source.file.model.span.encode_span(writer, value.range)
    encode_document_link_target(writer, value.target)
    if value.tooltip is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.tooltip)


def decode_document_link(reader: Reader) -> DocumentLink:
    field_0 = destack._generated.protocol.source.file.model.span.decode_span(reader)
    field_1 = decode_document_link_target(reader)
    field_2 = reader.read_option(lambda: reader.read_string())

    return DocumentLink(
        range=field_0,
        target=field_1,
        tooltip=field_2,
    )


@dataclass(frozen=True, slots=True)
class DocumentLinkTargetFile:
    """Link to a file (resolved import)."""

    """The file path."""
    path: str
    kind: Literal["file"] = "file"


@dataclass(frozen=True, slots=True)
class DocumentLinkTargetUrl:
    """Link to a URL."""

    """The URL."""
    url: str
    kind: Literal["url"] = "url"


@dataclass(frozen=True, slots=True)
class DocumentLinkTargetPosition:
    """Link to a position in a file."""

    """The file path."""
    path: str
    """Line number (0-indexed)."""
    line: int
    """Column number (0-indexed)."""
    column: int
    kind: Literal["position"] = "position"


"""The target of a document link."""
DocumentLinkTarget: TypeAlias = (
    DocumentLinkTargetFile | DocumentLinkTargetUrl | DocumentLinkTargetPosition
)


def encode_document_link_target(writer: Writer, value: DocumentLinkTarget) -> None:
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


def decode_document_link_target(reader: Reader) -> DocumentLinkTarget:
    variant = reader.read_number()

    if variant == 0:
        field_0 = reader.read_string()

        return DocumentLinkTargetFile(
            path=field_0,
        )
    elif variant == 1:
        field_0 = reader.read_string()

        return DocumentLinkTargetUrl(
            url=field_0,
        )
    elif variant == 2:
        field_0 = reader.read_string()
        field_1 = reader.read_number()
        field_2 = reader.read_number()

        return DocumentLinkTargetPosition(
            path=field_0,
            line=field_1,
            column=field_2,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


__all__ = [
    "DocumentLinksRequest",
    "encode_document_links_request",
    "decode_document_links_request",
    "DocumentLinksResponse",
    "encode_document_links_response",
    "decode_document_links_response",
    "DocumentLink",
    "encode_document_link",
    "decode_document_link",
    "DocumentLinkTarget",
    "encode_document_link_target",
    "decode_document_link_target",
    "DocumentLinkTargetFile",
    "DocumentLinkTargetUrl",
    "DocumentLinkTargetPosition",
]
