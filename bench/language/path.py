import re
from typing import TYPE_CHECKING, Optional, assert_never

from cachetools import LRUCache, cached

from bench.language.const import NODE_TYPES, BenchError, EnumType, StructType, enum_
from bench.language.node import BenchNode, InlineStruct, Node, Struct, struct_
from bench.language.property import p_regular
from bench.language.validation import NAME_REGEX_CHAR, SLUG_REGEX_CHAR
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
    ROOT = 1
    CURRENT = 2
    PARENT = 3
    BENCH = 4
    NAMED_NODE = 5
    SIBLING_NODE = 6
    UNIQUE_NODE = 7
    CONTAINING_NODE = 8
    PROPERTY = 10  # or field


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

    Relative:
        / -> root of this package
        . -> current node
        .. -> parent of current node
        ../.. -> parent of parent of current node

        Node -> ./Node -> node 'Node' relative to current node
        >Sibling -> sibling 'Sibling' of current node
        ~ -> closest container
        ~Node -> closest container with node 'Node'
        ^Name -> uniquely named node or child in closest container
        .property -> 'property' of current node

        Node2.property -> 'property' of 'Node2' (there must not be anything after .property)
        Node1/Node2/Node3 -> child 'Node3' of child 'Node2' of child 'Node1' of current node

        @bench -> absolute reference to bench 'bench'
        @bench/Node1/Node2/Node3 -> absolute reference to 'Node3' in current package of @bench
    """

    tokens: list[PathToken] = p_regular(31, require=True, array=True, struct=StructType.PATH_TOKEN)

    def __content_str__(self) -> str:
        return self.render()

    def __len__(self) -> int:
        return len(self.tokens)

    @property
    def is_absolute(self) -> bool:
        """Whether the path is absolute from a Bench root."""
        return len(self.tokens) > 0 and self.tokens[0].type == PathTokenType.ROOT

    @property
    def is_relative(self) -> bool:
        """Whether the path is relative to another node."""
        return not self.is_absolute

    def render(self) -> str:
        """Renders the path to a string."""
        return render_path(self)

    @staticmethod
    def parse(path: str) -> "Path":
        """Parses a path string into aPath."""
        return parse_path(path)


# see NAME_REGEX in validationl
BENCH_PATTERN = re.compile(rf"^@([{SLUG_REGEX_CHAR}]+)$")
NODE_PATTERN = re.compile(rf"^([>\^~])?([{NAME_REGEX_CHAR}\.]*)$")


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
            elif match := NODE_PATTERN.match(segment):
                node_type = PathTokenType.NAMED_NODE
                if match.group(1) == ">":
                    node_type = PathTokenType.SIBLING_NODE
                elif match.group(1) == "^":
                    node_type = PathTokenType.UNIQUE_NODE
                elif match.group(1) == "~":
                    node_type = PathTokenType.CONTAINING_NODE
                name = match.group(2)
                if not name and node_type != PathTokenType.CONTAINING_NODE:
                    raise PathSyntaxError(f"empty name in '{segment}' in '{path}'")
                token = PathToken(type=node_type, name=name or None)
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
                if not node_name:
                    if i > 0:
                        raise PathSyntaxError(
                            f"property shorthand must be first segment in '{path}'"
                        )
                    tokens.pop()
                else:
                    token.name = node_name
                property_token = PathToken(type=PathTokenType.PROPERTY, name=property_name)
                tokens.append(property_token)
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
        elif token.type == PathTokenType.SIBLING_NODE:
            path_parts.append(f">{token.name or ''}")
        elif token.type == PathTokenType.UNIQUE_NODE:
            path_parts.append(f"^{token.name or ''}")
        elif token.type == PathTokenType.CONTAINING_NODE:
            path_parts.append(f"~{token.name or ''}")
        elif token.type == PathTokenType.NAMED_NODE:
            path_parts.append(token.name)
        elif token.type == PathTokenType.PROPERTY:
            if path_parts:
                path_parts[-1] += f".{token.name}"
            else:  # property shorthand
                path_parts.append(f".{token.name}")
        else:
            raise PathLogicError(f"unknown token type: {token.type}")
    return "/".join(path_parts)


def get_node(path: str | Path, scope: Node) -> Node | None:
    """
    Resolves a node against the given scope.
    We try to be forgiving and just return None if we can't find the node / the path is weird.
    """
    if isinstance(path, str):
        path = parse_path(path)
    if len(path.tokens) == 0:
        return None
    current = scope
    for token in path.tokens:
        if token.type == PathTokenType.ROOT:
            if not isinstance(scope, BenchNode):
                raise PathLogicError(f"root references are only valid for bench nodes: {path}")
            current = scope.bench
        elif token.type == PathTokenType.CURRENT:
            pass
        elif token.type == PathTokenType.PARENT:
            current = current.parent
        elif token.type == PathTokenType.BENCH:
            if not isinstance(scope, BenchNode):
                raise PathLogicError(f"bench references are only valid for bench nodes: {path}")
            if token.name != scope.bench.name:
                raise PathLogicError(f"references to other benches are not supported: {path}")
            else:
                current = scope.bench
        elif token.type == PathTokenType.NAMED_NODE or token.type == PathTokenType.SIBLING_NODE:
            if token.type == PathTokenType.SIBLING_NODE:
                current = current.parent
                if current is None:
                    return None
            for child in current._graph.iter_descendants(current):
                if getattr(child, "name", None) == token.name:
                    current = child
                    break
            else:
                return None
        elif token.type == PathTokenType.UNIQUE_NODE:
            raise NotImplementedError(f"unique node references are not yet supported: {path}")
        elif token.type == PathTokenType.CONTAINING_NODE:
            raise NotImplementedError(f"containing node references are not yet supported: {path}")
        elif token.type == PathTokenType.PROPERTY:
            fields = getattr(current, "fields", None)
            if fields is not None:
                current = fields.get(token.name)
            if current is None:
                return current.__properties__.get(token.name)
        else:
            assert_never(token.type)
        if current is None:
            return None
    return current


def get_node_or_error(path: str | Path, scope: Node) -> Node:
    """Resolves a node against the given scope or raises an error."""
    node = get_node(path, scope)
    if node is None:
        raise PathLookupError(f"node at {path} not found in {scope!r}")
    return node
