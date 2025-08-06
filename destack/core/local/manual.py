"""Interactive manual browser for Destack builtin definitions."""

import dataclasses
from dataclasses import dataclass
from operator import itemgetter
from typing import TYPE_CHECKING, Any, assert_never

from destack.registry import (
    BUILTIN_DEFINITION_BY_NAME,
    ENUM_CLASS_BY_TYPE,
    HANDLE_CLASS_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
)

from . import console
from .parser import create_cli, create_repl

if TYPE_CHECKING:
    from destack import (
        EnumDefinition,
        HandleDefinition,
        ModuleDefinition,
        NodeDefinition,
        StructDefinition,
        Type,
    )

type_ = type
type _Definition = (
    "EnumDefinition | HandleDefinition | ModuleDefinition | NodeDefinition | StructDefinition"
)

cli = create_cli("manual")


@dataclass(slots=True)
class ManualContext:
    current: str | None = None
    history: list[str] = dataclasses.field(default_factory=list)


@cli.command()
def manual():
    """Interactive manual for the Destack language SDK."""
    # import here to avoid circular imports at module level
    from destack.core import VERSION

    from ...finalize import finalize

    # ensure definitions are loaded
    finalize()

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
    context = ManualContext()

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
        _show_definition(definition)

    @repl.command("back", "Go back to the previously viewed definition")
    def go_back():
        """Navigate back in history."""
        if not context.history:
            console.warn("No previous definition in history.")
            return

        prev = context.history.pop()
        context.current = prev

        definition = get_builtin_definition(prev)
        _show_definition(definition)

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


def _render_type(type: "Type") -> str:
    """Render a type to a string."""
    from ...core import TypeCardinality

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
        inner_str = f"dict[{key_name}, {value_name}]"
    else:
        assert_never(type.cardinality)

    # wrap in optional
    if not type.is_required:
        inner_str = f"Optional[{inner_str}]"

    return inner_str


def _render_type_scalar(type: "Type") -> str:
    """Render a scalar type to a string."""
    from destack.core import (
        PRIMITIVE_PY_ANNOTATION_BY_TYPE,
        ScalarType,
        TypeCardinality,
    )

    assert type.cardinality == TypeCardinality.SCALAR
    assert type.scalar_type is not None

    # primitive
    if type.scalar_type == ScalarType.PRIMITIVE:
        assert type.primitive_type is not None
        return PRIMITIVE_PY_ANNOTATION_BY_TYPE[type.primitive_type].__name__
    # enum
    elif type.scalar_type == ScalarType.ENUM:
        assert type.enum_type is not None
        return ENUM_CLASS_BY_TYPE[type.enum_type].__name__
    # node reference
    elif type.scalar_type == ScalarType.NODE_REFERENCE:
        assert type.node_types is not None
        if len(type.node_types) == 1:
            return NODE_CLASS_BY_TYPE[type.node_types[0]].__name__
        else:
            node_names = [NODE_CLASS_BY_TYPE[t].__name__ for t in type.node_types]
            return " | ".join(node_names)
    # node value
    elif type.scalar_type == ScalarType.NODE_VALUE:
        assert type.node_types is not None
        return NODE_CLASS_BY_TYPE[type.node_types[0]].__name__
    # struct
    elif type.scalar_type == ScalarType.STRUCT:
        assert type.struct_type is not None
        return STRUCT_CLASS_BY_TYPE[type.struct_type].__name__
    # handle
    elif type.scalar_type == ScalarType.HANDLE:
        assert type.handle_type is not None
        return HANDLE_CLASS_BY_TYPE[type.handle_type].__name__
    # union
    elif type.scalar_type == ScalarType.UNION:
        assert type.element_types is not None
        element_names = [_render_type_scalar(t) for t in type.element_types]
        return " | ".join(element_names)
    #
    else:
        assert_never(type.scalar_type)


def _show_definition(
    definition: "EnumDefinition | HandleDefinition | ModuleDefinition | NodeDefinition | StructDefinition",
) -> None:
    """Display a definition with full details."""
    from ...core.definition import (
        EnumDefinition,
        HandleDefinition,
        ModuleDefinition,
        NodeDefinition,
        StructDefinition,
    )

    # header
    console.print("")
    console.print("=" * 70, "bright_cyan")
    console.print(f"{definition.name} [{definition.__class__.__name__}]", "bright_cyan", "bold")
    console.print("=" * 70, "bright_cyan")

    # description
    if definition.description:
        console.print("\n" + console.color("Description:", "yellow", "bold"))
        # format description nicely
        desc_lines = definition.description.strip().split("\n")
        for line in desc_lines:
            console.print("  " + line)

    # type-specific information
    if isinstance(definition, NodeDefinition):
        _show_node(definition)
    elif isinstance(definition, StructDefinition):
        _show_struct(definition)
    elif isinstance(definition, EnumDefinition):
        _show_enum(definition)
    elif isinstance(definition, HandleDefinition):
        _show_handle(definition)
    elif isinstance(definition, ModuleDefinition):
        _show_module(definition)
    else:
        assert_never(definition)

    console.print("\n" + "=" * 70, "bright_cyan")


def _show_node(definition: "NodeDefinition") -> None:
    """Display Node-specific details."""
    # metadata
    console.print("\n" + console.color("Metadata:", "yellow", "bold"))

    metadata = []
    metadata.append(("Stability", str(definition.stability)))
    metadata.append(("Abstract", definition.is_abstract))
    metadata.append(("Final", definition.is_final))
    metadata.append(("Frozen", definition.is_frozen))
    metadata.append(("Singleton", definition.is_singleton))

    # metadata
    for key, value in metadata:
        console.print(f"  {key}: {console.color(value, 'green')}")

    # properties
    if definition.properties:
        console.print(
            "\n" + console.color(f"Properties ({len(definition.properties)}):", "yellow", "bold")
        )
        for prop in definition.properties:
            prop_str = f"  {prop.name}: {console.color(_render_type(prop.type), 'yellow')}"
            console.print(prop_str)

    # methods
    if definition.methods:
        console.print(
            "\n" + console.color(f"Methods ({len(definition.methods)}):", "yellow", "bold")
        )
        for method in definition.methods:
            console.print(f"  {console.color(method.name, 'magenta')}()")


def _show_struct(definition: "StructDefinition") -> None:
    """Display Struct-specific details."""
    # metadata
    console.print("\n" + console.color("Metadata:", "yellow", "bold"))

    metadata = []
    metadata.append(("Stability", str(definition.stability)))
    metadata.append(("Frozen", definition.is_frozen))
    metadata.append(("Abstract", definition.is_abstract))

    # metadata
    for key, value in metadata:
        console.print(f"  {key}: {console.color(value, 'green')}")

    # properties
    if definition.properties:
        console.print(
            "\n" + console.color(f"Properties ({len(definition.properties)}):", "yellow", "bold")
        )
        for prop in definition.properties:
            prop_str = f"  {prop.name}: {console.color(_render_type(prop.type), 'yellow')}"
            console.print(prop_str)


def _show_enum(definition: "EnumDefinition") -> None:
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


def _show_handle(definition: "HandleDefinition") -> None:
    """Display Handle-specific details."""
    # properties
    if definition.properties:
        console.print(
            "\n" + console.color(f"Properties ({len(definition.properties)}):", "yellow", "bold")
        )
        for prop in definition.properties:
            prop_str = f"  {prop.name}: {console.color(_render_type(prop.type), 'yellow')}"
            console.print(prop_str)


def _show_module(definition: "ModuleDefinition") -> None:
    """Display Module-specific details."""
    from ...core.utility.string import Casing, to_casing

    # content
    if definition.methods:
        console.print("\n" + console.color("Methods:", "yellow"))
        for method in definition.methods:
            camel_name = to_casing(method.name, Casing.CAMEL)
            console.print(f"  - {console.color(camel_name, 'cyan')}")
    if definition.node_types:
        console.print("\n" + console.color("Nodes:", "yellow"))
        for node_type in definition.node_types:
            camel_name = to_casing(node_type.name, Casing.CAMEL)
            console.print(f"  - {console.color(camel_name, 'cyan')}")
    if definition.struct_types:
        console.print("\n" + console.color("Structs:", "yellow"))
        for struct_type in definition.struct_types:
            camel_name = to_casing(struct_type.name, Casing.CAMEL)
            console.print(f"  - {console.color(camel_name, 'cyan')}")
    if definition.enum_types:
        console.print("\n" + console.color("Enums:", "yellow"))
        for enum_type in definition.enum_types:
            camel_name = to_casing(enum_type.name, Casing.CAMEL)
            console.print(f"  - {console.color(camel_name, 'cyan')}")
    if definition.handle_types:
        console.print("\n" + console.color("Handles:", "yellow"))
        for handle_type in definition.handle_types:
            camel_name = to_casing(handle_type.name, Casing.CAMEL)
            console.print(f"  - {console.color(camel_name, 'cyan')}")
    if definition.children_paths:
        console.print(f"  Children ({len(definition.children_paths)}):")
        for child_path in definition.children_paths:
            console.print(f"    - {console.color(child_path, 'green')}")
