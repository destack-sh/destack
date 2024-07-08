import re
from typing import TYPE_CHECKING, Optional

from cachetools import LRUCache, cached

from bench.language.const import NODE_TYPES, BenchError, EnumType, StructType, enum_
from bench.language.node import InlineStruct, Node, Struct, struct_
from bench.language.property import p_regular
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    pass


class PathError(BenchError, ValueError):
    pass


class PathSyntaxError(PathError):
    pass


class PathLogicError(PathError):
    pass


class PathLookupError(PathError, LookupError):
    pass


@enum_(EnumType.PATH_TOKEN_TYPE)
class PathTokenType(IdEnum):
    # named
    BENCH = 1
    NODE = 2
    UNIQUE_NODE = 3
    PROPERTY = 5  # or field
    # relative
    ROOT = 10
    CURRENT = 11
    PARENT = 12


@struct_(StructType.PATH_TOKEN, inline=True)
class PathToken(InlineStruct):
    """A semantic part of a Bench path."""

    type: PathTokenType = p_regular(31)
    name: Optional[str] = p_regular(32, default=None)
    node: Optional["Node"] = p_regular(
        33, require=False, array=False, default=None, references=NODE_TYPES.tuple
    )


@struct_(StructType.PATH)
class Path(Struct):
    """
    A human-readable Bench path to reference nodes and their properties.
    Paths are case-insensitive, support alphanum + spaces and use '/' as the primary node separator.
    Properties must be accessed with '.' separators (also works for Fields for consistency).

    / -> root of this package
    ./ -> current node
    ../.. -> parent of parent of current node
    Name -> ./Name -> Name relative to current node
    Node1/Node2.property -> property of Node2 (there must not be anything after .property)

    @bench -> absolute reference to bench
    @bench/Node1/Node2/Node3 -> absolute reference to Node3 in package
    """

    tokens: list[PathToken] = p_regular(31, require=True, array=True, struct=StructType.PATH_TOKEN)

    def __content_str__(self) -> str:
        return self.render()

    def render(self) -> str:
        """Renders the path to a string."""
        return render_path(self)

    @staticmethod
    def parse(path: str) -> "Path":
        """Parses a path string into aPath."""
        return parse_path(path)


BENCH_PATTERN = re.compile(r"^@([^/]+)")
NODE_PATTERN = re.compile(r"^([a-zA-Z0-9_\s\.]+)")
UNIQUE_NODE_PATTERN = re.compile(r"^#([a-zA-Z0-9_\s\.]+)")


@cached(LRUCache(maxsize=1024 * 10))
def parse_path(path: str) -> Path:
    # plain root references
    if path == "/":
        return Path(tokens=[PathToken(type=PathTokenType.ROOT)])

    segments = path.strip().split("/")

    # empty path
    if not segments:
        return Path(tokens=[])

    tokens: list[PathToken] = []
    if segments[0] == "":
        # skip the root segment
        tokens.append(PathToken(type=PathTokenType.ROOT))
        segments.pop(0)
    for i, segment in enumerate(segments):
        if not segment:
            raise PathSyntaxError(f"empty name in '{path}'")
        elif segment == ".":
            token = PathToken(type=PathTokenType.CURRENT)
            tokens.append(token)
        elif segment == "..":
            token = PathToken(type=PathTokenType.PARENT)
            tokens.append(token)
        else:
            # named nodes
            if match := BENCH_PATTERN.match(segment):
                if len(tokens) > 0:
                    raise PathLogicError(f"bench reference must be the first segment in '{path}'")
                token = PathToken(type=PathTokenType.BENCH, name=match.group(1))
                if len(match.group(1)) != len(segment) - 1:
                    raise PathSyntaxError(f"invalid bench name '{segment}' in '{path}'")
            elif match := UNIQUE_NODE_PATTERN.match(segment):
                token = PathToken(type=PathTokenType.UNIQUE_NODE, name=match.group(1))
                if len(match.group(1)) != len(segment) - 1:
                    raise PathSyntaxError(f"invalid node name '{segment}' in '{path}'")
            elif match := NODE_PATTERN.match(segment):
                token = PathToken(type=PathTokenType.NODE, name=match.group(1))
                if len(match.group(1)) != len(segment):
                    raise PathSyntaxError(f"invalid node name '{segment}' in '{path}'")
            else:
                raise PathSyntaxError(f"invalid path: '{segment}' in '{path}'")
            tokens.append(token)

            # get property (if any)
            if "." in segment:
                node_name, property_name = segment.split(".", maxsplit=1)
                if "." in property_name:
                    raise PathSyntaxError(f"cannot nest properties: '{segment}' in '{path}'")
                if i < len(segments) - 1:
                    raise PathSyntaxError(f"property must be the last segment in '{path}'")
                property_token = PathToken(type=PathTokenType.PROPERTY, name=property_name)
                tokens.append(property_token)
                token.name = node_name
            elif "." in segment:
                raise PathSyntaxError(f"invalid property syntax: '{segment}' in '{path}'")

    return Path(tokens=tokens)


def render_path(path: Path) -> str:
    """Renders a path back into a string."""
    path_parts = []
    for token in path.tokens:
        if token.type == PathTokenType.ROOT:
            path_parts.append("")
            if len(path.tokens) == 1:
                return "/"
        elif token.type == PathTokenType.CURRENT:
            path_parts.append(".")
        elif token.type == PathTokenType.PARENT:
            path_parts.append("..")
        elif token.type == PathTokenType.BENCH:
            path_parts.append(f"@{token.name}")
        elif token.type == PathTokenType.UNIQUE_NODE:
            path_parts.append(f"#{token.name}")
        elif token.type == PathTokenType.NODE:
            path_parts.append(token.name)
        elif token.type == PathTokenType.PROPERTY:
            path_parts[-1] += f".{token.name}"
        else:
            raise PathLogicError(f"unknown token type: {token.type}")
    return "/".join(path_parts)


def get_node(path: str | Path, scope: Node):
    """Resolves a node against the given scope."""
    raise NotImplementedError("nocheckin: get_node")
