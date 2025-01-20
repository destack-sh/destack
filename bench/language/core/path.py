from typing import (
    TYPE_CHECKING,
    Any,
    Literal,
    Mapping,
    NamedTuple,
    Optional,
    Sequence,
    Union,
    assert_never,
    cast,
)
from uuid import UUID

import regex
from cachetools import LRUCache, cached

from bench.utils.func import IdEnum
from bench.utils.string import to_code_name

from .const import EMPTY_DICT, BenchError, EnumType, NodeType, StructType, enum_
from .node import (
    BenchNode,
    HasContext,
    Node,
    NodeReference,
    PackageNode,
    PropertyReference,
    RunnableNode,
    SourceNode,
    Struct,
    struct_,
)
from .property import Property, p_regular
from .validation import NAME_REGEX_CHAR, SLUG_REGEX_CHAR

if TYPE_CHECKING:
    from bench.language import Bench, Field, Package

_property = property


class PathError(BenchError, ValueError):
    pass


class PathSyntaxError(PathError):
    pass


class PathLogicError(PathError):
    pass


class PathUnnamedNodeError(PathLogicError):
    pass


class PathLookupError(PathError, LookupError):
    pass


@enum_(EnumType.PATH_ELEMENT_TYPE)
class PathElementType(IdEnum):
    # absolute
    ROOT = 1
    BENCH = 2
    PACKAGE = 3
    NODE = 4
    # relative
    CURRENT = 10
    CONTAINER = 11
    UNIQUE = 12
    PARENT = 13
    CHILD = 14
    # sub
    ATTRIBUTE = 20
    # runtime
    CONTEXT = 30
    RUN = 31


@enum_(EnumType.PATH_RUN_SELECTOR)
class PathRunSelector(IdEnum):
    LATEST = 1


SIGN_BY_RUN_SELECTOR: dict[PathRunSelector, str] = {
    PathRunSelector.LATEST: "[-1]",
}
RUN_SELECTOR_BY_SIGN: dict[str, PathRunSelector] = {v: k for k, v in SIGN_BY_RUN_SELECTOR.items()}


@struct_(StructType.PATH_ELEMENT)
class PathElement(Struct):
    """A semantic part of a Bench path."""

    type: PathElementType = p_regular(31)
    name: Optional[str] = p_regular(32, default=None)
    node: Optional[Node] = p_regular(33, require=False, array=False, references="any")
    property: Optional[Property] = p_regular(
        34, require=False, struct=StructType.PROPERTY_REFERENCE
    )
    if TYPE_CHECKING:
        node_ptr: Optional[NodeReference] = None
        property_ptr: Optional[PropertyReference] = None
    run: Optional[PathRunSelector] = p_regular(35, require=False)

    def equals(
        self,
        other: "PathElement | Any",
        identity_map: Mapping[UUID, "NodeReference"] = EMPTY_DICT,
    ) -> bool:
        """Checks if the content of the two objects is equal (recursively)."""
        if other is None or self.metatype != getattr(other, "metatype", None):
            return False
        if other.type != self.type:
            return False
        if other.name != self.name:
            return False
        if (other.node_ptr.id if other.node_ptr else None) != (
            self.node_ptr.id if self.node_ptr else None
        ):
            return False
        if other.property_ptr != self.property_ptr:  # noqa: SIM103
            return False
        return True

    @_property
    def code_name(self) -> str | None:
        if self.name is None:
            return None
        else:
            return to_code_name(self.name)

    def __content_str__(self) -> str:
        if self.name:
            return f"{self.type.bench_name} {self.name}"
        else:
            return self.type.bench_name

    # absolute

    @staticmethod
    def root() -> "PathElement":
        return PathElement(type=PathElementType.ROOT)

    @staticmethod
    def bench(name: str) -> "PathElement":
        return PathElement(type=PathElementType.BENCH, name=name)

    @staticmethod
    def package(name: str) -> "PathElement":
        return PathElement(type=PathElementType.PACKAGE, name=name)

    @staticmethod
    def node_(node: Node) -> "PathElement":
        return PathElement(type=PathElementType.NODE, node=node)

    # relative

    @staticmethod
    def current() -> "PathElement":
        return PathElement(type=PathElementType.CURRENT)

    @staticmethod
    def container(name: str | None = None) -> "PathElement":
        return PathElement(type=PathElementType.CONTAINER, name=name)

    @staticmethod
    def unique(name: str | None = None) -> "PathElement":
        return PathElement(type=PathElementType.UNIQUE, name=name)

    @staticmethod
    def parent_() -> "PathElement":
        return PathElement(type=PathElementType.PARENT)

    @staticmethod
    def child(name: str | None = None) -> "PathElement":
        return PathElement(type=PathElementType.CHILD, name=name)

    # sub

    @staticmethod
    def attribute(key: "str | Property | Field") -> "PathElement":
        from bench.language import Field

        if isinstance(key, str):
            return PathElement(type=PathElementType.ATTRIBUTE, name=key)
        elif isinstance(key, Property):
            if type(key.value_packed_ptr) is Property:
                key = key.value_packed_ptr
            return PathElement(type=PathElementType.ATTRIBUTE, property=key)
        elif isinstance(key, Field):
            return PathElement(type=PathElementType.ATTRIBUTE, node=key)
        else:
            assert_never(key)

    # runtime

    @staticmethod
    def context() -> "PathElement":
        return PathElement(type=PathElementType.CONTEXT)

    @staticmethod
    def run_(selector: PathRunSelector = PathRunSelector.LATEST) -> "PathElement":
        return PathElement(type=PathElementType.RUN, run=selector)


@struct_(StructType.PATH)
class Path(Struct):
    """
    A human-readable Bench path to reference Nodes and Properties.
    Paths names and special operators are combined with slashes.
    Names may be the actual names or the code names of nodes.
    Fields/Properties are accessed with '.' separators.

    Segments:
        / -> root of this package
        . -> current node
        .. -> parent of current node
        ../.. -> parent of parent of current node

        Node -> ./Node -> node 'Node' relative to current node
        >Sibling -> sibling 'Sibling' of current node
        ~ -> closest container
        ~Node -> closest container with name 'Node'
        ^Name -> uniquely named node or child in closest container
        .attribute -> Field/Property of current node (or other value)

        Node1.name -> 'name' Property of 'Node1'
        Node2.field -> 'field' of 'Node2' (there must not be anything after .property)
        Node1/Node2/Node3 -> child 'Node3' of child 'Node2' of child 'Node1' of current node

        @bench -> absolute reference to bench 'bench' (uses main package)
        @bench:package -> absolute reference to package 'package' in bench 'bench'
        @bench/Node1/Node2/Node3 -> absolute reference to 'Node3' in current package of @bench

        $ -> context
        Node$> -> latest Run of 'Node'

    """

    elements: list[PathElement] = p_regular(
        31, require=True, array=True, struct=StructType.PATH_ELEMENT
    )

    def __content_str__(self) -> str:
        return self.render()

    def __len__(self) -> int:
        return len(self.elements)

    def __getitem__(self, index: int) -> PathElement:
        return self.elements[index]

    @property
    def is_absolute(self) -> bool:
        """Whether the path is absolute from a Bench root."""
        return len(self.elements) > 0 and self.elements[0].type == PathElementType.ROOT

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

    from_string = parse


PathElementIn = Union[
    "PathElement",
    Literal[
        PathElementType.ROOT,
        PathElementType.PARENT,
        PathElementType.CURRENT,
        PathElementType.CONTEXT,
        PathElementType.RUN,
    ],
    str,
    Property,
    "SourceNode",
]


def path_element(element_in: PathElementIn) -> PathElement:
    """Create a PathElement from a PathElement-like."""
    from bench.language import Field, Property

    if isinstance(element_in, PathElement):
        return element_in
    elif isinstance(element_in, str):
        return parse_path(element_in).elements[0]
    elif isinstance(element_in, (Property, Field)):
        return PathElement.attribute(element_in)
    elif isinstance(element_in, SourceNode):
        return PathElement.node_(element_in)
    elif isinstance(element_in, PathElementType):
        return PathElement(type=element_in)
    else:
        assert_never(element_in)


def path(*elements_in: PathElementIn) -> Path:
    """Create a Path from a list of PathElements or strings."""
    elements = [path_element(element_in) for element_in in elements_in]
    return Path(elements=elements)


PathIn = PathElementIn | Sequence[PathElementIn] | Path


def to_path(path_in: PathIn) -> Path:
    """Create a Path from a Path-like."""
    if isinstance(path_in, Path):
        return path_in
    elif isinstance(path_in, Sequence):
        return path(*path_in)
    else:
        return path(path_in)


def reverse_path_element(element: PathElement) -> PathElementIn:
    """Reverse a PathElement back into a PathElement-like (where possible)."""
    from bench.language import Field, Property, SourceNode

    if element.type == PathElementType.ATTRIBUTE:
        if isinstance(node := element.node, Field):
            return node
        elif isinstance(prop := element.property, Property):
            return prop
    elif element.type == PathElementType.NODE:
        if isinstance(node := element.node, SourceNode):
            return node
    elif element.type in (PathElementType.ROOT, PathElementType.CURRENT, PathElementType.PARENT):
        return element.type
    elif element.type == PathElementType.RUN and (
        element.run is None or element.run == PathRunSelector.LATEST
    ):
        return PathElementType.RUN
    return element


# see NAME_REGEX in validation
BENCH_PATTERN = regex.compile(rf"^@([{SLUG_REGEX_CHAR}]+)(?::([{SLUG_REGEX_CHAR}]+))?$")
NODE_PATTERN = regex.compile(rf"^([>\^~:])?([{NAME_REGEX_CHAR}\.]*)$")
CONTEXT_PATTERN: regex.Pattern[str] = regex.compile(
    rf"^\$((?:{'|'.join(regex.escape(sign) for sign in SIGN_BY_RUN_SELECTOR.values())}))?$"
)


@cached(LRUCache(maxsize=1024 * 10))
def parse_path(path: str) -> Path:
    """Parse a path string into a Path (for named Nodes/Properties)."""
    # plain root references
    if path == "/":
        return Path(elements=[PathElement(type=PathElementType.ROOT)])

    segments = path.strip().split("/")

    # empty path
    if not segments:
        return Path(elements=[])

    elements: list[PathElement] = []
    if segments[0] == "":
        # skip the root segment
        elements.append(PathElement(type=PathElementType.ROOT))
        segments.pop(0)
    for segment in segments:
        if not segment:
            raise PathSyntaxError(f"empty name in '{path}'")
        elif segment == ".":
            element = PathElement(type=PathElementType.CURRENT)
            elements.append(element)
        elif segment == "..":
            element = PathElement(type=PathElementType.PARENT)
            elements.append(element)
        else:
            # named nodes
            if match := BENCH_PATTERN.match(segment):
                if len(elements) > 0:
                    raise PathLogicError(f"bench reference must be the first segment in '{path}'")
                element = PathElement(type=PathElementType.BENCH, name=match.group(1))
                if len(match.group(1)) != len(segment) - 1 and not match.group(2):
                    raise PathSyntaxError(f"invalid bench name '{segment}' in '{path}'")
                elements.append(element)
                if package_name := match.group(2):
                    elements.append(PathElement(type=PathElementType.PACKAGE, name=package_name))
                continue
            elif match := CONTEXT_PATTERN.match(segment):
                if match.group(1):
                    # $> means RUN with LATEST selector
                    run_sign = match.group(1)
                    if run_sign not in RUN_SELECTOR_BY_SIGN:
                        raise PathSyntaxError(f"invalid run selector '{run_sign}' in '{path}'")
                    element = PathElement(
                        type=PathElementType.RUN, run=RUN_SELECTOR_BY_SIGN[run_sign]
                    )
                else:
                    # $ means CONTEXT
                    element = PathElement(type=PathElementType.CONTEXT)
                elements.append(element)
                continue
            elif match := NODE_PATTERN.match(segment):
                node_type = PathElementType.CHILD
                if match.group(1) == "~":
                    node_type = PathElementType.CONTAINER
                elif match.group(1) == "^":
                    node_type = PathElementType.UNIQUE
                name = match.group(2)
                if not name and node_type != PathElementType.CONTAINER:
                    raise PathSyntaxError(f"empty name in '{segment}' in '{path}'")
                element = PathElement(type=node_type, name=name or None)
            else:
                raise PathSyntaxError(f"invalid path: '{segment}' in '{path}'")
            elements.append(element)

            # get attribute (if any)
            if element.name and "." in element.name:
                parts = element.name.split(".")
                node_name = parts[0]
                attribute_names = parts[1:]
                if not node_name:
                    elements.pop()
                else:
                    element.name = node_name
                for attr_name in attribute_names:
                    if not attr_name:
                        raise PathSyntaxError(f"empty attribute name in '{segment}' in '{path}'")
                    property_element = PathElement(type=PathElementType.ATTRIBUTE, name=attr_name)
                    elements.append(property_element)
            elif "." in segment:
                raise PathSyntaxError(f"invalid property syntax: '{segment}' in '{path}'")

    return Path(elements=elements)


def render_path(path: Path) -> str:
    """Renders a path back into a string."""
    path_parts = []
    for i, element in enumerate(path.elements):
        # absolute
        if element.type == PathElementType.ROOT:
            path_parts.append("")
            if len(path.elements) == 1:
                return "/"
        elif element.type == PathElementType.BENCH:
            bench_part = f"@{element.code_name}"
            # combined with package if set
            if i + 1 < len(path.elements) and path.elements[i + 1].type == PathElementType.PACKAGE:
                bench_part += f":{path.elements[i + 1].code_name}"
            path_parts.append(bench_part)
        elif element.type == PathElementType.PACKAGE:
            continue  # handled above
        elif element.type == PathElementType.NODE:
            node = element.node
            assert node is not None, f"no node found for {element!r} in {path!r}"
            if "name" in node.__properties__:
                path_parts.append(node.absolute_path)
            else:
                path_parts.append(f"{node.metatype.bench_name}[id={node.id}]")
        # relative
        elif element.type == PathElementType.CURRENT:
            path_parts.append(".")
        elif element.type == PathElementType.CONTAINER:
            path_parts.append(f"~{element.code_name or ''}")
        elif element.type == PathElementType.UNIQUE:
            path_parts.append(f"^{element.code_name or ''}")
        elif element.type == PathElementType.PARENT:
            path_parts.append("..")
        elif element.type == PathElementType.CHILD:
            path_parts.append(element.code_name)
        # sub
        elif element.type == PathElementType.ATTRIBUTE:
            element_key = element.code_name
            if element_key is None:
                if (node := element.node) is not None:
                    element_key = node._path_key
                elif (prop := element.property) is not None:
                    if type(prop.value_runtime_ptr) is Property:
                        prop = prop.value_runtime_ptr
                    element_key = prop.name
                else:
                    element_key = "???"
            if path_parts:
                path_parts[-1] += f".{element_key}"
            else:
                path_parts.append(f".{element_key}")
        # runtime
        elif element.type == PathElementType.RUN:
            sign = f"${SIGN_BY_RUN_SELECTOR[element.run]}" if element.run is not None else "$"
            if path_parts:
                path_parts[-1] += sign
            else:
                path_parts.append(sign)
        elif element.type == PathElementType.CONTEXT:
            path_parts.append("$")
        else:
            assert_never(element.type)
    return "/".join(path_parts)


# NOTE :Performance: index some of the path lookups in the graph somehow?


def _get_child(scope: Node, name: str, node_type: NodeType | None = None) -> Node | None:
    """Finds a named child from a scope (if any)."""
    for child in scope._graph.iter_descendants(scope, node_type=node_type):
        if getattr(child, "name", None) == name or child.code_name == name:
            return child
    return None


def _get_descendant(scope: Node, name: str, node_type: NodeType | None = None) -> Node | None:
    """Finds any named descendant from a scope (if any, ignoring container boundaries)."""
    for descendant in scope._graph.iter_descendants(scope, recursive=True, node_type=node_type):
        if getattr(descendant, "name", None) == name or descendant.code_name == name:
            return descendant
    return None


def _get_contained_descendant(scope: Node, name: str) -> Node | None:
    """Finds a descendant that is directly contained by a scope (in block/page/action/pkg, if any)."""
    from bench.language import Action, Block, View

    if isinstance(scope, Block):
        # recurse child triggers/fields/queries
        for node_type in (NodeType.FIELD,):
            if node := _get_child(scope, name, node_type):
                return node
        # recurse descendant views/actions
        for node_type in (NodeType.VIEW, NodeType.ACTION, NodeType.PIPE):
            if node := _get_descendant(scope, name, node_type):
                return node
        # recurse down into blocks until we hit pages
        blocks = [scope]
        while blocks:
            block = blocks.pop()
            for child in block.blocks:
                if child.name == name or child.code_name == name:
                    return child
                if not child.is_page:
                    blocks.append(child)
    elif isinstance(scope, Action):
        # recurse own fields
        for node_type in (NodeType.FIELD,):
            if node := _get_child(scope, name, node_type):
                return node
        # recurse down into actions
        actions = [scope]
        while actions:
            action = actions.pop()
            for child in action.actions:
                if child.name == name or child.code_name == name:
                    return child
                actions.append(child)
    elif isinstance(scope, View):
        # recurse descendant views
        for node_type in (NodeType.VIEW,):
            if node := _get_descendant(scope, name, node_type):
                return node
    else:
        # just get children
        for node_type in (NodeType.SPACE, NodeType.BLOCK):
            if node := _get_child(scope, name, node_type):
                return node

    return None


def _get_container(scope: Node, name: str | None = None) -> Node | None:
    """Finds the next containing ancestor up from a scope (block/page/pkg, if any)."""

    if not isinstance(scope, SourceNode):
        # there is no container outside of source other than the bench/pkg
        if isinstance(scope, PackageNode) and (
            name is None or getattr(scope, "name", None) == name or scope.code_name == name
        ):
            return scope.package
    else:
        # if we're not in a block, find containing block or space (or skip to bench/pkg)
        parent = scope.parent
        while parent is not None:
            if parent.metatype in (NodeType.BLOCK, NodeType.SPACE, NodeType.PACKAGE) and (
                name is None or getattr(parent, "name", None) == name or parent.code_name == name
            ):
                return parent
            parent = parent.parent

    return None


def _get_unique(scope: Node, name: str) -> Node | None:
    """
    Finds a named node in any containing ancestor scope.
    The order of search is:
     1. 'Siblings' - descendents of parent container.
     2. Descendants - descendants of scope.
     3. Ancestors - descendants of ancestor containers (above parent).
    The rationale is that we generally want to refer to nodes in the same container
     more than we want our child nodes (like an output Field with the name of a ChoiceBlock,
     or other Actions in the same Flows more than our input Fields).
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


def _lower_scope(scope: Node) -> Node:
    """Lower Bench into its main Package."""
    if scope.metatype == NodeType.BENCH:
        package = cast("Bench", scope).main_package
        assert package is not None, f"bench {scope!r} has no main package"
        return package
    else:
        return scope


def _raise_scope(scope: Node) -> Node:
    """Raise a Package into its Bench."""
    if scope.metatype == NodeType.PACKAGE:
        bench = cast("Package", scope).bench
        assert bench is not None, f"package {scope!r} has no bench"
        return bench
    else:
        return scope


class PathOptions(NamedTuple):
    """Options for evaluating a path."""

    missing_is: Literal["none", "invalid"] = "none"


DEFAULT_EVALUATE_OPTIONS = PathOptions()


def evaluate_path(
    current: Node | Any,
    scope: Node,
    context: HasContext,
    path: str | Path | Sequence[PathElement],
    *,
    options: PathOptions = DEFAULT_EVALUATE_OPTIONS,
) -> Any | None:
    """Get the thing pointed to by a Path."""
    from bench.language import Bench, CustomObject, Field, Package

    # resolve to elements
    if isinstance(path, str):
        elements = parse_path(path).elements
    elif isinstance(path, Path):
        elements = path.elements
    else:
        elements = path
    if len(elements) == 0:
        return None

    # walk path
    current = scope
    for element in elements:
        # absolute
        if element.type == PathElementType.ROOT:
            if not isinstance(scope, (Package, PackageNode)):
                raise PathLogicError(f"root references are only valid for Bench Nodes: {path}")
            current = scope.package
        elif element.type == PathElementType.BENCH:
            if not isinstance(scope, BenchNode):
                raise PathLogicError(f"bench references are only valid for Bench Nodes: {path}")
            bench = scope.bench
            if bench is not None and element.name != bench.name:
                raise PathLogicError(f"references to other Benches are not supported: {path}")
            else:
                current = bench
        elif element.type == PathElementType.PACKAGE:
            assert element.name, f"missing name for {element!r} in {path!r}"
            if not isinstance(current, Bench):
                raise PathLogicError(f"package references are only valid for Benches: {path}")
            current = _get_child(current, element.name, NodeType.PACKAGE)
        elif element.type == PathElementType.NODE:
            node = element.node
            if node is None:
                if options.missing_is == "invalid":
                    raise PathLookupError(f"node at {path} not found in {scope!r}")
                else:
                    return None
            current = node

        # relative
        elif element.type == PathElementType.CURRENT:
            pass
        elif element.type == PathElementType.CONTAINER:
            if current.metatype == NodeType.PACKAGE:
                return None  # has no parent in path
            if not isinstance(current, Node):
                raise PathLogicError(f"cannot get container of {current!r} in {path!r}")
            current = _lower_scope(current)
            current = _get_container(current, element.name)
        elif element.type == PathElementType.UNIQUE:
            assert element.name, f"missing name for {element!r} in {path!r}"
            if not isinstance(current, Node):
                raise PathLogicError(f"cannot get unique of {current!r} in {path!r}")
            current = _lower_scope(current)
            current = _get_unique(current, element.name)
        elif element.type == PathElementType.PARENT:
            if current.metatype == NodeType.PACKAGE:
                return None  # has no parent in path
            if not isinstance(current, Node):
                raise PathLogicError(f"cannot get parent of {current!r} in {path!r}")
            current = current.parent
        elif element.type == PathElementType.CHILD:
            if not isinstance(current, Node):
                raise PathLogicError(f"cannot get child of {current!r} in {path!r}")
            current = _lower_scope(current)
            assert element.name, f"missing name for {element!r} in {path!r}"
            current = _get_child(current, element.name)

        # sub
        elif element.type == PathElementType.ATTRIBUTE:
            try:
                if element.name is not None:
                    current = getattr(current, element.name)
                elif (node := element.node) is not None:
                    assert isinstance(node, Field), f"expected Field, got {node!r}"
                    if isinstance(current, CustomObject):
                        current = current._do_get(node)
                    else:
                        if options.missing_is == "invalid":
                            raise PathLookupError(
                                f"cannot get {node!r} from {current!r} in {path!r}"
                            )
                        else:
                            return None
                elif (prop := element.property) is not None:
                    if type(prop.value_runtime_ptr) is Property:
                        prop = prop.value_runtime_ptr
                    current = getattr(current, prop.name)
                else:
                    return None
            except AttributeError as e:
                if options.missing_is == "invalid":
                    raise PathLookupError(
                        f"cannot get {element!r} from {current!r} in {path!r}"
                    ) from e
                else:
                    return None

        # runtime
        elif element.type == PathElementType.CONTEXT:
            current = context
        elif element.type == PathElementType.RUN:
            runtime = context.active_session.runtime
            run_selector = element.run or PathRunSelector.LATEST
            if not isinstance(current, Node):
                raise PathLogicError(f"cannot get run of {current!r} in {path!r}")
            if run_selector == PathRunSelector.LATEST:
                current = runtime.get_latest_run(cast("RunnableNode", current))
            else:
                assert_never(run_selector)
        else:
            assert_never(element.type)

        # bail if we can't find anything
        if current is None:
            return None

    return current


def get_node(scope: Node, context: HasContext, path: str | Path) -> Node | None:
    """
    Resolves a Node against the given scope.
    We try to be forgiving and just return None if we can't find the Node / the Path is weird.
    """
    if isinstance(path, str):
        path = parse_path(path)
    target = evaluate_path(scope, scope, context, path)
    if not isinstance(target, Node):
        return None

    # raise into bench if last element wasn't specifically package
    if path.elements and path.elements[-1].type != PathElementType.PACKAGE:
        target = _raise_scope(target)
    return target


def get_node_or_error(scope: Node, context: HasContext, path: str | Path) -> Node:
    """Resolves a Node against the given scope or raises an error."""
    node = get_node(scope, context, path)
    if node is None:
        raise PathLookupError(f"node at {path} not found in {scope!r}")
    return node


def _get_path_to_root(node: Node) -> list[Node]:
    """Gets the path relevant ancestors to a Node (including the Node, up to Package/Bench)"""
    supergraph = node._supergraph
    ancestors: list[Node] = [node]
    cur = node
    while cur.parent_ptr is not None and cur.metatype != NodeType.BENCH:
        next_cur = supergraph.get(cur.parent_ptr)
        assert (
            next_cur is not None
        ), f"no parent for {cur!r} in {supergraph!r} (parent_ptr={cur.parent_ptr})"
        ancestors.append(next_cur)
        cur = next_cur
    return ancestors


def _get_bench_path(node: "Bench | Package") -> Path:
    from .bench import Bench, Package

    if isinstance(node, Bench):
        return Path(elements=[PathElement(type=PathElementType.BENCH, name=node.slug)])
    elif isinstance(node, Package):
        if node.bench is not None:
            if node.bench.main_package_id == node.id:
                return Path(
                    elements=[PathElement(type=PathElementType.BENCH, name=node.bench.slug)]
                )
            else:
                return Path(
                    elements=[
                        PathElement(type=PathElementType.BENCH, name=node.bench.slug),
                        PathElement(type=PathElementType.PACKAGE, name=node.slug),
                    ]
                )
        else:
            raise PathLogicError(f"package {node!r} has no bench")
    else:
        assert_never(node)


def get_path(scope: Node, node: Node) -> Path:
    """Finds a path to the given node from a scope. The inverse of get_node."""
    from .bench import Bench, Package

    if scope == node:
        if isinstance(node, (Bench, Package)):
            return _get_bench_path(node)
        else:
            return Path(elements=[PathElement(type=PathElementType.CURRENT)])

    scope_path = _get_path_to_root(scope)
    node_path = _get_path_to_root(node)
    if scope_path[-1] != node_path[-1] or isinstance(node, (Bench, Package)):
        # make absolute path (different bench)
        if (
            not isinstance(node, BenchNode)
            or not node_path
            or not isinstance(node_path[-1], (Bench, Package))
        ):
            raise PathLogicError(f"no common ancestor found for {scope!r} and {node!r}")
        elements = _get_bench_path(node_path[-1]).elements
        for node_ancestor in node_path[1:]:
            name = getattr(node_ancestor, "name", None)
            if name is None:
                raise PathUnnamedNodeError(node_ancestor)
            elements.append(PathElement(type=PathElementType.CHILD, name=name))
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
        if isinstance(common_ancestor, (Bench, Package)):
            # absolute path from bench to node
            elements = _get_bench_path(common_ancestor).elements
            for node_ancestor in reversed(node_path):
                if not isinstance(node_ancestor, (Bench, Package)):
                    name = getattr(node_ancestor, "name", None)
                    if name is None:
                        raise PathUnnamedNodeError(node_ancestor)
                    elements.append(PathElement(type=PathElementType.CHILD, name=name))
        elif node_ancestor_idx == 0:
            # node is a direct ancestor of scope
            elements = []
            for i in range(1, scope_ancestor_idx + 1):
                name = getattr(scope_path[i], "name", None)
                if name is None:
                    raise PathUnnamedNodeError(scope_path[i])
                elements.append(PathElement(type=PathElementType.CONTAINER, name=name))
        else:
            # get from scope to common ancestor, then from common ancestor to node
            elements = []
            for i in range(1, scope_ancestor_idx):
                name = getattr(scope_path[i], "name", None)
                if name is None:
                    raise PathUnnamedNodeError(scope_path[i])
                elements.append(PathElement(type=PathElementType.CONTAINER, name=name))
            for i in range(node_ancestor_idx - 1, -1, -1):
                name = getattr(node_path[i], "name", None)
                if name is None:
                    raise PathUnnamedNodeError(node_path[i])
                if not elements and not _get_child(scope, name):
                    # refer to sibling as unique node
                    elements.append(PathElement(type=PathElementType.UNIQUE, name=name))
                else:
                    # must be a child if we already have other elements
                    elements.append(PathElement(type=PathElementType.CHILD, name=name))

    if node.metatype == NodeType.FIELD:
        elements[-1].type = PathElementType.ATTRIBUTE
    return Path(elements=elements)
