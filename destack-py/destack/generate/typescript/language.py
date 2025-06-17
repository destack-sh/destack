from dataclasses import dataclass
from pathlib import Path
from typing import Literal, assert_never

from destack.language import (
    BuiltinObjectBase,
    Enum,
    EnumDefinition,
    NodeDefinition,
    StructDefinition,
    TraitDefinition,
)
from destack.language.registry import (
    ENUM_CLASS_BY_TYPE,
    ENUM_DEFINITION_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    NODE_DEFINITION_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
    STRUCT_DEFINITION_BY_TYPE,
    TRAIT_CLASS_BY_TRAIT,
    TRAIT_DEFINITION_BY_TYPE,
)

GENERATION_PATH = "destack-ts/src/language"
MARKER_START = "/* DESTACK_GENERATED_START:{kind}:{id} */"
MARKER_END = "/* DESTACK_GENERATED_END:{kind}:{id} */"
MARKER_CUSTOM_START = "/* DESTACK_GENERATED_CUSTOM_START */"
MARKER_CUSTOM_END = "/* DESTACK_GENERATED_CUSTOM_END */"

Kind = Literal["ENUM", "STRUCT", "TRAIT", "NODE"]
Definition = EnumDefinition | StructDefinition | TraitDefinition | NodeDefinition


@dataclass(slots=True)
class SourceFile:
    name: str
    path: str
    definitions: dict[str, "SourceDefinition"]


@dataclass(slots=True)
class SourceDefinition:
    kind: Kind
    id: int
    definition: Definition
    definition_str: str


def _generate_enum(definition: EnumDefinition) -> str:
    enum_str = f"""
    export enum {definition.name} {{
        {", ".join(f"{option.name}: {option.id}" for option in definition.options)}
    }}
    """
    return enum_str


def _generate_struct(definition: StructDefinition) -> str:
    struct_str = """
    export class {definition.name} {{
        // nocheckin: implement
    }}
    """
    return struct_str


def _generate_trait(definition: TraitDefinition) -> str:
    trait_str = """
    export class {definition.name} {{
        // nocheckin: implement
    }}
    """
    return trait_str


def _generate_node(definition: NodeDefinition) -> str:
    node_str = """
    export class {definition.name} {{
        // nocheckin: implement
    }}
    """
    return node_str


def _generate_definition(definition: Definition) -> SourceDefinition:
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

    source_definition = SourceDefinition(
        kind=kind,
        id=definition.id,
        definition=definition,
        definition_str=definition_str,
    )
    return source_definition


def _generate_file(file: SourceFile, existing_file_str: str | None) -> str:
    if existing_file_str is not None:
        # nocheckin
        return existing_file_str

    file_parts: list[str] = []

    for definition in file.definitions.values():
        file_parts.append(MARKER_START.format(kind=definition.kind, id=definition.id))
        file_parts.append(definition.definition_str)
        file_parts.append(MARKER_END.format(kind=definition.kind, id=definition.id))

    return "\n\n".join(file_parts)


def _write_file(file: SourceFile) -> str:
    target_path = Path(GENERATION_PATH) / file.path
    target_path.parent.mkdir(parents=True, exist_ok=True)

    existing_file_str = target_path.read_text() if target_path.exists() else None
    target_file_str = _generate_file(file, existing_file_str)
    target_path.write_text(target_file_str)

    return target_file_str


def generate():
    """Generate the Typescript language code."""
    files_by_path: dict[str, SourceFile] = {}

    def _get_file(cls: type[Enum | BuiltinObjectBase]) -> tuple[str, SourceFile]:
        path = cls.__module__.split(".", maxsplit=2)[-1]
        if path not in files_by_path:
            files_by_path[path] = SourceFile(
                name=path,
                path=path,
                definitions={},
            )
        return path, files_by_path[path]

    for enum_type, enum_cls in ENUM_CLASS_BY_TYPE.items():
        _, file = _get_file(enum_cls)
        file.definitions[enum_cls.__name__] = _generate_definition(
            ENUM_DEFINITION_BY_TYPE[enum_type]
        )
    for struct_type, struct_cls in STRUCT_CLASS_BY_TYPE.items():
        _, file = _get_file(struct_cls)
        file.definitions[struct_cls.__name__] = _generate_definition(
            STRUCT_DEFINITION_BY_TYPE[struct_type]
        )
    for trait_type, trait_cls in TRAIT_CLASS_BY_TRAIT.items():
        _, file = _get_file(trait_cls)
        file.definitions[trait_cls.__name__] = _generate_definition(
            TRAIT_DEFINITION_BY_TYPE[trait_type]
        )
    for node_type, node_cls in NODE_CLASS_BY_TYPE.items():
        _, file = _get_file(node_cls)
        file.definitions[node_cls.__name__] = _generate_definition(
            NODE_DEFINITION_BY_TYPE[node_type]
        )

    for file in files_by_path.values():
        file_str = _write_file(file)
