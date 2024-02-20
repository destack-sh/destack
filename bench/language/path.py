import re
from typing import Optional

from bench.language.const import StructType
from bench.language.node import struct, Struct
from bench.language.property import p_regular


class InvalidBenchPath(ValueError):
    pass


BENCH_SLUG_PATTERN = re.compile(r"^[a-z0-9-]+")
IDENTIFIER_PATTERN = re.compile(r"[\w ]+")
RELATIVE_PATTERN = re.compile(r"(\.\.)|(\.)")


@struct(StructType.PATH_SEGMENT, inline=True)
class PathSegment(Struct):
    pass


@struct(StructType.PATH_TOKEN, inline=True)
class PathToken(Struct):
    pass


@struct(StructType.PATH)
class Path(Struct):
    """
    A human-readable Bench path to reference source nodes and their fields/properties. Absolute or relative.
    Paths are case-insensitive, support alphanum + spaces and use '/' as the primary node separator.
    Nodes 'below' block-level are prefixed with one ':'. Fields are accessed with '.' separators.

    flotothemoon/Mirror/Notion/Databases/Landscape
    ^ bench      ^ blocks
    flotothemoon/Applications/Birdy/MainScreen:Dashboard/Big Graphs/Graph1.name
    ^ bench      ^ blocks                      ^ sub-block                ^ field
    flotothemoon/Sandbox/Sales/Pipeline/Scraping/WebsiteSamples/Replit.document.title
    ^ bench      ^ blocks                                              ^ field  ^ field

    flotothemoon
    ^ bench
    flotothemoon.name
    ^ bench      ^ field
    flotothemoon-tests/Tests/Databases/TestPopulate.code
    ^ bench            ^ blocks                     ^ field

    .
    ^ current
    ..
    ^ parent
    ../../Header Screen:Header/Title.theme.primary.color
    ^ blocks           ^ sub-nodes  ^ field
    ../../../../Graphs
    ^ parents

    symbolx@2024-01-01/Library/Common/Utils/DateUtils
    ^ bench ^ package  ^ blocks
    symbolx@MyNewFeature:2024-01-01/Applications/Chat/MainScreen:ChatInput/Input.text
    ^ bench ^ branch     ^ package  ^ blocks                     ^ sub-nodes     ^ field

    NOT YET SUPPORTED:
        / -> package (=Package)
        $ -> module (=Block|Package)
        ^ -> page (=Block)
        $User -> module-unique node (=Block|View)
        ~ -> source module root (like $ but for templated)
        [<expr like ck=...>] -> dynamic Expression filter

    ''
    ERROR (invalid, empty path)
    '../'
    ERROR (invalid, trailing slash)
    ../../Something/../SomethingElse
    ERROR (invalid, cannot go up and down in the same path)

    The general syntax is:
    [bench-name][@branch-name][:package-name][/[block-name][:sub-node-name]][.field-name]
    For absolute paths, the bench name is required.
    """

    bench_slug: Optional[str] = p_regular(30, default=None)
    block_path: list[str, ...] = p_regular(31, array=True)
    sub_node_path: list[str, ...] = p_regular(32, array=True)
    field_path: list[str, ...] = p_regular(33, array=True)

    branch_slug: Optional[str] = p_regular(34, default=None)  # (not yet supported)
    package_slug: Optional[str] = p_regular(35, default=None)  # (not yet supported)

    def __content_str__(self) -> str:
        path_str = self.bench_slug or ""
        if self.block_path:
            path_str += "/" + "/".join(self.block_path)
        if self.sub_node_path:
            path_str += ":" + "/".join(self.sub_node_path)
        if self.field_path:
            path_str += "." + ".".join(self.field_path)
        return path_str

    @property
    def is_absolute(self) -> bool:
        return self.bench_slug is not None

    @property
    def is_relative(self) -> bool:
        return self.bench_slug is None

    @staticmethod
    def parse(path: str) -> "Path":
        """Parses a path string into a BenchPath."""
        raise NotImplementedError("TODO @Incomplete :BenchPath")
