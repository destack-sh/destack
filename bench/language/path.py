import re
from typing import Optional

from bench.language.const import StructType, NodeType
from bench.language.node import p_regular, struct, Struct


class InvalidBenchPath(ValueError):
    pass


BENCH_SLUG_PATTERN = re.compile(r"^[a-z0-9-]+")
IDENTIFIER_PATTERN = re.compile(r"[\w ]+")
RELATIVE_PATTERN = re.compile(r"(\.\.)|(\.)")


@struct(StructType.BENCH_PATH)
class BenchPath(Struct):
    """
    A human-readable Bench path to reference source nodes and fields/properties. Absolute or relative.
    Paths are case-insensitive, support alphanum + spaces and use '/' as a primary separator.
    Nodes 'below' block-level are prefixed with a ':'. Fields are accessed with '.'.

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
    /
    ^ package root (not yet supported)
    $
    ^ module root (not yet supported)
    $User
    ^ module-unique node (not yet supported)
    ~
    ^ source module root (not yet supported)

    symbolx@2024-01-01/Library/Common/Utils/DateUtils
    ^ bench ^ package  ^ blocks
    symbolx@MyNewFeature:2024-01-01/Applications/Chat/MainScreen:ChatInput/Input.text
    ^ bench ^ branch     ^ package  ^ blocks                     ^ sub-nodes     ^ field

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
    block_path: tuple[str, ...] | None = p_regular(31, default=None)
    sub_node_path: tuple[str, ...] | None = p_regular(32, default=None)
    field_path: tuple[str, ...] | None = p_regular(33, default=None)

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
    def parse(path: str, root_type: NodeType = None) -> "BenchPath":
        """
        Parses a path string into a BenchPath. Uses root type to disambiguate some relative paths.
        TODO @Performance @Cleanup: simplify BenchPath.parse
         (I tried to use a single regex here, but it was really convoluted)
        """

        if not path:
            raise InvalidBenchPath("empty path")

        # optionally strip 'bench://' prefix
        if path.startswith("bench://"):
            path = path[8:]

        cur_pos = 0
        bench_slug = BENCH_SLUG_PATTERN.match(path)
        if bench_slug is not None:
            cur_pos = bench_slug.end() + 1  # eat '/'
            bench_slug = bench_slug.group(0)

        block_path: list[str] = []
        sub_node_path: list[str] | None = None
        field_path: list[str] | None = None
        is_all_relative = False

        # relative paths
        if bench_slug is None:
            # parse relative segments
            while cur_pos < len(path):
                match = RELATIVE_PATTERN.match(path, cur_pos)
                if match is None:
                    break
                cur_pos = match.end() + 1  # eat '/'
                if cur_pos < len(path) and path[cur_pos - 1] != "/":
                    raise InvalidBenchPath(
                        f"bad relative path at {cur_pos}: {path[cur_pos]} in {path}"
                    )
                block_path.append(match.group(0))
                is_all_relative = True
            if ":" not in path and root_type is not None:
                # skip straight into sub node mode if ambiguous and given root can't be above block
                if root_type not in (NodeType.BENCH, NodeType.PACKAGE, NodeType.BLOCK):
                    sub_node_path = []

        # skip straight into field parsing mode
        cur_char = path[cur_pos - 1] if cur_pos < len(path) else None
        if not is_all_relative and cur_char is not None and cur_char == ".":
            field_path = []

        # parse block path, sub block, and field
        # we use the None-ness of the arrays as our 'state machine'
        while cur_pos < len(path):
            match = IDENTIFIER_PATTERN.match(path, cur_pos)
            if match is None:
                raise InvalidBenchPath(f"bad path after {cur_pos}: {path[cur_pos:]} in {path}")
            cur_pos = match.end() + 1
            cur_char = path[cur_pos - 1] if cur_pos < len(path) else None

            if field_path is not None:
                if cur_char is not None and cur_char != ".":
                    raise InvalidBenchPath(f"bad field path at {cur_pos}: {cur_char} in {path}")
                field_path.append(match.group(0))
            elif sub_node_path is not None:
                if cur_char is not None:
                    if cur_char == ".":
                        field_path = []
                    elif cur_char != "/":
                        raise InvalidBenchPath(f"bad node path at {cur_pos}: {cur_char} in {path}")
                sub_node_path.append(match.group(0))
            else:
                if cur_char is not None:
                    if cur_char == ".":
                        field_path = []
                    elif cur_char == ":":
                        sub_node_path = []
                    elif cur_char != "/":
                        raise InvalidBenchPath(f"bad block path at {cur_pos}: {cur_char} in {path}")
                block_path.append(match.group(0))
            is_all_relative = False

        # is_all_relative is a hacky flag since we're not properly eating
        last_char = path[-1:]
        if not (re.match(r"\w", last_char) or last_char == "." and is_all_relative):
            raise InvalidBenchPath(f"cannot end in trailing: {last_char} in {path}")

        return BenchPath(
            bench_slug=bench_slug,
            block_path=tuple(block_path) if block_path else None,
            sub_node_path=tuple(sub_node_path) if sub_node_path else None,
            field_path=tuple(field_path) if field_path else None,
        )
