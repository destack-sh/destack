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
        ActionDefinition,
        ConstantDefinition,
        EnumDefinition,
        HandleDefinition,
        MethodDefinition,
        ModuleDefinition,
        NodeDefinition,
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
    current: str | None = None
    history: list[str] = dataclasses.field(default_factory=list)


@cli.command()
def manual():
    """Interactive manual for the schema."""
    from destack import VERSION

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
        inner_str = f"{inner_str} | None"

    return inner_str


def _render_type_scalar(type: "Type") -> str:
    """Render a scalar type to a string."""
    from destack.core import (
        PRIMITIVE_PY_ANNOTATION_BY_TYPE,
        ScalarType,
        TypeCardinality,
    )

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
    # node reference
    elif type.scalar_type == ScalarType.NODE_REFERENCE:
        if type.node_types is None:
            return "Node"
        elif len(type.node_types) == 1:
            return NODE_CLASS_BY_TYPE[type.node_types[0]].__name__
        else:
            node_names = [NODE_CLASS_BY_TYPE[t].__name__ for t in type.node_types]
            return " | ".join(node_names)
    # node value
    elif type.scalar_type == ScalarType.NODE_VALUE:
        if type.node_types is None:
            return "Node"
        elif len(type.node_types) == 1:
            return NODE_CLASS_BY_TYPE[type.node_types[0]].__name__
        else:
            node_names = [NODE_CLASS_BY_TYPE[t].__name__ for t in type.node_types]
            return " | ".join(node_names)
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


def _show_properties(properties: list["PropertyDefinition"], title: str = "Properties") -> None:
    """Display properties section."""
    if properties:
        console.print("\n" + console.color(f"{title} ({len(properties)}):", "yellow", "bold"))
        for prop in properties:
            if prop.name in ("metakind", "metatype") or prop.name.startswith("_"):
                continue
            prop_str = f"  {console.color(prop.name, 'white')} ({console.color(str(prop.id), 'dim')}): {console.color(_render_type(prop.type), 'yellow')}"
            console.print(prop_str)


def _show_methods(methods: list["MethodDefinition"], title: str = "Methods") -> None:
    """Display methods section."""
    console.print("\n" + console.color(f"{title} ({len(methods)}):", "yellow", "bold"))
    for method in methods:
        input_signature_parts: list[str] = []
        for input_property in method.input_properties:
            input_property_str = f"{console.color(input_property.name, 'white')}: {console.color(_render_type(input_property.type), 'yellow')}"
            if input_property.default_value is not None:
                input_property_str = (
                    f"{input_property_str} = {_render_value(input_property.default_value)}"
                )
            input_signature_parts.append(input_property_str)
        input_signature = ", ".join(input_signature_parts)
        if method.output_property is not None:
            output_signature = (
                f"{console.color(_render_type(method.output_property.type), 'yellow')}"
            )
        else:
            output_signature = "None"
        console.print(
            f"  {console.color(method.name, 'white')} ({console.color(str(method.id), 'dim')}) {console.color('(', 'dim')}{input_signature}{console.color(')', 'dim')} {console.color('->', 'dim')} {output_signature}"
        )


def _show_actions(actions: list["ActionDefinition"], title: str = "Actions") -> None:
    """Display actions section."""
    from ...core import ActionType, StringCasing, to_casing

    console.print("\n" + console.color(f"{title} ({len(actions)}):", "yellow", "bold"))
    for action in actions:
        input_type = (
            to_casing(action.input_message_type.name, StringCasing.UPPER_CAMEL)
            if action.input_message_type
            else "None"
        )
        if action.type in (ActionType.STREAM_IN_UNARY_OUT, ActionType.STREAM_IN_STREAM_OUT):
            input_type = f"Stream[{input_type}]"
        output_type = (
            to_casing(action.output_message_type.name, StringCasing.UPPER_CAMEL)
            if action.output_message_type
            else "None"
        )
        if action.type in (ActionType.STREAM_IN_UNARY_OUT, ActionType.STREAM_IN_STREAM_OUT):
            output_type = f"Stream[{output_type}]"
        type_signature = f"{console.color(input_type, 'yellow')} {console.color('->', 'dim')} {console.color(output_type, 'yellow')}"
        console.print(
            f"  {console.color(action.name, 'white')} ({console.color(str(action.id), 'dim')}): {type_signature}"
        )


def _show_constants(constants: list["ConstantDefinition"], title: str = "Constants") -> None:
    """Display constants section."""
    console.print("\n" + console.color(f"{title}:", "yellow"))
    for constant in constants:
        console.print(
            f"  {console.color(constant.name, 'white')} ({console.color(str(constant.id), 'dim')}) = {_render_value(constant.value)}"
        )


def _show_node(definition: "NodeDefinition") -> None:
    """Display Node-specific details."""
    # metadata
    console.print("\n" + console.color("Metadata:", "yellow", "bold"))

    metadata = []
    metadata.append(("Inherits", "->".join([base.name for base in definition.inherits])))
    metadata.append(("Stability", str(definition.stability)))
    metadata.append(("Abstract", definition.is_abstract))
    metadata.append(("Final", definition.is_final))
    metadata.append(("Singleton", definition.is_singleton))

    # metadata
    for key, value in metadata:
        console.print(f"  {key}: {console.color(value, 'green')}")

    if definition.properties:
        _show_properties(definition.properties)
    if definition.methods:
        _show_methods(definition.methods)
    if definition.actions:
        _show_actions(definition.actions)


def _show_struct(definition: "StructDefinition") -> None:
    """Display Struct-specific details."""
    # metadata
    console.print("\n" + console.color("Metadata:", "yellow", "bold"))

    metadata = []
    metadata.append(("Inherits", "->".join([base.name for base in definition.inherits])))
    metadata.append(("Stability", str(definition.stability)))
    metadata.append(("Immutable", definition.is_immutable))
    metadata.append(("Abstract", definition.is_abstract))

    # metadata
    for key, value in metadata:
        console.print(f"  {key}: {console.color(value, 'green')}")

    if definition.properties:
        _show_properties(definition.properties)
    if definition.methods:
        _show_methods(definition.methods)


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
    if definition.properties:
        _show_properties(definition.properties)


def _show_module(definition: "ModuleDefinition") -> None:
    """Display Module-specific details."""
    from ...core.builtin import StringCasing, to_casing

    if definition.methods:
        _show_methods(definition.methods)
    if definition.constants:
        _show_constants(definition.constants)

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
