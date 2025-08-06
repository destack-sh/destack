"""Interactive manual browser for Destack builtin definitions."""

import dataclasses
from dataclasses import dataclass
from operator import itemgetter
from typing import TYPE_CHECKING, Any, assert_never

from destack.registry import BUILTIN_DEFINITION_BY_NAME

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

cli = create_cli("manual")

type _Definition = (
    "EnumDefinition | HandleDefinition | ModuleDefinition | NodeDefinition | StructDefinition"
)


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
    return [s[2] for s in suggestions[:n]]


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
    from ...core import invert_type

    return str(invert_type(type))


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

    if metadata:
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

    if metadata:
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
