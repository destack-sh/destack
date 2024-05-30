import re
from typing import TYPE_CHECKING, Mapping, Optional

from bench.language.const import EnumType, NodeType, StructType, enum_
from bench.language.node import Struct, struct
from bench.language.property import p_regular
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Node


class InvalidBenchPathError(ValueError):
    pass


BENCH_SLUG_PATTERN = re.compile(r"^[a-z0-9-]+")
IDENTIFIER_PATTERN = re.compile(r"[\w ]+")
RELATIVE_PATTERN = re.compile(r"(\.\.)|(\.)")


@enum_(EnumType.PATH_TOKEN_TYPE)
class PathTokenType(IdEnum):
    # special
    SLASH = 1
    COLON = 2
    DOT = 3
    AT = 4
    DOLLAR = 5
    CARET = 6
    TILDE = 7
    DOUBLE_DOT = 8

    # dynamic
    NAME = 10
    EXPRESSION = 11


PATH_TOKEN_TO_STR: Mapping[PathTokenType, str] = {
    PathTokenType.SLASH: "/",
    PathTokenType.COLON: ":",
    PathTokenType.DOT: ".",
    PathTokenType.AT: "@",
    PathTokenType.DOLLAR: "$",
    PathTokenType.CARET: "^",
    PathTokenType.TILDE: "~",
    PathTokenType.DOUBLE_DOT: "..",
}


@struct(StructType.PATH_TOKEN, inline=True)
class PathToken(Struct):
    type: PathTokenType = p_regular(31)


@enum_(EnumType.PATH_SEGMENT_TYPE)
class PathSegmentType(IdEnum):
    # specific
    BENCH = 1
    ENVIRONMENT = 2
    BRANCH = 3
    PACKAGE = 4
    BLOCK = 5
    SUB_BLOCK = 6
    PROPERTY = 7
    # relative
    CURRENT = 10
    CURRENT_PARENT = 11
    CURRENT_PACKAGE = 12
    CURRENT_MODULE = 13
    CURRENT_PAGE = 14
    CURRENT_SOURCE_MODULE = 15
    CURRENT_UNIQUE = 16
    # conditional
    FILTER = 20


@struct(StructType.PATH_SEGMENT, inline=True)
class PathSegment(Struct):
    type: PathSegmentType = p_regular(31)
    name: Optional[str] = p_regular(32, default=None)
    reference: Optional["Node"] = p_regular(
        33,
        require=False,
        array=False,
        default=None,
        references=(
            NodeType.BENCH,
            NodeType.ENVIRONMENT,
            NodeType.BRANCH,
            NodeType.PACKAGE,
            NodeType.BLOCK,
            NodeType.FIELD,
            NodeType.VIEW,
        ),
    )


@struct(StructType.PATH)
class Path(Struct):
    """
    A human-readable Bench path to reference source nodes and their fields/properties. Absolute or relative.
    Paths are case-insensitive, support alphanum + spaces and use '/' as the primary node separator.
    Nodes 'below' block-level are prefixed with one ':'. Fields are accessed with '.' separators.

    flotothemoon/Mirror/Notion/Databases/Landscape
    ^ bench      ^ blocks
    flotothemoon/Applications/Birdy/MainScreen:Dashboard/Big Graphs/Graph1.name
    ^ bench      ^ blocks                      ^ sub-block nod            ^ field
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

    also relative:
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

    segments: list[PathSegment] = p_regular(
        31, require=True, array=True, struct=StructType.PATH_SEGMENT
    )

    def __content_str__(self) -> str:
        return self.render()

    def render(self) -> str:
        return ":Incomplete"

    @staticmethod
    def parse(path: str) -> "Path":
        """Parses a path string into a BenchPath."""
        raise NotImplementedError("TODO :Incomplete :BenchPath")
