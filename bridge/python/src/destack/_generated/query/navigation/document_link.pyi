# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.core.target
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class DocumentLinksRequest:
    """Request document links for a document."""

    # the queried module
    module: destack._generated.query.core.target.QueryModule

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DocumentLinksRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DocumentLinksRequest: ...

def encode_document_links_request(
    writer: BinaryWriter, value: DocumentLinksRequest
) -> None: ...
def decode_document_links_request(reader: BinaryReader) -> DocumentLinksRequest: ...
def to_json_document_links_request(value: DocumentLinksRequest) -> Json: ...
def from_json_document_links_request(value: Json) -> DocumentLinksRequest: ...

@dataclass(frozen=True, slots=True)
class DocumentLinksResponse:
    """Response payload for document links queries."""

    # document links
    links: Sequence[DocumentLink]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DocumentLinksResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DocumentLinksResponse: ...

def encode_document_links_response(
    writer: BinaryWriter, value: DocumentLinksResponse
) -> None: ...
def decode_document_links_response(reader: BinaryReader) -> DocumentLinksResponse: ...
def to_json_document_links_response(value: DocumentLinksResponse) -> Json: ...
def from_json_document_links_response(value: Json) -> DocumentLinksResponse: ...

@dataclass(frozen=True, slots=True)
class DocumentLink:
    """A clickable link in a document."""

    # the range of the link in the document
    range: destack._generated.source.file.model.span.Span
    # the target of the link
    target: DocumentLinkTarget
    # tooltip text (shown on hover)
    tooltip: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DocumentLink: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DocumentLink: ...

def encode_document_link(writer: BinaryWriter, value: DocumentLink) -> None: ...
def decode_document_link(reader: BinaryReader) -> DocumentLink: ...
def to_json_document_link(value: DocumentLink) -> Json: ...
def from_json_document_link(value: Json) -> DocumentLink: ...

@dataclass(frozen=True, slots=True)
class DocumentLinkTargetFile:
    """Link to a file (resolved import)."""

    # the file path
    path: str
    kind: typing.Literal["file"] = "file"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DocumentLinkTargetUrl:
    """Link to a URL."""

    # the URL
    url: str
    kind: typing.Literal["url"] = "url"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""The target of a document link."""
DocumentLinkTarget: typing.TypeAlias = (
    DocumentLinkTargetFile | DocumentLinkTargetUrl | DocumentLinkTargetPosition
)

def encode_document_link_target(
    writer: BinaryWriter, value: DocumentLinkTarget
) -> None: ...
def decode_document_link_target(reader: BinaryReader) -> DocumentLinkTarget: ...
def to_json_document_link_target(value: DocumentLinkTarget) -> Json: ...
def from_json_document_link_target(value: Json) -> DocumentLinkTarget: ...

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
