"""Interactive manual browser for Destack builtin definitions."""

import dataclasses
from dataclasses import dataclass
from operator import itemgetter
from typing import TYPE_CHECKING, Any, Literal, assert_never

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
from . import console
from .parser import create_cli, create_repl

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

cli = create_cli("manual", "Interactive manual for the schema.")


@dataclass(slots=True)
class ManualContext:
    memory_sizer: "ObjectSizer"
    packed_sizer: "ObjectSizer"
    sizers: dict[str, "ObjectSizer"]
    current: str | None = None
    history: list[str] = dataclasses.field(default_factory=list)


@cli.command()
def manual():
    """Interactive manual for the schema."""
    from destack import (
        VERSION,
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
        packed_sizer=KompaktObjectSizer(),
        sizers={
            "Memory": RustObjectSizer(),
            "Packed": KompaktObjectSizer(),
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
    from ...registry import BUILTIN_DEFINITION_BY_NAME, get_builtin_definition

    @repl.command("show", "Show detailed information about a definition")
    def show_definition(*args):
        """Show detailed definition information."""
        if not args:
            console.error("Please provide a definition name.")
            console.print("Use 'list' to see available definitions.", "dim")
            return

        name = args[0]
        definitions = _find_definition(name)
        if not definitions:
            console.error(f"Definition '{name}' not found.")
            return
        definition = definitions[0]

        # update context
        if context.current and context.current != name:
            context.history.append(context.current)
        context.current = name

        # display the definition
        _show_definition(definition, context)

    @repl.command(
        "tree",
        "Show inheritance tree. Usage: tree <name> [--direction=up|down|both|-d up] [--full|-f]",
    )
    def show_tree(*args):
        """Show inheritance tree for an Object (Node/Struct/Handle)."""
        if not args:
            console.error("Please provide a definition name.")
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
            console.error(f"Definition '{name}' not found.")
            return
        definition = definitions[0]
        # if we go down, default to full subtree
        include_subclasses = full or direction in ("down", "both")
        tree = _render_inheritance_tree(
            definition, direction=direction, include_subclasses=include_subclasses
        )
        if tree is None:
            console.warn("Only Objects (Node/Struct/Handle) have inheritance.")
            return
        console.section("Inheritance Tree", tree)

    @repl.command("back", "Go back to the previously viewed definition")
    def go_back():
        """Navigate back in history."""
        if not context.history:
            console.warn("No previous definition in history.")
            return

        prev = context.history.pop()
        context.current = prev

        definition = get_builtin_definition(prev)
        _show_definition(definition, context)

    @repl.command("up", "Show the base (parent) of the current definition")
    def go_up():
        """Navigate to the base of the current Object definition."""
        if not context.current:
            console.warn("No current definition selected.")
            return
        defs = _find_definition(context.current, 1)
        if not defs:
            console.warn("Current definition not found.")
            return
        parent = _get_base_definition(defs[0])
        if parent is None:
            console.warn("No base definition.")
            return
        context.history.append(context.current)
        context.current = parent.name
        _show_definition(parent, context)

    @repl.command("down", "Show children (direct subclasses) of the current definition")
    def go_down():
        """Show direct subclasses of the current Object definition."""
        if not context.current:
            console.warn("No current definition selected.")
            return
        defs = _find_definition(context.current, 1)
        if not defs:
            console.warn("Current definition not found.")
            return
        children = _get_children_definitions(defs[0])
        if not children:
            console.warn("No direct subclasses.")
            return
        console.section("Direct Subclasses", "\n".join(d.name for d in children))

    @repl.command("stats", "Show statistics about all definitions")
    def show_stats():
        """Show statistics."""
        from ...core.definition import (
            EnumDefinition,
            HandleDefinition,
            ModuleDefinition,
            NodeDefinition,
            StructDefinition,
        )

        stats = {
            "Node": 0,
            "Struct": 0,
            "Enum": 0,
            "Handle": 0,
            "Module": 0,
        }
        for definition in BUILTIN_DEFINITION_BY_NAME.values():
            if isinstance(definition, NodeDefinition):
                stats["Node"] += 1
            elif isinstance(definition, StructDefinition):
                stats["Struct"] += 1
            elif isinstance(definition, EnumDefinition):
                stats["Enum"] += 1
            elif isinstance(definition, HandleDefinition):
                stats["Handle"] += 1
            elif isinstance(definition, ModuleDefinition):
                stats["Module"] += 1

        console.section("Definition Statistics")

        total = sum(stats.values())
        console.print(f"Total Definitions: {console.color(str(total), 'green', 'bold')}\n")

        headers = ["Type", "Count", "Percentage"]
        rows = []
        for def_type, count in sorted(stats.items(), key=lambda x: -x[1]):
            percentage = f"{(count / total * 100):.1f}%"
            rows.append([console.color(def_type, "cyan"), str(count), percentage])

        console.print(console.table(rows, headers))

    # default handler for direct definition names
    @repl.command("default")
    def handle_default(user_input: str):
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


def _render_size(size: "ObjectSize") -> str:
    """Render a size to a string."""
    if size.max_size is None:
        return f"{size.min_size}.."
    elif size.min_size == size.max_size:
        return f"{size.min_size}"
    else:
        return f"{size.min_size}..{size.max_size}"


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
        ScalarType.NODE_MOMENT,
        ScalarType.NODE_UNTYPED_IDENTITY,
        ScalarType.NODE_IDENTITY,
        ScalarType.NODE_LOCATION,
    ):
        if type.node_types is None:
            node_str = "Node"
        elif len(type.node_types) == 1:
            node_str = NODE_CLASS_BY_TYPE[type.node_types[0]].__name__
        else:
            node_names = [NODE_CLASS_BY_TYPE[t].__name__ for t in type.node_types]
            node_str = " | ".join(node_names)
        return f"->{node_str}[{type.scalar_type.name}]"
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
    from ...core.definition import (
        EnumDefinition,
        HandleDefinition,
        ModuleDefinition,
        NodeDefinition,
        StructDefinition,
    )

    definition_cls = BUILTIN_CLASS_BY_NAME[definition.name]

    # header
    console.print("")
    console.print("=" * 70, "bright_cyan")
    console.print(f"{definition.name} [{definition.__class__.__name__}]", "bright_cyan", "bold")
    console.print(f"{definition_cls.__module__}", "bright_cyan")
    console.print("=" * 70, "bright_cyan")

    # description
    if definition.description:
        console.print("\n" + console.color("Description:", "yellow", "bold"))
        desc_lines = definition.description.strip().split("\n")
        for line in desc_lines:
            console.print("  " + line)

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

    console.print("\n" + "=" * 70, "bright_cyan")


def _show_properties(
    owner: "EnumDefinition | HandleDefinition | ModuleDefinition | NodeDefinition | StructDefinition",
    properties: list["PropertyDefinition"],
    title: str,
    context: ManualContext,
) -> None:
    """Display properties section."""
    instance_properties = [p for p in properties if not p.is_static and not p.is_runtime_only]
    console.print("\n" + console.color(f"{title} ({len(instance_properties)}):", "yellow", "bold"))
    rows: list[list[str]] = []
    headers = ["ID", "Name", "Type", "Memory", "Packed", "Defined In", "Flags"]
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
        rows.append(
            [
                console.color(str(prop.id), "dim"),
                console.color(prop.name, "white"),
                console.color(_render_type(prop.type), "yellow"),
                console.color(_render_size(context.memory_sizer.size_type(prop.type)), "green"),
                console.color(_render_size(context.packed_sizer.size_type(prop.type)), "green"),
                console.color(origin or "-", "cyan"),
                console.color("|".join(flags) if flags else "", "gray"),
            ]
        )
    console.print(console.table(rows, headers))


def _show_methods(
    owner: "EnumDefinition | HandleDefinition | ModuleDefinition | NodeDefinition | StructDefinition",
    methods: list["MethodDefinition"],
    title: str,
    context: ManualContext,
) -> None:
    """Display methods section formatted as a table."""
    console.print("\n" + console.color(f"{title} ({len(methods)}):", "yellow", "bold"))
    if not methods:
        return
    headers = ["ID", "Name", "Inputs", "Returns", "Defined In", "Alias"]
    rows: list[list[str]] = []
    for method in methods:
        if method.alias_of is not None:
            # find the aliased method by id
            aliased_method = next((m for m in methods if m.id == method.alias_of), None)
            assert aliased_method is not None, f"aliased method #{method.alias_of} not found"
            rows.append(
                [
                    console.color(str(method.id), "dim"),
                    console.color(method.name, "white"),
                    console.color("-", "gray"),
                    console.color("-", "gray"),
                    console.color(_get_origin_for(owner, method) or "-", "cyan"),
                    console.color(aliased_method.name, "cyan"),
                ]
            )
            continue

        input_signature_parts: list[str] = []
        for input_property in method.input_properties:
            part = f"{console.color(input_property.name, 'white')}: {console.color(_render_type(input_property.type), 'yellow')}"
            if input_property.default_value is not None:
                part = f"{part} = {_render_value(input_property.default_value)}"
            input_signature_parts.append(part)
        inputs = (
            ", ".join(input_signature_parts)
            if input_signature_parts
            else console.color("-", "gray")
        )
        returns = (
            console.color(_render_type(method.output_property.type), "yellow")
            if method.output_property is not None
            else console.color("None", "gray")
        )
        rows.append(
            [
                console.color(str(method.id), "dim"),
                console.color(method.name, "white"),
                inputs,
                returns,
                console.color(_get_origin_for(owner, method) or "-", "cyan"),
                console.color("-", "gray"),
            ]
        )
    console.print(console.table(rows, headers))


def _show_actions(
    owner: "EnumDefinition | HandleDefinition | ModuleDefinition | NodeDefinition | StructDefinition",
    actions: list["ActionDefinition"],
    title: str,
    context: ManualContext,
) -> None:
    """Display actions as a table with type and message IO."""
    from ...core import ActionType, StringCasing, to_casing

    console.print("\n" + console.color(f"{title} ({len(actions)}):", "yellow", "bold"))
    if not actions:
        return
    headers = ["ID", "Name", "Type", "Input", "Output", "Defined In"]
    rows: list[list[str]] = []
    for action in actions:
        # action type label
        type_label = action.type.name if isinstance(action.type, ActionType) else str(action.type)

        def fmt_msg(msg_type):
            if msg_type is None:
                return console.color("None", "gray")
            return console.color(to_casing(msg_type.name, StringCasing.UPPER_CAMEL), "yellow")

        input_label = fmt_msg(action.input_message_type)
        output_label = fmt_msg(action.output_message_type)

        if action.type in (ActionType.STREAM_IN_UNARY_OUT, ActionType.STREAM_IN_STREAM_OUT):
            input_label = f"Stream[{input_label}]"
        if action.type in (ActionType.UNARY_IN_STREAM_OUT, ActionType.STREAM_IN_STREAM_OUT):
            output_label = f"Stream[{output_label}]"

        rows.append(
            [
                console.color(str(action.id), "dim"),
                console.color(action.name, "white"),
                console.color(type_label, "cyan"),
                input_label,
                output_label,
                console.color(_get_origin_for(owner, action) or "-", "cyan"),
            ]
        )

    console.print(console.table(rows, headers))


def _show_constants(
    constants: list["ConstantDefinition"],
    title: str,
    context: ManualContext,
) -> None:
    """Display constants section."""
    console.print("\n" + console.color(f"{title}:", "yellow"))
    for constant in constants:
        console.print(
            f"  {console.color(constant.name, 'white')} ({console.color(str(constant.id), 'dim')}) = {_render_value(constant.value)}"
        )


def _show_size(definition: "StructDefinition | NodeDefinition", context: ManualContext) -> None:
    """Display size information."""

    # metadata size
    console.print(
        f"  Memory: {console.color(_render_size(context.memory_sizer.size_object(definition)), 'green')}"
    )
    console.print(
        f"  Packed: {console.color(_render_size(context.packed_sizer.size_object(definition)), 'green')}"
    )

    # specific sizers
    console.print("\n" + console.color("Sizes:", "yellow", "bold"))
    for name, sizer in context.sizers.items():
        console.print(
            f"  {console.color(name, 'cyan')}: {console.color(_render_size(sizer.size_object(definition)), 'green')}"
        )


def _show_node(definition: "NodeDefinition", context: ManualContext) -> None:
    """Display Node-specific details."""
    # metadata
    console.print("\n" + console.color("Metadata:", "yellow", "bold"))
    metadata = []
    metadata.append(("Inherits", "->".join([base.name for base in definition.inherits])))
    metadata.append(("Stability", str(definition.stability)))
    metadata.append(("Abstract", definition.is_abstract))
    metadata.append(("Final", definition.is_final))
    metadata.append(("Singleton", definition.is_singleton))
    for key, value in metadata:
        console.print(f"  {key}: {console.color(value, 'green')}")
    _show_size(definition, context)

    # inheritance tree (object-only)
    tree = _render_inheritance_tree(definition, direction="both", include_subclasses=False)
    if tree:
        console.print("")
        console.print("\n" + console.color("Inheritance:", "yellow", "bold"))
        console.print(tree)

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

    # metadata
    console.print("\n" + console.color("Metadata:", "yellow", "bold"))
    metadata = []
    metadata.append(("Inherits", "->".join([base.name for base in definition.inherits])))
    metadata.append(("Stability", str(definition.stability)))
    metadata.append(("Abstract", definition.is_abstract))
    for key, value in metadata:
        console.print(f"  {key}: {console.color(value, 'green')}")
    _show_size(definition, context)

    # inheritance tree (object-only)
    tree = _render_inheritance_tree(definition, direction="both", include_subclasses=False)
    if tree:
        console.print("")
        console.print("\n" + console.color("Inheritance:", "yellow", "bold"))
        console.print(tree)

    if definition.properties:
        _show_properties(definition, definition.properties, "Properties", context)
    if definition.methods:
        _show_methods(definition, definition.methods, "Methods", context)


def _show_enum(definition: "EnumDefinition", context: ManualContext) -> None:
    """Display Enum-specific details."""
    # options
    if definition.options:
        console.print(
            "\n" + console.color(f"Options ({len(definition.options)}):", "yellow", "bold")
        )
        for option in definition.options:
            opt_str = f"  {console.color(option.name, 'green')} = {option.id}"
            if option.description:
                opt_str = f"  # {console.color(option.description, 'dim')}\n{opt_str}"
            console.print(opt_str)


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
    include_subclasses: bool = False,
) -> str | None:
    """Build and render the inheritance tree for an Object definition.

    Returns None for non-Object (e.g., Module/Enum).
    """
    from ...core.definition import (
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
    def get_base(defn: NodeDefinition | StructDefinition | HandleDefinition):
        if isinstance(defn, NodeDefinition):
            base_type = defn.base_type
            return NODE_DEFINITION_BY_TYPE.get(base_type) if base_type is not None else None
        if isinstance(defn, StructDefinition):
            base_type = defn.base_type
            return STRUCT_DEFINITION_BY_TYPE.get(base_type) if base_type is not None else None
        base_type = defn.base_type
        return HANDLE_DEFINITION_BY_TYPE.get(base_type) if base_type is not None else None

    def get_children(defn: NodeDefinition | StructDefinition | HandleDefinition) -> list[Any]:
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
        parent = get_base(cur)  # type: ignore[arg-type]
        if parent is None:
            break
        ancestors.append(parent)
        cur = parent

    # build label map
    def label(defn: Any) -> str:
        # compute descendant count for this node
        def count_desc(d: Any) -> int:
            return len(_get_all_descendants(d))

        num = count_desc(defn)
        suffix = f" ({num})" if num > 0 else ""
        return f"{defn.name}{suffix}"

    label_by_id: dict[str, str] = {}
    children_by_id: dict[str, list[str]] = {}

    target_id = definition.name
    label_by_id[target_id] = label(definition)

    def add_edge(parent: Any, child: Any) -> None:
        pid = parent.name
        cid = child.name
        label_by_id.setdefault(pid, label(parent))
        label_by_id.setdefault(cid, label(child))
        children_by_id.setdefault(pid, []).append(cid)

    # choose root and assemble according to direction
    if direction in ("up", "both"):
        # stitch the ancestor chain: root -> ... -> target
        chain = [*list(reversed(ancestors)), definition]
        for i in range(len(chain) - 1):
            add_edge(chain[i], chain[i + 1])
        root_id = chain[0].name if chain else definition.name
    else:
        root_id = definition.name

    if direction in ("down", "both"):
        # include descendants from the target (only direct unless full=True)
        def walk_desc(defn: Any):
            children = get_children(defn)
            for child in children:
                add_edge(defn, child)
                if include_subclasses:
                    walk_desc(child)

        walk_desc(definition)

    # if we only asked for up, ensure root is the top ancestor
    if direction == "up":
        # nothing else to do; render the chain
        pass

    return console.render_tree(root_id, children_by_id, label_by_id, highlight_id=target_id)


def _get_all_descendants(
    definition: "EnumDefinition | HandleDefinition | ModuleDefinition | NodeDefinition | StructDefinition",
) -> list[Any]:
    from ...core.definition import (
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
    all_desc: list[Any] = []
    for child in direct:
        all_desc.append(child)
        all_desc.extend(_get_all_descendants(child))
    return all_desc


def _get_base_definition(
    definition: "EnumDefinition | HandleDefinition | ModuleDefinition | NodeDefinition | StructDefinition",
):
    from ...core.definition import (
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
    from ...core.definition import (
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
        if hasattr(super_cls, attribute.name):
            return super_cls.__name__
    return None


def _show_module(definition: "ModuleDefinition", context: ManualContext) -> None:
    """Display Module-specific details."""
    from ...core.builtin import StringCasing, to_casing

    if definition.methods:
        _show_methods(definition, definition.methods, "Methods", context)
    if definition.constants:
        _show_constants(definition.constants, "Constants", context)

    # types
    if definition.node_types:
        console.print("\n" + console.color("Nodes:", "yellow", "bold"))
        for node_type in definition.node_types:
            camel_name = to_casing(node_type.name, StringCasing.UPPER_CAMEL)
            console.print(
                f"  {console.color(camel_name, 'cyan')} ({console.color(str(node_type.value), 'dim')})"
            )

    # structs
    if definition.struct_types:
        console.print("\n" + console.color("Structs:", "yellow", "bold"))
        for struct_type in definition.struct_types:
            camel_name = to_casing(struct_type.name, StringCasing.UPPER_CAMEL)
            console.print(
                f"  {console.color(camel_name, 'cyan')} ({console.color(str(struct_type.value), 'dim')})"
            )

    # enums
    if definition.enum_types:
        console.print("\n" + console.color("Enums:", "yellow", "bold"))
        for enum_type in definition.enum_types:
            camel_name = to_casing(enum_type.name, StringCasing.UPPER_CAMEL)
            console.print(
                f"  {console.color(camel_name, 'cyan')} ({console.color(str(enum_type.value), 'dim')})"
            )

    # handles
    if definition.handle_types:
        console.print("\n" + console.color("Handles:", "yellow", "bold"))
        for handle_type in definition.handle_types:
            camel_name = to_casing(handle_type.name, StringCasing.UPPER_CAMEL)
            console.print(
                f"  {console.color(camel_name, 'cyan')} ({console.color(str(handle_type.value), 'dim')})"
            )

    # children
    if definition.children_paths:
        console.print(f"  Children ({len(definition.children_paths)}):")
        for child_path in definition.children_paths:
            console.print(
                f"  {console.color(child_path, 'green')} ({console.color(child_path, 'dim')})"
            )
