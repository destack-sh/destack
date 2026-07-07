# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.protocol.target
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class LinksRequest:
    """Request links for a module."""

    # the queried module
    module: destack._generated.query.protocol.target.Module

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LinksRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LinksRequest: ...

def encode_links_request(writer: BinaryWriter, value: LinksRequest) -> None: ...
def decode_links_request(reader: BinaryReader) -> LinksRequest: ...
def to_json_links_request(value: LinksRequest) -> Json: ...
def from_json_links_request(value: Json) -> LinksRequest: ...

@dataclass(frozen=True, slots=True)
class LinksResponse:
    """Response payload for links queries."""

    # links
    links: Sequence[Link]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LinksResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LinksResponse: ...

def encode_links_response(writer: BinaryWriter, value: LinksResponse) -> None: ...
def decode_links_response(reader: BinaryReader) -> LinksResponse: ...
def to_json_links_response(value: LinksResponse) -> Json: ...
def from_json_links_response(value: Json) -> LinksResponse: ...

@dataclass(frozen=True, slots=True)
class Link:
    """A clickable link in a module."""

    # the range of the link in the module
    range: destack._generated.source.file.model.span.Span
    # the target of the link
    target: LinkTarget
    # tooltip text (shown on hover)
    tooltip: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Link: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Link: ...

def encode_link(writer: BinaryWriter, value: Link) -> None: ...
def decode_link(reader: BinaryReader) -> Link: ...
def to_json_link(value: Link) -> Json: ...
def from_json_link(value: Json) -> Link: ...

@dataclass(frozen=True, slots=True)
class LinkTargetFile:
    """Link to a file (resolved import)."""

    # the file path
    path: str
    kind: typing.Literal["file"] = "file"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LinkTargetUrl:
    """Link to a URL."""

    # the URL
    url: str
    kind: typing.Literal["url"] = "url"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""The target of a link."""
LinkTarget: typing.TypeAlias = LinkTargetFile | LinkTargetUrl | LinkTargetPosition

def encode_link_target(writer: BinaryWriter, value: LinkTarget) -> None: ...
def decode_link_target(reader: BinaryReader) -> LinkTarget: ...
def to_json_link_target(value: LinkTarget) -> Json: ...
def from_json_link_target(value: Json) -> LinkTarget: ...

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
