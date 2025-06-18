import textwrap
from collections import defaultdict
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import Literal, assert_never, cast

from destack.language import (
    BuiltinObjectBase,
    Enum,
    EnumDefinition,
    NodeBase,
    NodeDefinition,
    PrimitiveType,
    Property,
    RoleType,
    ScalarType,
    StructDefinition,
    Trait,
    TraitDefinition,
    TypeCardinality,
)
from destack.language.core.runtime.graph import get_node_types
from destack.language.registry import (
    ENUM_CLASS_BY_TYPE,
    ENUM_DEFINITION_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    NODE_DEFINITION_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
    STRUCT_DEFINITION_BY_TYPE,
    TRAIT_CLASS_BY_TRAIT,
    TRAIT_DEFINITION_BY_TYPE,
    TRAIT_TYPE_BY_CLASS,
)
from destack.utils.string import Casing, to_casing

GENERATION_PATH = "destack-ts/src/language"
MARKER_START = "/* ==== DESTACK_GENERATED_START:{kind}:{id} ==== */"
MARKER_END = "/* ==== DESTACK_GENERATED_END:{kind}:{id} ==== */"
MARKER_CUSTOM_START = "/* ==== DESTACK_GENERATED_CUSTOM_START ==== */"
MARKER_CUSTOM_END = "/* ==== DESTACK_GENERATED_CUSTOM_END ==== */"

Kind = Literal["ENUM", "STRUCT", "TRAIT", "NODE"]
Definition = EnumDefinition | StructDefinition | TraitDefinition | NodeDefinition


@dataclass(slots=True)
class TypescriptFile:
    name: str
    module: str
    definitions: Mapping[str, "TypescriptDefinition"]
    path: Path
    existing_str: str | None = None
    new_str: str | None = None


@dataclass(slots=True)
class TypescriptDefinition:
    kind: Kind
    id: int
    definition: Definition
    definition_str: str
    dependencies: Mapping[str, Definition]


TYPESCRIPT_TYPE_BY_PRIMITIVE_TYPE: Mapping[PrimitiveType, str] = {
    PrimitiveType.BOOLEAN: "boolean",
    PrimitiveType.INT16: "number",
    PrimitiveType.INT32: "number",
    PrimitiveType.INT64: "number",
    PrimitiveType.FLOAT32: "number",
    PrimitiveType.FLOAT64: "number",
    PrimitiveType.STRING: "string",
    PrimitiveType.UUID: "string",
    PrimitiveType.JSON: "any",
    PrimitiveType.BYTES: "Uint8Array",
    PrimitiveType.DATETIME: "DateTime",
    PrimitiveType.DATE: "Date",
    PrimitiveType.TIME: "Time",
    PrimitiveType.DURATION: "Duration",
}


def _generate_property(prop: Property, is_readonly: bool, is_interface: bool) -> str:
    """Generate a Property definition."""

    ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)

    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        # special case node references: custom getters/setters
        assert prop.cardinality == TypeCardinality.SCALAR, (
            f"node references must be scalar: {prop!r}"
        )
        node_types = get_node_types(prop.node_types)
        return ""

    elif prop.scalar_type == ScalarType.ENUM:
        assert prop.enum_type is not None, f"no enum_type for {prop!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[prop.enum_type]
        scalar_str = f"{enum_cls.__name__}"
    elif prop.scalar_type == ScalarType.PRIMITIVE:
        assert prop.primitive_type is not None, f"no primitive_type for {prop!r}"
        scalar_str = TYPESCRIPT_TYPE_BY_PRIMITIVE_TYPE[prop.primitive_type]
    elif prop.scalar_type == ScalarType.STRUCT:
        assert prop.struct_type is not None, f"no struct_type for {prop!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
        scalar_str = f"{struct_cls.__name__}"
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        raise ValueError(f"unsupported scalar_type: {prop!r}")
    else:
        assert_never(prop.scalar_type)

    prop_str = f"{ts_name}: {scalar_str}"
    if prop.is_optional:
        prop_str = f"{prop_str} | null"
    if (
        is_readonly
        or prop.can_write is None
        or prop.can_write == RoleType.SYSTEM
        or prop.is_managed
    ):
        prop_str = f"readonly {prop_str}"
    return f"{prop_str};"


def _generate_constructor(cls: type[BuiltinObjectBase]) -> str:
    """Generate a Typescript constructor."""
    return ""


def _generate_enum(definition: EnumDefinition) -> str:
    """Generate a Typescript Enum definition."""
    enum_parts: list[str] = []
    for option in definition.options:
        enum_parts.append(f"{option.name} = {option.id},")
    enum_str = f"""\
export enum {definition.name} {{
{textwrap.indent("\n".join(enum_parts), "  ")}
}}"""
    return enum_str


def _get_properties(cls: type[BuiltinObjectBase]) -> list[Property]:
    """Get the properties of a class."""
    properties = [prop for prop in cls.__properties__.values() if prop.id == 1 or prop.is_wired]
    properties.sort(key=lambda prop: prop.id or 0)
    return properties


def _generate_struct(definition: StructDefinition) -> str:
    """Generate a Typescript Struct definition."""
    struct_cls = STRUCT_CLASS_BY_TYPE[definition.type]
    struct_parts: list[str] = []

    # properties
    for prop in _get_properties(struct_cls):
        prop_str = _generate_property(prop, is_readonly=definition.is_frozen, is_interface=False)
        struct_parts.extend(prop_str.splitlines())

    # constructor
    struct_parts.append(_generate_constructor(struct_cls))

    struct_str = f"""\
export class {definition.name} extends BuiltinObject {{
{textwrap.indent("\n".join(struct_parts), "  ")}
}}"""
    return struct_str


def _generate_trait(definition: TraitDefinition) -> str:
    """Generate a Typescript Trait definition."""

    trait_cls = TRAIT_CLASS_BY_TRAIT[definition.type]
    trait_parts: list[str] = []

    # properties
    for prop in _get_properties(trait_cls):
        prop_str = _generate_property(prop, is_readonly=False, is_interface=True)
        trait_parts.extend(prop_str.splitlines())

    # interface
    trait_classes: list[type[NodeBase]] = [
        TRAIT_CLASS_BY_TRAIT[trait_type] for trait_type in definition.traits
    ]
    trait_classes.sort(key=lambda cls: TRAIT_TYPE_BY_CLASS[cast(type[Trait], cls)])
    implements_str = (
        " implements " + ", ".join(trait_cls.__name__ for trait_cls in trait_classes)
        if trait_classes
        else ""
    )
    trait_str = f"""\
export interface {definition.name}{implements_str} {{
{textwrap.indent("\n".join(trait_parts), "  ")}
}}"""
    return trait_str


def _generate_node(definition: NodeDefinition) -> str:
    """Generate a Typescript Node definition."""

    node_cls = NODE_CLASS_BY_TYPE[definition.type]
    node_parts: list[str] = []

    # properties
    for prop in _get_properties(node_cls):
        prop_str = _generate_property(prop, is_readonly=False, is_interface=False)
        node_parts.extend(prop_str.splitlines())

    # class
    trait_classes: list[type[NodeBase]] = [
        TRAIT_CLASS_BY_TRAIT[trait_type] for trait_type in definition.traits
    ]
    trait_classes.sort(key=lambda cls: TRAIT_TYPE_BY_CLASS[cast(type[Trait], cls)])
    implements_str = (
        " implements " + ", ".join(trait_cls.__name__ for trait_cls in trait_classes)
        if trait_classes
        else ""
    )
    node_str = f"""\
export class {definition.name} extends Node{implements_str} {{
{textwrap.indent("\n".join(node_parts), "  ")}
}}
"""
    return node_str


def _generate_definition(definition: Definition) -> TypescriptDefinition:
    dependencies: dict[str, Definition] = {}

    if isinstance(definition, EnumDefinition):
        definition_str = _generate_enum(definition)
        kind = "ENUM"
    elif isinstance(definition, StructDefinition):
        definition_str = _generate_struct(definition)
        kind = "STRUCT"
    elif isinstance(definition, TraitDefinition):
        definition_str = _generate_trait(definition)
        kind = "TRAIT"
    elif isinstance(definition, NodeDefinition):
        definition_str = _generate_node(definition)
        kind = "NODE"
    else:
        assert_never(definition)

    source_definition = TypescriptDefinition(
        kind=kind,
        id=definition.id,
        definition=definition,
        definition_str=definition_str,
        dependencies=dependencies,
    )
    return source_definition


def _generate_file(file: TypescriptFile) -> str:
    if file.existing_str is not None:
        # nocheckin
        return file.existing_str

    file_parts: list[str] = []

    for definition in file.definitions.values():
        file_parts.append(
            f"""\
{MARKER_START.format(kind=definition.kind, id=definition.id)}
{definition.definition_str}
{MARKER_END.format(kind=definition.kind, id=definition.id)}
"""
        )

    return "\n\n".join(file_parts)


def generate():
    """Generate the Typescript language code."""

    # collect definitions
    definition_by_cls: dict[type[BuiltinObjectBase] | type[Enum], TypescriptDefinition] = {}
    for enum_type, enum_cls in ENUM_CLASS_BY_TYPE.items():
        definition = _generate_definition(ENUM_DEFINITION_BY_TYPE[enum_type])
        definition_by_cls[enum_cls] = definition
    for struct_type, struct_cls in STRUCT_CLASS_BY_TYPE.items():
        definition = _generate_definition(STRUCT_DEFINITION_BY_TYPE[struct_type])
        definition_by_cls[struct_cls] = definition
    for trait_type, trait_cls in TRAIT_CLASS_BY_TRAIT.items():
        definition = _generate_definition(TRAIT_DEFINITION_BY_TYPE[trait_type])
        definition_by_cls[trait_cls] = definition
    for node_type, node_cls in NODE_CLASS_BY_TYPE.items():
        definition = _generate_definition(NODE_DEFINITION_BY_TYPE[node_type])
        definition_by_cls[node_cls] = definition

    # organize definitions into files
    definitions_by_module: dict[str, list[TypescriptDefinition]] = defaultdict(list)
    for cls, definition in definition_by_cls.items():
        module = cls.__module__
        definitions_by_module[module].append(definition)

    # generate files
    files_by_module: dict[str, TypescriptFile] = {}
    for module, definitions in definitions_by_module.items():
        target_path = Path(GENERATION_PATH) / (module.replace(".", "/") + ".ts")
        target_path.parent.mkdir(parents=True, exist_ok=True)
        existing_file_str = target_path.read_text() if target_path.exists() else None

        definitions_by_name: dict[str, TypescriptDefinition] = {
            definition.definition.name: definition for definition in definitions
        }
        files_by_module[module] = TypescriptFile(
            name=module,
            module=module,
            path=target_path,
            definitions=definitions_by_name,
            existing_str=existing_file_str,
        )

    for file in files_by_module.values():
        file.new_str = _generate_file(file)
        print("=" * 80)
        print(file.path)
        print("=" * 80)
        print(file.new_str)
        print("=" * 80)
