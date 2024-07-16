import re
from typing import TYPE_CHECKING, Optional, assert_never, cast
from uuid import UUID

from cachetools import LRUCache, cached

from bench.language.bench import Bench, Branch, Package
from bench.language.const import BenchError, EnumType, NodeType, StructType, enum_
from bench.language.graph import NodeList
from bench.language.node import (
    BenchNode,
    InlineStruct,
    Node,
    PackageNode,
    SourceNode,
    Struct,
    struct_,
)
from bench.language.property import p_regular
from bench.language.validation import NAME_REGEX_CHAR, SLUG_REGEX_CHAR
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language.field import Field


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
    CHILD = 5
    CONTAINER = 7
    UNIQUE = 8
    FIELD = 10


@struct_(StructType.PATH_TOKEN, inline=True)
class PathToken(InlineStruct):
    """A semantic part of a Bench path."""

    type: PathTokenType = p_regular(31)
    name: Optional[str] = p_regular(32, default=None)

    def __content_str__(self) -> str:
        if self.name:
            return f"{self.type.bench_name} {self.name}"
        else:
            return self.type.bench_name


@struct_(StructType.PATH)
class Path(Struct):
    """
    A human-readable Bench path to reference nodes.
    Paths names and special operators combined with slashes.
    Fields may also be accessed with '.' separators.

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
                node_type = PathTokenType.CHILD
                if match.group(1) == "~":
                    node_type = PathTokenType.CONTAINER
                elif match.group(1) == "^":
                    node_type = PathTokenType.UNIQUE
                name = match.group(2)
                if not name and node_type != PathTokenType.CONTAINER:
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
                property_token = PathToken(type=PathTokenType.FIELD, name=property_name)
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
        elif token.type == PathTokenType.CONTAINER:
            path_parts.append(f"~{token.name or ''}")
        elif token.type == PathTokenType.UNIQUE:
            path_parts.append(f"^{token.name or ''}")
        elif token.type == PathTokenType.CHILD:
            path_parts.append(token.name)
        elif token.type == PathTokenType.FIELD:
            if path_parts:
                path_parts[-1] += f".{token.name}"
            else:  # property shorthand
                path_parts.append(f".{token.name}")
        else:
            assert_never(token.type)
    return "/".join(path_parts)


# NOTE :Performance: should probably index some of the path lookups in the graph?


def _normalize_node(scope: Node) -> Node:
    """'Normalize' the scope, lowering a bench into its current main package."""
    if scope.metatype == NodeType.BENCH:
        bench = cast(Bench, scope)
        main_branch = bench.main_branch
        assert main_branch is not None, f"bench {bench!r} has no main branch"
        main_package = main_branch.main_package
        assert main_package is not None, f"branch {main_branch!r} has no main package"
        return main_package
    elif scope.metatype == NodeType.BRANCH:
        branch = cast(Branch, scope)
        main_package = branch.main_package
        assert main_package is not None, f"branch {branch!r} has no main package"
        return main_package
    else:
        return scope


def _get_child(scope: Node, name: str, node_type: NodeType | None = None) -> Node | None:
    """Finds a named child from a scope (if any)."""
    for child in scope._graph.iter_descendants(scope, node_type=node_type):
        if getattr(child, "name", None) == name:
            return child
    return None


def _get_descendant(scope: Node, name: str, node_type: NodeType | None = None) -> Node | None:
    """Finds any named descendant from a scope (if any, ignoring container boundaries)."""
    for descendant in scope._graph.iter_descendants(scope, recursive=True, node_type=node_type):
        if getattr(descendant, "name", None) == name:
            return descendant
    return None


def _get_contained_descendant(scope: Node, name: str) -> Node | None:
    """Finds a descendant that is directly contained by a scope (if any)."""
    from bench.language.block import Block

    if not isinstance(scope, Block):
        # just get children
        for node_type in (NodeType.SPACE, NodeType.BLOCK):
            if node := _get_child(scope, name, node_type):
                return node
    else:
        # recurse blocks until we hit pages
        blocks = [scope]
        while blocks:
            block = blocks.pop()
            for child in block.blocks:
                if child.name == name:
                    return child
                if not child.is_page:
                    blocks.append(child)

        # and recurse own views/fields/triggers/steps
        for node_type in (NodeType.TRIGGER, NodeType.FIELD, NodeType.QUERY):
            if node := _get_child(scope, name, node_type):
                return node
        for node_type in (NodeType.VIEW, NodeType.STEP):
            if node := _get_descendant(scope, name, node_type):
                return node

    return None


def _get_container(scope: Node, name: str | None = None) -> Node | None:
    """Finds the next containing ancestor up from a scope (if any)."""

    if not isinstance(scope, SourceNode):
        # there is no container outside of source other than the bench
        if isinstance(scope, PackageNode) and (
            name is None or getattr(scope, "name", None) == name
        ):
            return scope.package
    elif scope.metatype != NodeType.BLOCK:
        # if we're not in a block, find containing block or space (or skip to bench)
        parent = scope.parent
        while parent is not None:
            if parent.metatype in (NodeType.BLOCK, NodeType.SPACE, NodeType.BENCH) and (
                name is None or getattr(parent, "name", None) == name
            ):
                return parent
            parent = parent.parent
    else:
        # if we're in a block, find next block that's a container/page or (or skip to bench)
        from bench.language.block import Block

        parent = scope.parent
        while parent is not None:
            if (
                (isinstance(parent, Block) and parent.is_page) or parent.metatype == NodeType.BENCH
            ) and (name is None or getattr(parent, "name", None) == name):
                return parent
            parent = parent.parent

    return None


def _get_unique(scope: Node, name: str) -> Node | None:
    """
    Finds a uniquely named node in any containing ancestor scope.
    The order of search is:
     1. 'Siblings' - descendents of parent container.
     2. Descendants - descendants of scope.
     3. Ancestors - descendants of ancestor containers.
    The rationale is that we generally want to refer to nodes in the same container
     more than we want our child nodes (like an output Field with the name of a ChoiceBlock,
     or other Steps in the same Flows more than our input Fields).
    """
    # 'siblings'
    parent = _get_container(scope)
    if parent is not None and (node := _get_contained_descendant(parent, name)) is not None:
        return node
    # descendants
    if (node := _get_contained_descendant(scope, name)) is not None:
        return node
    # ancestors
    if parent is None:
        return None
    parent = _get_container(parent)
    while parent is not None:
        descendant = _get_contained_descendant(parent, name)
        if descendant is not None:
            return descendant
        parent = _get_container(parent)
    return None


def get_node(scope: Node, path: str | Path) -> Node | None:
    """
    Resolves a node against the given scope.
    We try to be forgiving and just return None if we can't find the node / the path is weird.
    """
    scope = _normalize_node(scope)
    if isinstance(path, str):
        path = parse_path(path)
    if len(path.tokens) == 0:
        return None
    current = scope
    for token in path.tokens:
        if token.type == PathTokenType.ROOT:
            if not isinstance(scope, (Package, PackageNode)):
                raise PathLogicError(f"root references are only valid for bench nodes: {path}")
            current = scope.package
        elif token.type == PathTokenType.CURRENT:
            pass
        elif token.type == PathTokenType.PARENT:
            if current.metatype == NodeType.PACKAGE:
                return None  # has no parent in path
            current = current.parent
        elif token.type == PathTokenType.BENCH:
            if not isinstance(scope, (Package, PackageNode)):
                raise PathLogicError(f"bench references are only valid for bench nodes: {path}")
            if token.name != scope.package.name:
                raise PathLogicError(f"references to other benches are not supported: {path}")
            else:
                current = scope.package
        elif token.type == PathTokenType.CHILD:
            assert token.name, f"missing name for {token!r} in {path!r}"
            current = _get_child(current, token.name)
        elif token.type == PathTokenType.CONTAINER:
            if current.metatype == NodeType.PACKAGE:
                return None  # has no parent in path
            current = _get_container(current, token.name)
        elif token.type == PathTokenType.UNIQUE:
            assert token.name, f"missing name for {token!r} in {path!r}"
            current = _get_unique(current, token.name)
        elif token.type == PathTokenType.FIELD:
            assert token.name, f"missing name for {token!r} in {path!r}"
            fields = cast(NodeList["Field"] | None, getattr(current, "fields", None))
            if fields is not None:
                current = fields.get(token.name)
            else:
                return None
        else:
            assert_never(token.type)
        if current is None:
            return None
    return current


def get_node_or_error(scope: Node, path: str | Path) -> Node:
    """Resolves a node against the given scope or raises an error."""
    node = get_node(scope, path)
    if node is None:
        raise PathLookupError(f"node at {path} not found in {scope!r}")
    return node


def _get_path_to_root(node: Node) -> list[Node]:
    """Gets the path relevant ancestors to a node (including the node, up to package)"""
    graph = node._graph
    ancestors: list[Node] = [node]
    cur = node
    while cur.parent_ptr is not None and cur.metatype != NodeType.PACKAGE:
        cur = graph._nodes_by_id[cast(UUID, cur.parent_ptr.id)]
        ancestors.append(cur)
    return ancestors


def get_path(scope: Node, node: Node) -> Path:
    """Finds a path to the given node from a scope. Basically an inverse of get_node."""
    scope = _normalize_node(scope)
    node = _normalize_node(node)

    if scope == node:
        if isinstance(node, Package):
            return Path(tokens=[PathToken(type=PathTokenType.BENCH, name=node.name)])
        else:
            return Path(tokens=[PathToken(type=PathTokenType.CURRENT)])

    scope_path = _get_path_to_root(scope)
    node_path = _get_path_to_root(node)
    if scope_path[-1] != node_path[-1] or isinstance(node, Package):
        # make absolute path (different bench)
        if (
            not isinstance(node, BenchNode)
            or not node_path
            or not isinstance(node_path[-1], Package)
        ):
            raise PathLogicError(f"no common ancestor found for {scope!r} and {node!r}")
        tokens = [PathToken(type=PathTokenType.BENCH, name=node_path[-1].name)]
        for node_ancestor in node_path[1:]:
            name = getattr(node_ancestor, "name", None)
            assert name is not None, f"no name for {node_ancestor!r}"
            tokens.append(PathToken(type=PathTokenType.CHILD, name=name))
    else:
        # find relative path from scope to node (up/down)
        common_ancestor = None
        scope_ancestor_idx = 0  # to ensure type checker that it will be assigned
        for node_ancestor_idx, node_ancestor in enumerate(node_path):  # noqa: B007
            for scope_ancestor_idx, scope_ancestor in enumerate(scope_path):  # noqa: B007
                if node_ancestor == scope_ancestor:
                    common_ancestor = node_ancestor
                    break
            if common_ancestor is not None:
                break
        else:
            raise PathLogicError(f"no common ancestor found for {scope!r} and {node!r}")
        if isinstance(common_ancestor, Package):
            # absolute path from bench to node
            tokens = [PathToken(type=PathTokenType.ROOT)]
            for node_ancestor in reversed(node_path[:-1]):
                name = getattr(node_ancestor, "name", None)
                assert name is not None, f"no name for {node_ancestor!r}"
                tokens.append(PathToken(type=PathTokenType.CHILD, name=name))
        elif node_ancestor_idx == 0:
            # node is a direct ancestor of scope
            tokens = []
            for i in range(1, scope_ancestor_idx + 1):
                name = getattr(scope_path[i], "name", None)
                assert name is not None, f"no name for {scope_path[i]!r}"
                tokens.append(PathToken(type=PathTokenType.CONTAINER, name=name))
        else:
            # get from scope to common ancestor, then from common ancestor to node
            tokens = []
            for i in range(1, scope_ancestor_idx):
                name = getattr(scope_path[i], "name", None)
                assert name is not None, f"no name for {scope_path[i]!r}"
                tokens.append(PathToken(type=PathTokenType.CONTAINER, name=name))
            for i in range(node_ancestor_idx - 1, -1, -1):
                name = getattr(node_path[i], "name", None)
                assert name is not None, f"no name for {node_path[i]!r}"
                tokens.append(PathToken(type=PathTokenType.CHILD, name=name))

    if node.metatype == NodeType.FIELD:
        tokens[-1].type = PathTokenType.FIELD
    return Path(tokens=tokens)
