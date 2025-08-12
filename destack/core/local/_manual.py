"""Interactive manual browser for Destack builtin definitions."""

import dataclasses
from dataclasses import dataclass
from math import ceil
from operator import itemgetter
from typing import TYPE_CHECKING, Any, Literal, Optional, assert_never

from destack.registry import (
    BUILTIN_CLASS_BY_NAME,
    BUILTIN_DEFINITION_BY_NAME,
    ENUM_CLASS_BY_TYPE,
    HANDLE_CLASS_BY_TYPE,
    HANDLE_DEFINITION_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    NODE_DEFINITION_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
    STRUCT_DEFINITION_BY_TYPE,
)

from .._utils import get_superclasses
from . import _console
from ._console import Color
from ._parser import create_cli, create_repl

if TYPE_CHECKING:
    from destack import (
        ActionDefinition,
        ConstantDefinition,
        EnumDefinition,
        HandleDefinition,
        MethodDefinition,
        ModuleDefinition,
        NodeDefinition,
        ObjectSize,
        ObjectSizer,
        PropertyDefinition,
        StructDefinition,
        Type,
        Value,
    )

type_ = type
type _Definition = (
    "EnumDefinition | HandleDefinition | ModuleDefinition | NodeDefinition | StructDefinition"
)

cli = create_cli(
    "manual",
    aliases=["m", "man"],
    help="Interactive manual for the schema.",
)


@dataclass(slots=True)
class ManualContext:
    """State and display configuration for the manual."""

    # sizing
    memory_sizer: "ObjectSizer"
    kompakt_sizer: "ObjectSizer"
    flott_sizer: "ObjectSizer"
    sizers: dict[str, "ObjectSizer"]

    # navigation
    current: str | None = None
    history: list[str] = dataclasses.field(default_factory=list)

    # colors
    color_header: Color = "bright_cyan"
    color_module: Color = "bright_cyan"
    color_section_title: Color = "yellow"
    color_name: Color = "white"
    color_type: Color = "yellow"
    color_value: Color = "green"
    color_dim: Color = "dim"
    color_origin: Color = "cyan"
    color_alias: Color = "cyan"
    color_tree_highlight: Color = "bright_white"
    color_tree_default: Color = "white"


@cli.command()
def manual() -> None:
    """Interactive manual for the schema."""
    from destack import (
        VERSION,
        FlottObjectSizer,
        KompaktObjectSizer,
        PythonObjectSizer,
        RustObjectSizer,
        TypeScriptObjectSizer,
    )

    # create the REPL
    repl = create_repl(
        prompt_text="destack> ",
        welcome=f"""\
{"=" * 70}
The Destack Manual
Version: {VERSION}
{"=" * 70}
Type 'help' for available commands.""",
    )

    # current context for navigation
    context = ManualContext(
        memory_sizer=RustObjectSizer(),
        kompakt_sizer=KompaktObjectSizer(),
        flott_sizer=FlottObjectSizer(),
        sizers={
            "Memory": RustObjectSizer(),
            "Kompakt": KompaktObjectSizer(),
            "Flott": FlottObjectSizer(),
            "Rust": RustObjectSizer(),
            "JavaScript": TypeScriptObjectSizer(),
            "Python": PythonObjectSizer(),
        },
    )

    # register commands
    _register_commands(repl, context)

    # run the REPL
    repl.run()


def _register_commands(repl: Any, context: ManualContext) -> None:
    """Register all REPL commands."""

    # commands: show
    @repl.command("show", "Show detailed information about a definition")
    def show_definition(*args: str) -> None:
        """Show detailed definition information."""
        if not args:
            _console.error("Please provide a definition name.")
            _console.print("Use 'list' to see available definitions.", "dim")
            return

        name = args[0]
        definitions = _find_definition(name)
        if not definitions:
            _console.error(f"Definition '{name}' not found.")
            return
        definition = definitions[0]

        # update context
        if context.current and context.current != name:
            context.history.append(context.current)
        context.current = name

        # display the definition inside a pager
        with _console.capture_output() as _buf:
            _show_definition(definition, context)
        _console.page("".join(_buf))

    # commands: tree
    @repl.command(
        "tree",
        "Show inheritance tree. Usage: tree <name> [--direction=up|down|both|-d up] [--full|-f]",
    )
    def show_tree(*args: str) -> None:
        """Show inheritance tree for an Object (Node/Struct/Handle)."""
        if not args:
            _console.error("Please provide a definition name.")
            return
        name = args[0]
        # defaults
        direction: Literal["up", "down", "both"] = "both"
        full: bool = True  # go down all the way by default

        # parse options from remaining args
        for i, raw in enumerate(args[1:]):
            if raw in ("--full", "-f"):
                full = True
            elif raw.startswith("--direction="):
                _, val = raw.split("=", 1)
                if val in ("up", "down", "both"):
                    direction = val  # type: ignore[assignment]
            elif raw in ("--direction", "-d"):
                if i + 2 <= len(args) and args[i + 2] in ("up", "down", "both"):
                    direction = args[i + 2]  # type: ignore[assignment]

        definitions = _find_definition(name)
        if not definitions:
            _console.error(f"Definition '{name}' not found.")
            return
        definition = definitions[0]
        # if we go down, default to full subtree
        include_subclasses = full or direction in ("down", "both")
        tree = _render_inheritance_tree(
            definition,
            context=context,
            direction=direction,
            include_subclasses=include_subclasses,
        )
        if tree is None:
            _console.warn("Only Objects (Node/Struct/Handle) have inheritance.")
            return
        _console.section("Inheritance Tree", tree)

    # default handler for direct definition names
    # commands: default passthrough
    @repl.command("default")
    def handle_default(user_input: str) -> None:
        """Handle direct definition names."""
        # remove any quotes
        name = user_input.strip().strip("'\"")
        # try to show the definition
        show_definition(name)


def _find_definition(name: str, n: int = 5) -> list[_Definition]:
    """Find matching definition names using substring matching with preference for literal matches."""
    suggestions: list[tuple[int, str, _Definition]] = []

    for def_name, definition in BUILTIN_DEFINITION_BY_NAME.items():
        # calculate score based on substring matching
        score = _calculate_substring_score(name, def_name)
        suggestions.append((score, def_name, definition))

    # sort by score (lower is better) and return top suggestions
    suggestions.sort(key=itemgetter(0, 1))
    return [definition for score, _, definition in suggestions[:n] if score < 1000]


def _calculate_substring_score(query: str, target: str) -> int:
    """Calculate substring matching score, preferring literal substrings."""
    # exact match gets best score
    if query == target:
        return 0
    # case-insensitive exact match
    if query.lower() == target.lower():
        return 1
    # literal substring match (case sensitive)
    if query in target:
        return 10 + len(target) - len(query)
    # case-insensitive substring match
    if query.lower() in target.lower():
        return 20 + len(target) - len(query)
    # prefix match (case sensitive)
    if target.startswith(query):
        return 30 + len(target) - len(query)
    # case-insensitive prefix match
    if target.lower().startswith(query.lower()):
        return 40 + len(target) - len(query)
    # no match - use length difference as penalty
    return 1000 + abs(len(target) - len(query))


def _render_size(size: "ObjectSize", show_instances_per_bytes: int | None = None) -> str:
    """Render a size to a string."""
    if size.max_size is None:
        base = f"{size.min_size}.."
    elif size.min_size == size.max_size:
        base = f"{size.min_size}"
    else:
        base = f"{size.min_size}..{size.max_size}"

    # calculate how many instances fit in show_size_per_bytes
    if show_instances_per_bytes is not None:
        count_max = show_instances_per_bytes // size.min_size
        if size.max_size is None:
            count_str = f"<{count_max}"
        elif size.min_size == size.max_size:
            count_str = f"={count_max}"
        else:
            count_min = show_instances_per_bytes // size.max_size
            count_str = f"{count_min}..{count_max}"

        # format bytes with appropriate units
        if show_instances_per_bytes >= (1024 * 1024):
            bytes_str = f"{show_instances_per_bytes / (1024 * 1024):.2f}MB"
        elif show_instances_per_bytes >= 1024:
            bytes_str = f"{show_instances_per_bytes / 1024:.2f}KB"
        else:
            bytes_str = f"{show_instances_per_bytes}B"

        base = f"{base} ({count_str} per {bytes_str})"

    return base


def _render_size_value(size: "ObjectSize") -> str:
    """Render only the size part as a string (no per-bytes info)."""
    if size.max_size is None:
        return f"{size.min_size}.."
    if size.min_size == size.max_size:
        return f"{size.min_size}"
    return f"{size.min_size}..{size.max_size}"


def _render_size_instances_per(size: "ObjectSize", per_bytes: int) -> str:
    """Render only the instances-per-bytes part as a string."""
    if size.min_size == 0:
        return "∞"
    count_max = per_bytes // size.min_size
    if size.max_size is None:
        return f"<{count_max}"
    if size.min_size == size.max_size:
        return f"={count_max}"
    count_min = per_bytes // size.max_size
    return f"{count_min}..{count_max}"


def _render_type(type: "Type") -> str:
    """Render a type to a string."""
    from destack.core import TypeCardinality

    # scalar
    if type.cardinality == TypeCardinality.SCALAR:
        inner_str = _render_type_scalar(type)
    # list
    elif type.cardinality == TypeCardinality.LIST:
        assert type.value_type is not None
        inner_str = f"list[{_render_type(type.value_type)}]"
    # tuple
    elif type.cardinality == TypeCardinality.TUPLE:
        assert type.element_types is not None
        element_names = [_render_type(t) for t in type.element_types]
        inner_str = f"tuple[{', '.join(element_names)}]"
    # map
    elif type.cardinality == TypeCardinality.MAP:
        assert type.key_type is not None
        assert type.value_type is not None
        key_name = _render_type(type.key_type)
        value_name = _render_type(type.value_type)
        inner_str = f"map[{key_name}, {value_name}]"
    else:
        assert_never(type.cardinality)

    # wrap in optional
    if not type.is_required:
        inner_str = f"{inner_str}?"

    return inner_str


def _render_type_scalar(type: "Type") -> str:
    """Render a scalar type to a string."""
    from destack.core import PRIMITIVE_PY_ANNOTATION_BY_TYPE, ScalarType, TypeCardinality

    assert type.cardinality == TypeCardinality.SCALAR
    assert type.scalar_type is not None, f"no scalar_type for {type!r}"

    # primitive
    if type.scalar_type == ScalarType.PRIMITIVE:
        assert type.primitive_type is not None, f"no primitive_type for {type!r}"
        return PRIMITIVE_PY_ANNOTATION_BY_TYPE[type.primitive_type].__name__
    # enum
    elif type.scalar_type == ScalarType.ENUM:
        assert type.enum_type is not None, f"no enum_type for {type!r}"
        return ENUM_CLASS_BY_TYPE[type.enum_type].__name__
    # node value
    elif type.scalar_type == ScalarType.NODE:
        if type.node_types is None:
            return "Node"
        elif len(type.node_types) == 1:
            return NODE_CLASS_BY_TYPE[type.node_types[0]].__name__
        else:
            node_names = [NODE_CLASS_BY_TYPE[t].__name__ for t in type.node_types]
            return " | ".join(node_names)
    # node reference
    elif type.scalar_type in (
        ScalarType.NODE_RAW,
        ScalarType.NODE_IDENTITY,
        ScalarType.NODE_SPATIAL,
        ScalarType.NODE_TEMPORAL,
    ):
        if type.node_types is None:
            node_str = "Node"
        elif len(type.node_types) == 1:
            node_str = NODE_CLASS_BY_TYPE[type.node_types[0]].__name__
        else:
            node_names = [NODE_CLASS_BY_TYPE[t].__name__ for t in type.node_types]
            node_str = " | ".join(node_names)
        reference_type_name = type.scalar_type.name.split("_")[-1][0]
        return f"->{node_str}[{reference_type_name}]"
    # struct
    elif type.scalar_type == ScalarType.STRUCT:
        assert type.struct_type is not None, f"no struct_type for {type!r}"
        return STRUCT_CLASS_BY_TYPE[type.struct_type].__name__
    # handle
    elif type.scalar_type == ScalarType.HANDLE:
        assert type.handle_type is not None, f"no handle_type for {type!r}"
        return HANDLE_CLASS_BY_TYPE[type.handle_type].__name__
    # union
    elif type.scalar_type == ScalarType.UNION:
        assert type.element_types is not None, f"no element_types for {type!r}"
        element_names = [_render_type_scalar(t) for t in type.element_types]
        return " | ".join(element_names)
    #
    else:
        assert_never(type.scalar_type)


def _render_value(value: "Value") -> str:
    """Render a value to a string."""
    return repr(value.value)


def _show_definition(
    definition: "EnumDefinition | HandleDefinition | ModuleDefinition | NodeDefinition | StructDefinition",
    context: ManualContext,
) -> None:
    """Display a definition with full details."""
    from ..definition import (
        EnumDefinition,
        HandleDefinition,
        ModuleDefinition,
        NodeDefinition,
        StructDefinition,
    )

    definition_cls = BUILTIN_CLASS_BY_NAME[definition.name]

    # header
    # header block
    _console.print("")
    _console.print("=" * 70, context.color_header)
    _console.print(
        f"{definition.name} [{definition.__class__.__name__}]",
        context.color_header,
        "bold",
    )
    _console.print(f"{definition_cls.__module__}", context.color_module)
    _console.print("=" * 70, context.color_header)

    # description
    if definition.description:
        _console.print("\n" + _console.color("Description:", context.color_section_title, "bold"))
        desc_lines = definition.description.strip().split("\n")
        for line in desc_lines:
            _console.print("  " + line)

    # type-specific information
    if isinstance(definition, NodeDefinition):
        _show_node(definition, context)
    elif isinstance(definition, StructDefinition):
        _show_struct(definition, context)
    elif isinstance(definition, EnumDefinition):
        _show_enum(definition, context)
    elif isinstance(definition, HandleDefinition):
        _show_handle(definition, context)
    elif isinstance(definition, ModuleDefinition):
        _show_module(definition, context)
    else:
        assert_never(definition)

    _console.print("\n" + "=" * 70, context.color_header)


def _show_properties(
    owner: "EnumDefinition | HandleDefinition | ModuleDefinition | NodeDefinition | StructDefinition",
    properties: list["PropertyDefinition"],
    title: str,
    context: ManualContext,
) -> None:
    """Display properties section."""
    from destack.core import Node, Struct

    def _resolve_tag_name(owner_local: Any, tag_id: Optional[int]) -> Optional[str]:
        """Resolve a tag id to its name for the given owner definition."""
        if tag_id is None:
            return None
        # try from definition's own tags first
        try:
            for tag in owner_local.tags:  # type: ignore[attr-defined]
                if tag.id == tag_id:
                    return tag.name
        except Exception:
            pass
        # fallback to builtin class declaration tags (walk mro)
        object_cls = BUILTIN_CLASS_BY_NAME.get(owner_local.name)
        if object_cls is None:
            return None
        for cls in object_cls.__mro__:
            if issubclass(cls, Node) or issubclass(cls, Struct):
                for tag in cls.__declaration__.tags:
                    if tag.id == tag_id:
                        return tag.name
        return None

    # properties
    instance_properties = [p for p in properties if not p.is_static and not p.is_runtime_only]
    _console.print(
        "\n"
        + _console.color(
            f"{title} ({len(instance_properties)}):", context.color_section_title, "bold"
        )
    )
    headers = [
        "ID",
        "Name",
        "Type",
        "Memory",
        "Kompakt",
        "Flott",
        "Tag",
        "Defined In",
        "Flags",
    ]

    rows: list[dict[str, str]] = []
    for prop in instance_properties:
        flags: list[str] = []
        if prop.is_readonly:
            flags.append("ro")
        if prop.is_managed:
            flags.append("ma")
        if prop.is_interned:
            flags.append("in")
        if prop.is_repr:
            flags.append("re")
        if prop.is_eq:
            flags.append("eq")
        if prop.is_hash:
            flags.append("ha")
        origin = _get_origin_for(owner, prop)
        tag_name = _resolve_tag_name(owner, prop.tag)
        rows.append(
            {
                "ID": _console.color(str(prop.id), context.color_dim),
                "Name": _console.color(prop.name, context.color_name),
                "Tag": _console.color(tag_name, context.color_origin) if tag_name else "",
                "Type": _console.color(_render_type(prop.type), context.color_type),
                "Memory": _console.color(
                    _render_size(context.memory_sizer.size_property(prop)), context.color_value
                ),
                "Kompakt": _console.color(
                    _render_size(context.kompakt_sizer.size_property(prop)), context.color_value
                ),
                "Flott": _console.color(
                    _render_size(context.flott_sizer.size_property(prop)), context.color_value
                ),
                "Defined In": _console.color(origin or "-", context.color_origin),
                "Flags": _console.color("|".join(flags) if flags else "", "gray"),
            }
        )

    _console.print(
        _console.table(
            rows,
            headers,
            separators_on_change=["Defined In", "Tag"],
        )
    )


def _show_methods(
    owner: "EnumDefinition | HandleDefinition | ModuleDefinition | NodeDefinition | StructDefinition",
    methods: list["MethodDefinition"],
    title: str,
    context: ManualContext,
) -> None:
    """Display methods section formatted as a table."""
    # methods
    _console.print(
        "\n" + _console.color(f"{title} ({len(methods)}):", context.color_section_title, "bold")
    )
    if not methods:
        return
    headers = ["ID", "Name", "Inputs", "Returns", "Defined In", "Alias"]
    rows: list[dict[str, str]] = []
    for method in methods:
        # find the aliased method by id
        if method.alias_of is not None:
            aliased_method = next((m for m in methods if m.id == method.alias_of), None)
            assert aliased_method is not None, f"aliased method #{method.alias_of} not found"
            rows.append(
                {
                    "ID": _console.color(str(method.id), context.color_dim),
                    "Name": _console.color(method.name, context.color_name),
                    "Inputs": _console.color("-", "gray"),
                    "Returns": _console.color("-", "gray"),
                    "Defined In": _console.color(
                        _get_origin_for(owner, method) or "-", context.color_origin
                    ),
                    "Alias": _console.color(aliased_method.name, context.color_alias),
                }
            )
            continue

        # signature: one input per line
        input_signature_parts: list[str] = []
        for input_property in method.input_properties:
            part = f"{_console.color(input_property.name, context.color_name)}: {_console.color(_render_type(input_property.type), context.color_type)}"
            if input_property.default_value is not None:
                part = f"{part} = {_render_value(input_property.default_value)}"
            input_signature_parts.append(part)
        inputs = (
            "\n".join(input_signature_parts)
            if input_signature_parts
            else _console.color("-", "gray")
        )
        returns = (
            _console.color(_render_type(method.output_property.type), context.color_type)
            if method.output_property is not None
            else _console.color("None", "gray")
        )

        rows.append(
            {
                "ID": _console.color(str(method.id), context.color_dim),
                "Name": _console.color(method.name, context.color_name),
                "Inputs": inputs,
                "Returns": returns,
                "Defined In": _console.color(
                    _get_origin_for(owner, method) or "-", context.color_origin
                ),
                "Alias": _console.color("-", "gray"),
            }
        )
    _console.print(_console.table(rows, headers))


def _show_actions(
    owner: "EnumDefinition | HandleDefinition | ModuleDefinition | NodeDefinition | StructDefinition",
    actions: list["ActionDefinition"],
    title: str,
    context: ManualContext,
) -> None:
    """Display actions as a table with type and message IO."""
    from .. import ActionType, StringCasing, to_casing

    # actions
    _console.print(
        "\n" + _console.color(f"{title} ({len(actions)}):", context.color_section_title, "bold")
    )
    if not actions:
        return
    headers = ["ID", "Name", "Type", "Input", "Output", "Defined In"]
    rows: list[dict[str, str]] = []
    for action in actions:
        # action type label
        type_label = action.type.name if isinstance(action.type, ActionType) else str(action.type)

        def fmt_msg(msg_type: Any) -> str:
            if msg_type is None:
                return _console.color("None", "gray")
            return _console.color(
                to_casing(msg_type.name, StringCasing.UPPER_CAMEL), context.color_type
            )

        input_label = fmt_msg(action.input_message_type)
        output_label = fmt_msg(action.output_message_type)

        if action.type in (ActionType.STREAM_IN_UNARY_OUT, ActionType.STREAM_IN_STREAM_OUT):
            input_label = f"Stream[{input_label}]"
        if action.type in (ActionType.UNARY_IN_STREAM_OUT, ActionType.STREAM_IN_STREAM_OUT):
            output_label = f"Stream[{output_label}]"

        rows.append(
            {
                "ID": _console.color(str(action.id), context.color_dim),
                "Name": _console.color(action.name, context.color_name),
                "Type": _console.color(type_label, context.color_origin),
                "Input": input_label,
                "Output": output_label,
                "Defined In": _console.color(
                    _get_origin_for(owner, action) or "-", context.color_origin
                ),
            }
        )

    _console.print(
        _console.table(rows, headers, separators_on_change=["Defined In"]),
    )


def _show_constants(
    constants: list["ConstantDefinition"],
    title: str,
    context: ManualContext,
) -> None:
    """Display constants section."""
    # constants
    _console.print("\n" + _console.color(f"{title}:", context.color_section_title))
    if not constants:
        return
    headers = ["ID", "Name", "Value"]
    rows: list[dict[str, str]] = []
    for constant in constants:
        rows.append(
            {
                "ID": _console.color(str(constant.id), context.color_dim),
                "Name": _console.color(constant.name, context.color_name),
                "Value": str(_render_value(constant.value)),
            }
        )
    _console.print(_console.table(rows, headers))


def _show_layout(definition: "StructDefinition | NodeDefinition", context: ManualContext) -> None:
    """Display layout/size information."""

    # specific sizers
    _console.print("\n" + _console.color("Layout:", context.color_section_title, "bold"))
    headers = [
        "Layout",
        "Size",
        "/1MB",
        "1k Disk R",
        "1k Disk W",
        "1k Net R",
        "1k Net W",
        "Lines",
    ]
    secondary_headers = [
        "memory",
        "unaligned",
        "contiguous",
        "~3 GB/s",
        "~2 GB/s",
        "100 Mb/s",
        "20 Mb/s",
        "64B each",
    ]
    rows: list[dict[str, str]] = []
    for name, sizer in context.sizers.items():
        size = sizer.size_object(definition)
        size_str = _render_size_value(size)
        # per counts
        per_str_1mb = _render_size_instances_per(size, 1024 * 1024)
        # performance estimates
        # assume median size when a range; use min_size for conservative throughput
        approx_size_bytes = (
            size.min_size if size.max_size is None else (size.min_size + size.max_size) // 2
        )
        # disk: NVMe SSD approximate throughput ~ 2 GB/s (read) and ~ 1 GB/s (write)
        disk_read_bps = 2 * 1024 * 1024 * 1024
        disk_write_bps = 1 * 1024 * 1024 * 1024
        disk_r = _console.humanize_duration((approx_size_bytes * 1000) / disk_read_bps)
        disk_w = _console.humanize_duration((approx_size_bytes * 1000) / disk_write_bps)
        # network: 100 Mb/s down, 20 Mb/s up (megabits)
        net_r_bps = 100 * 1_000_000 / 8  # to bytes/s
        net_w_bps = 20 * 1_000_000 / 8  # to bytes/s
        net_r = _console.humanize_duration((approx_size_bytes * 1000) / net_r_bps)
        net_w = _console.humanize_duration((approx_size_bytes * 1000) / net_w_bps)
        # cache lines: size / 64B rounded up
        cache_lines = ceil(approx_size_bytes / 64) if approx_size_bytes > 0 else 0
        is_wired = name in ("Memory", "Kompakt", "Flott")
        rows.append(
            {
                "Layout": _console.color(name, context.color_origin),
                "Size": _console.color(
                    _console.humanize_bytes(approx_size_bytes)
                    if size.min_size == size.max_size
                    else size_str,
                    context.color_value,
                ),
                "/1MB": _console.color(
                    _console.humanize_count_text(per_str_1mb), context.color_value
                ),
                "1k Disk R": _console.color(
                    disk_r, context.color_value if is_wired else context.color_dim
                ),
                "1k Disk W": _console.color(
                    disk_w, context.color_value if is_wired else context.color_dim
                ),
                "1k Net R": _console.color(
                    net_r, context.color_value if is_wired else context.color_dim
                ),
                "1k Net W": _console.color(
                    net_w, context.color_value if is_wired else context.color_dim
                ),
                "Lines": _console.color(
                    _console.humanize_count(cache_lines),
                    context.color_value if is_wired else context.color_dim,
                ),
            }
        )
    _console.print(_console.table(rows, headers, secondary_headers=secondary_headers))


def _show_node(definition: "NodeDefinition", context: ManualContext) -> None:
    """Display Node-specific details."""
    from ..builtin import StringCasing, to_casing

    # metadata
    _console.print("\n" + _console.color("Metadata:", context.color_section_title, "bold"))
    meta_dict: dict[str, object] = {
        "Inherits": " -> ".join(
            [
                _console.color(to_casing(base.name, StringCasing.UPPER_CAMEL), context.color_value)
                for base in definition.inherits
            ]
            or "-"
        ),
        "Stability": _console.color(
            to_casing(definition.stability.name, StringCasing.UPPER_CAMEL), context.color_value
        ),
        "Abstract": _console.color("Yes" if definition.is_abstract else "No", context.color_value),
        "Final": _console.color("Yes" if definition.is_final else "No", context.color_value),
        "Singleton": _console.color(
            "Yes" if definition.is_singleton else "No", context.color_value
        ),
    }
    _console.print(_console.kv(meta_dict))

    # layout
    _show_layout(definition, context)

    # inheritance tree (object-only)
    tree = _render_inheritance_tree(
        definition, context=context, direction="both", include_subclasses=False
    )
    if tree:
        _console.print("")
        _console.print("\n" + _console.color("Inheritance:", context.color_section_title, "bold"))
        _console.print(tree)

    if definition.properties:
        _show_properties(definition, definition.properties, "Properties", context)
    if definition.methods:
        _show_methods(definition, definition.methods, "Methods", context)
    if definition.actions:
        _show_actions(definition, definition.actions, "Actions", context)
    if definition.constants:
        _show_constants(definition.constants, "Constants", context)


def _show_struct(definition: "StructDefinition", context: ManualContext) -> None:
    """Display Struct-specific details."""
    from ..builtin import StringCasing, to_casing

    # metadata
    _console.print("\n" + _console.color("Metadata:", context.color_section_title, "bold"))
    meta_dict: dict[str, object] = {
        "Inherits": " -> ".join(
            [
                _console.color(to_casing(base.name, StringCasing.UPPER_CAMEL), context.color_value)
                for base in definition.inherits
            ]
            or "-"
        ),
        "Stability": _console.color(
            to_casing(definition.stability.name, StringCasing.UPPER_CAMEL), context.color_value
        ),
        "Abstract": _console.color("Yes" if definition.is_abstract else "No", context.color_value),
    }
    _console.print(_console.kv(meta_dict))

    # layout
    _show_layout(definition, context)

    # inheritance tree (object-only)
    tree = _render_inheritance_tree(
        definition, context=context, direction="both", include_subclasses=False
    )
    if tree:
        _console.print("")
        _console.print("\n" + _console.color("Inheritance:", context.color_section_title, "bold"))
        _console.print(tree)

    if definition.properties:
        _show_properties(definition, definition.properties, "Properties", context)
    if definition.methods:
        _show_methods(definition, definition.methods, "Methods", context)


def _show_enum(definition: "EnumDefinition", context: ManualContext) -> None:
    """Display Enum-specific details."""
    # options
    if definition.options:
        _console.print(
            "\n"
            + _console.color(
                f"Options ({len(definition.options)}):", context.color_section_title, "bold"
            )
        )
        for option in definition.options:
            opt_str = f"  {_console.color(option.name, context.color_value)} = {option.id}"
            if option.description:
                opt_str = f"  # {_console.color(option.description, 'dim')}\n{opt_str}"
            _console.print(opt_str)


def _show_handle(definition: "HandleDefinition", context: ManualContext) -> None:
    """Display Handle-specific details."""
    if definition.properties:
        _show_properties(definition, definition.properties, "Properties", context)
    if definition.methods:
        _show_methods(definition, definition.methods, "Methods", context)
    if definition.constants:
        _show_constants(definition.constants, "Constants", context)


def _render_inheritance_tree(
    definition: "EnumDefinition | HandleDefinition | ModuleDefinition | NodeDefinition | StructDefinition",
    *,
    direction: Literal["up", "down", "both"] = "both",
    include_subclasses: bool,
    context: ManualContext,
) -> str | None:
    """Build and render the inheritance tree for an Object definition.

    Returns None for non-Object (e.g., Module/Enum).
    """
    from ..definition import (
        EnumDefinition,
        HandleDefinition,
        ModuleDefinition,
        NodeDefinition,
        StructDefinition,
    )

    # only Objects have inheritance trees
    if isinstance(definition, (ModuleDefinition, EnumDefinition)):
        return None

    # resolve per-kind helpers
    def _get_base(defn: NodeDefinition | StructDefinition | HandleDefinition):
        if isinstance(defn, NodeDefinition):
            base_type = defn.base_type
            return NODE_DEFINITION_BY_TYPE.get(base_type) if base_type is not None else None
        if isinstance(defn, StructDefinition):
            base_type = defn.base_type
            return STRUCT_DEFINITION_BY_TYPE.get(base_type) if base_type is not None else None
        base_type = defn.base_type
        return HANDLE_DEFINITION_BY_TYPE.get(base_type) if base_type is not None else None

    def _get_children(defn: NodeDefinition | StructDefinition | HandleDefinition) -> list[Any]:
        if isinstance(defn, NodeDefinition):
            children_types = defn.extended_by
            return [NODE_DEFINITION_BY_TYPE[t] for t in children_types]
        if isinstance(defn, StructDefinition):
            children_types = defn.extended_by
            return [STRUCT_DEFINITION_BY_TYPE[t] for t in children_types]
        children_types = defn.extended_by
        return [HANDLE_DEFINITION_BY_TYPE[t] for t in children_types]

    # collect ancestors chain up to root
    ancestors: list[Any] = []
    cur = definition
    while True:
        parent = _get_base(cur)  # type: ignore[arg-type]
        if parent is None:
            break
        ancestors.append(parent)
        cur = parent

    # build label map
    def _add_label(defn: Any) -> str:
        # compute descendant count for this node
        def count_desc(d: Any) -> int:
            return len(_get_all_descendants(d))

        num = count_desc(defn)
        suffix = f" ({num})" if num > 0 else ""

        # add per-level size deltas for Structs/Nodes
        inc_str = ""
        if isinstance(defn, (NodeDefinition, StructDefinition)):
            parent = _get_base(defn)
            mem_def = context.memory_sizer.size_object(defn)
            if parent is not None and isinstance(parent, (NodeDefinition, StructDefinition)):
                mem_par = context.memory_sizer.size_object(parent)
                mem_inc = max(0, mem_def.min_size - mem_par.min_size)
                inc_str = f" [+{mem_inc}B]"
            else:
                inc_str = f" [{mem_def.min_size}B]"

        # suffixes should be dim gray, even when the label itself is highlighted
        suffix_col = _console.color(suffix, "dim") if suffix else ""
        inc_col = _console.color(inc_str, "gray", "dim") if inc_str else ""
        return f"{defn.name}{suffix_col}{inc_col}"

    label_by_id: dict[str, str] = {}
    children_by_id: dict[str, list[str]] = {}

    target_id = definition.name
    label_by_id[target_id] = _add_label(definition)

    def _add_edge(parent: Any, child: Any) -> None:
        pid = parent.name
        cid = child.name
        label_by_id.setdefault(pid, _add_label(parent))
        label_by_id.setdefault(cid, _add_label(child))
        children_by_id.setdefault(pid, []).append(cid)

    # choose root and assemble according to direction
    if direction in ("up", "both"):
        # stitch the ancestor chain: root -> ... -> target
        chain = [*list(reversed(ancestors)), definition]
        for i in range(len(chain) - 1):
            _add_edge(chain[i], chain[i + 1])
        root_id = chain[0].name if chain else definition.name
    else:
        root_id = definition.name

    # include descendants from the target (only direct unless include_subclasses=True)
    if direction in ("down", "both"):

        def _walk_desc(defn: Any):
            children = _get_children(defn)
            for child in children:
                _add_edge(defn, child)
                if include_subclasses:
                    _walk_desc(child)

        _walk_desc(definition)

    return _console.render_tree(root_id, children_by_id, label_by_id, highlight_id=target_id)


def _get_all_descendants(
    definition: "EnumDefinition | HandleDefinition | ModuleDefinition | NodeDefinition | StructDefinition",
) -> list[
    "EnumDefinition | HandleDefinition | ModuleDefinition | NodeDefinition | StructDefinition"
]:
    """Get all descendants of a definition."""
    from ..definition import (
        EnumDefinition,
        ModuleDefinition,
        NodeDefinition,
        StructDefinition,
    )

    if isinstance(definition, (ModuleDefinition, EnumDefinition)):
        return []
    if isinstance(definition, NodeDefinition):
        direct = [NODE_DEFINITION_BY_TYPE[t] for t in definition.extended_by]
    elif isinstance(definition, StructDefinition):
        direct = [STRUCT_DEFINITION_BY_TYPE[t] for t in definition.extended_by]
    else:
        direct = [HANDLE_DEFINITION_BY_TYPE[t] for t in definition.extended_by]
    all_desc: list[
        EnumDefinition | HandleDefinition | ModuleDefinition | NodeDefinition | StructDefinition
    ] = []
    for child in direct:
        all_desc.append(child)
        all_desc.extend(_get_all_descendants(child))
    return all_desc


def _get_base_definition(
    definition: "EnumDefinition | HandleDefinition | ModuleDefinition | NodeDefinition | StructDefinition",
):
    """Get the base definition of a definition."""
    from ..definition import (
        EnumDefinition,
        ModuleDefinition,
        NodeDefinition,
        StructDefinition,
    )

    if isinstance(definition, (ModuleDefinition, EnumDefinition)):
        return None
    if isinstance(definition, NodeDefinition):
        base_type = definition.base_type
        return NODE_DEFINITION_BY_TYPE.get(base_type) if base_type is not None else None
    if isinstance(definition, StructDefinition):
        base_type = definition.base_type
        return STRUCT_DEFINITION_BY_TYPE.get(base_type) if base_type is not None else None
    base_type = definition.base_type
    return HANDLE_DEFINITION_BY_TYPE.get(base_type) if base_type is not None else None


def _get_children_definitions(
    definition: "EnumDefinition | HandleDefinition | ModuleDefinition | NodeDefinition | StructDefinition",
) -> list[Any]:
    from ..definition import (
        EnumDefinition,
        ModuleDefinition,
        NodeDefinition,
        StructDefinition,
    )

    if isinstance(definition, (ModuleDefinition, EnumDefinition)):
        return []
    if isinstance(definition, NodeDefinition):
        return [NODE_DEFINITION_BY_TYPE[t] for t in definition.extended_by]
    if isinstance(definition, StructDefinition):
        return [STRUCT_DEFINITION_BY_TYPE[t] for t in definition.extended_by]
    return [HANDLE_DEFINITION_BY_TYPE[t] for t in definition.extended_by]


def _get_origin_for(
    owner: "EnumDefinition | HandleDefinition | ModuleDefinition | NodeDefinition | StructDefinition",
    attribute: "PropertyDefinition | MethodDefinition | ActionDefinition",
) -> str | None:
    object_cls = BUILTIN_CLASS_BY_NAME.get(owner.name)
    if object_cls is None:
        return None
    for super_cls in reversed(list(get_superclasses(object_cls))):
        # avoid hasattr per code style: check class dict
        if attribute.name in getattr(super_cls, "__dict__", {}):
            return super_cls.__name__
    return None


def _show_module(definition: "ModuleDefinition", context: ManualContext) -> None:
    """Display Module-specific details."""
    from ..builtin import StringCasing, to_casing

    if definition.methods:
        _show_methods(definition, definition.methods, "Methods", context)
    if definition.constants:
        _show_constants(definition.constants, "Constants", context)

    # types
    if definition.node_types:
        _console.print("\n" + _console.color("Nodes:", context.color_section_title, "bold"))
        for node_type in definition.node_types:
            camel_name = to_casing(node_type.name, StringCasing.UPPER_CAMEL)
            _console.print(
                f"  {_console.color(camel_name, context.color_origin)} ({_console.color(str(node_type.value), context.color_dim)})"
            )

    # structs
    if definition.struct_types:
        _console.print("\n" + _console.color("Structs:", context.color_section_title, "bold"))
        for struct_type in definition.struct_types:
            camel_name = to_casing(struct_type.name, StringCasing.UPPER_CAMEL)
            _console.print(
                f"  {_console.color(camel_name, context.color_origin)} ({_console.color(str(struct_type.value), context.color_dim)})"
            )

    # enums
    if definition.enum_types:
        _console.print("\n" + _console.color("Enums:", context.color_section_title, "bold"))
        for enum_type in definition.enum_types:
            camel_name = to_casing(enum_type.name, StringCasing.UPPER_CAMEL)
            _console.print(
                f"  {_console.color(camel_name, context.color_origin)} ({_console.color(str(enum_type.value), context.color_dim)})"
            )

    # handles
    if definition.handle_types:
        _console.print("\n" + _console.color("Handles:", context.color_section_title, "bold"))
        for handle_type in definition.handle_types:
            camel_name = to_casing(handle_type.name, StringCasing.UPPER_CAMEL)
            _console.print(
                f"  {_console.color(camel_name, context.color_origin)} ({_console.color(str(handle_type.value), context.color_dim)})"
            )

    # children
    if definition.children_paths:
        _console.print(f"  Children ({len(definition.children_paths)}):")
        for child_path in definition.children_paths:
            _console.print(
                f"  {_console.color(child_path, context.color_value)} ({_console.color(child_path, context.color_dim)})"
            )
