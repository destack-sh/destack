import re
import textwrap
from collections import defaultdict
from pathlib import Path
from typing import assert_never, cast

from destack.language import (
    EMPTY_DICT,
    NODE_TYPES,
    BuiltinObjectBase,
    Enum,
    EnumDefinition,
    Node,
    NodeBase,
    NodeDefinition,
    NodeType,
    Property,
    RoleType,
    ScalarType,
    StructDefinition,
    Trait,
    TraitDefinition,
    TraitType,
    TypeCardinality,
    get_node_types,
)
from destack.language.registry import (
    ENUM_CLASS_BY_TYPE,
    ENUM_DEFINITION_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    NODE_DEFINITION_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
    STRUCT_DEFINITION_BY_TYPE,
    TRAIT_CLASS_BY_TYPE,
    TRAIT_DEFINITION_BY_TYPE,
    TRAIT_TYPE_BY_CLASS,
)
from destack.utils.string import Casing, to_casing

from .const import GENERATION_PATH, MARKER_CUSTOM_START, MARKER_END, MARKER_START, Definition, Kind
from .core import (
    TypescriptCodeBlock,
    TypescriptDefinition,
    TypescriptDefinitionBlock,
    TypescriptFile,
)
from .map import TYPESCRIPT_TYPE_BY_PRIMITIVE_TYPE


def _get_properties(cls: type[BuiltinObjectBase]) -> list[Property]:
    """Get the properties of a class."""
    properties: list[Property] = []
    for prop in cls.__wired_properties__.values():
        if prop.id == 1:
            continue
        if prop.runtime_prop is not None:
            prop = prop.runtime_prop
        properties.append(prop)
    properties.sort(key=lambda prop: prop.id or 0)
    return properties


def _is_property_readonly(prop: Property) -> bool:
    """Check if a property is (effectively) readonly to the user."""
    return prop.can_write is None or prop.can_write == RoleType.SYSTEM or prop.is_managed


def _generate_property(
    prop: Property, *, is_readonly: bool, is_node: bool, is_interface: bool
) -> str:
    """Generate a Property definition."""

    ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)

    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        # special case node references: custom getters/setters
        assert prop.cardinality == TypeCardinality.SCALAR, (
            f"node references must be scalar: {prop!r}"
        )

        # ptr property
        ptr_prop_name = f"{ts_name}Ptr"
        ptr_prop_str = f"{ptr_prop_name}: NodeReference"
        if prop.is_optional:
            ptr_prop_str = f"{ptr_prop_str} | null"
        if is_readonly:
            ptr_prop_str = f"readonly {ptr_prop_str}"

        # node property
        node_types = get_node_types(prop.node_types)
        node_classes = tuple(NODE_CLASS_BY_TYPE[node_type] for node_type in node_types or ())
        if node_classes and len(node_classes) < len(NODE_TYPES):
            node_scalar_str = " | ".join(node_cls.__name__ for node_cls in node_classes)
        else:
            node_scalar_str = "Node"

        # node getter/setter
        if is_interface:
            # interface declaration
            node_getter_str = f"get {ts_name}(): {node_scalar_str} | null;"
            if not is_readonly:
                node_setter_str = f"set {ts_name}(value: {node_scalar_str} | null): void;"
                node_prop_str = f"{node_getter_str}\n{node_setter_str}"
            else:
                node_prop_str = node_getter_str
        else:
            # actual getter/setter
            if is_node:
                node_getter_str = f"""\
get {ts_name}(): {node_scalar_str} | null {{
    const nodePtr: NodeReference | null = this.{ptr_prop_name};
    if (nodePtr !== null) {{
        return this._supergraph.get(nodePtr.id);
    }}
    return null;
}}
"""
                if not is_readonly:
                    node_setter_str = f"""\
set {ts_name}(value: {node_scalar_str} | null) {{
    if (value === null) {{
        this.{ptr_prop_name} = null;
    }} else {{
        this.{ptr_prop_name} = value.toRef();
    }}
}}
"""
                    node_prop_str = f"{node_getter_str}\n{node_setter_str}"
                else:
                    node_prop_str = node_getter_str

            else:
                node_getter_str = f"""\
get {ts_name}(): {node_scalar_str} | null {{
    const nodePtr: NodeReference | null = this.{ptr_prop_name};
    if (nodePtr !== null) {{
        if (this._supergraph === null) {{
            return null;
        }}
        return this._supergraph.get(nodePtr.id);
    }}
    return null;
}}
"""
                if not is_readonly:
                    node_setter_str = f"""\
set {ts_name}(value: {node_scalar_str} | null) {{
    if (value == null) {{
        this.{ptr_prop_name} = null;
    }} else {{
        this.{ptr_prop_name} = value.toRef();
    }}
}}
"""
                    node_prop_str = f"{node_getter_str}\n{node_setter_str}"
                else:
                    node_prop_str = node_getter_str

        return f"{node_prop_str};\n{ptr_prop_str};"

    elif prop.scalar_type == ScalarType.ENUM:
        assert prop.enum_type is not None, f"no enum_type for {prop!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[prop.enum_type]
        node_scalar_str = f"{enum_cls.__name__}"
    elif prop.scalar_type == ScalarType.PRIMITIVE:
        assert prop.primitive_type is not None, f"no primitive_type for {prop!r}"
        node_scalar_str = TYPESCRIPT_TYPE_BY_PRIMITIVE_TYPE[prop.primitive_type]
    elif prop.scalar_type == ScalarType.STRUCT:
        assert prop.struct_type is not None, f"no struct_type for {prop!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
        node_scalar_str = f"{struct_cls.__name__}"
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        raise ValueError(f"unsupported scalar_type: {prop!r}")
    else:
        assert_never(prop.scalar_type)

    node_prop_str = f"{ts_name}: {node_scalar_str}"
    if prop.is_optional:
        node_prop_str = f"{node_prop_str} | null"
    if is_readonly:
        node_prop_str = f"readonly {node_prop_str}"
    return f"{node_prop_str};"


def _generate_init(cls: type[BuiltinObjectBase]) -> str:
    """Generate a Typescript constructor."""
    return "// nocheckin: Typescript BuiltinObject.constructor"


def _generate_equals(cls: type[BuiltinObjectBase]) -> str:
    """Generate a Typescript equals method."""
    return "// nocheckin: Typescript BuiltinObject.equals"


def _generate_hash(cls: type[BuiltinObjectBase]) -> str:
    """Generate a Typescript hash method."""
    return "// nocheckin: Typescript BuiltinObject.hash"


def _generate_validate(cls: type[BuiltinObjectBase]) -> str:
    """Generate a Typescript validate method."""
    return "// nocheckin: Typescript BuiltinObject.validate"


def _generate_to_ref(cls: type["Node"]) -> str:
    """Generate a Typescript toRef method."""

    node_type = cls.metatype
    if node_type == NodeType.SPACE:
        ref_impl = f"""\
__toRef__(): NodeReference {{
    return new NodeReference({{
        nodeType: NodeType.{node_type.name},
        id: this.id,
        spaceId: this.id,
    }});
}}
"""
    elif TraitType.CUSTOM_NODE in cls.__traits__:
        ref_impl = f"""\
__toRef__(): NodeReference {{
    return new NodeReference({{
        nodeType: NodeType.{node_type.name},
        id: this.id,
        definitionId: this.definitionId,
        spaceId: this.spaceId,
    }});
}}
"""
    elif TraitType.SPATIAL in cls.__traits__:
        ref_impl = f"""\
__toRef__(): NodeReference {{
    return new NodeReference({{
        nodeType: NodeType.{node_type.name},
        id: this.id,
        spaceId: this.spaceId,
    }});
}}
"""
    else:
        ref_impl = f"""\
__toRef__(): NodeReference {{
    return new NodeReference({{
        nodeType: NodeType.{node_type.name},
        id: this.id,
    }});
"""
    return ref_impl.strip()


def _generate_path(cls: type[Node]) -> str:
    """Generate a Typescript path method."""
    # Node._path_key
    if "slug" in cls.__properties__:
        if "name" in cls.__properties__:
            path_key_str = "this.slug or this.name"
        else:
            path_key_str = f'this.slug or "{cls.__name__}[id={{this.id}}]"'
    elif "name" in cls.__properties__:
        path_key_str = "this.name"
    elif "title" in cls.__properties__:
        path_key_str = "this.title"
    else:
        path_key_str = f'"{cls.__name__}[id={{this.id}}]"'

    # Node.path
    if cls.__root_type__ is None:
        path_str = f"""\
get _pathKey(): string {{
    return {path_key_str};
}}

get path(): string {{
    return {path_key_str};
}}"""
    else:
        path_str = f"""\
get _pathKey(): string {{
    return {path_key_str};
}}

get path(): string {{
    const path_parts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {{
        path_parts.push(node._pathKey);
        node = node.parent;
    }}
    if (!this._isAttached) {{
        path_parts.push("<detached>");
    }}
    return path_parts.reverse().join("/");
}}
"""

    return path_str.strip()


def _generate_enum(definition: EnumDefinition) -> str:
    """Generate a Typescript Enum definition."""
    enum_parts: list[str] = []
    for option in definition.options:
        enum_parts.append(f"{option.name} = {option.id},")
    enum_str = f"""\
export enum {definition.name} {{
{textwrap.indent("\n".join(enum_parts), "  ")}
}}
"""
    return enum_str.strip()


def _generate_struct(definition: StructDefinition) -> str:
    """Generate a Typescript Struct definition."""
    struct_cls = STRUCT_CLASS_BY_TYPE[definition.type]
    struct_parts: list[str] = []

    # properties
    for prop in _get_properties(struct_cls):
        prop_str = _generate_property(
            prop,
            is_readonly=definition.is_frozen,
            is_node=False,
            is_interface=False,
        )
        struct_parts.extend(prop_str.splitlines())

    # body
    init_str = _generate_init(struct_cls)
    struct_parts.append(init_str)
    equals_str = _generate_equals(struct_cls)
    struct_parts.append(equals_str)
    hash_str = _generate_hash(struct_cls)
    struct_parts.append(hash_str)
    validate_str = _generate_validate(struct_cls)
    struct_parts.append(validate_str)

    struct_str = f"""\
export class {definition.name} extends BuiltinObject {{
{textwrap.indent("\n".join(struct_parts), "  ")}
}}"""
    return struct_str.strip()


def _generate_trait(definition: TraitDefinition) -> str:
    """Generate a Typescript Trait definition."""

    trait_cls = TRAIT_CLASS_BY_TYPE[definition.type]
    trait_parts: list[str] = []

    # properties
    for prop in _get_properties(trait_cls):
        prop_str = _generate_property(
            prop,
            is_readonly=_is_property_readonly(prop),
            is_node=False,
            is_interface=True,
        )
        trait_parts.extend(prop_str.splitlines())

    # interface
    trait_classes: list[type[NodeBase]] = [
        TRAIT_CLASS_BY_TYPE[trait_type] for trait_type in definition.traits
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
    return trait_str.strip()


def _generate_node(definition: NodeDefinition) -> str:
    """Generate a Typescript Node definition."""

    node_cls = NODE_CLASS_BY_TYPE[definition.type]
    node_parts: list[str] = []

    # properties
    for prop in _get_properties(node_cls):
        prop_str = _generate_property(
            prop,
            is_readonly=_is_property_readonly(prop),
            is_node=True,
            is_interface=False,
        )
        node_parts.extend(prop_str.splitlines())

    # body
    init_str = _generate_init(node_cls)
    node_parts.append(init_str)
    equals_str = _generate_equals(node_cls)
    node_parts.append(equals_str)
    hash_str = _generate_hash(node_cls)
    node_parts.append(hash_str)
    validate_str = _generate_validate(node_cls)
    node_parts.append(validate_str)
    to_ref_str = _generate_to_ref(node_cls)
    node_parts.append(to_ref_str)
    path_str = _generate_path(node_cls)
    node_parts.append(path_str)

    # class
    trait_classes: list[type[NodeBase]] = [
        TRAIT_CLASS_BY_TYPE[trait_type] for trait_type in definition.traits
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
    return node_str.strip()


def _get_definition_dependencies(cls: type[BuiltinObjectBase]) -> dict[str, Definition]:
    """Get the dependencies of a definition."""
    dependencies: dict[str, Definition] = {}

    # base classes
    if issubclass(cls, (Trait, NodeBase)):
        for trait_type in cls.__traits__:
            trait_cls = TRAIT_CLASS_BY_TYPE[trait_type]
            dependencies[trait_cls.__name__] = TRAIT_DEFINITION_BY_TYPE[trait_type]

    # properties
    for prop in _get_properties(cls):
        if prop.scalar_type == ScalarType.NODE_REFERENCE:
            node_types = get_node_types(prop.node_types)
            for node_type in node_types or ():
                node_cls = NODE_CLASS_BY_TYPE[node_type]
                dependencies[node_cls.__name__] = NODE_DEFINITION_BY_TYPE[node_type]
        elif prop.scalar_type == ScalarType.STRUCT:
            assert prop.struct_type is not None, f"no struct_type for {prop!r}"
            struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
            dependencies[struct_cls.__name__] = STRUCT_DEFINITION_BY_TYPE[prop.struct_type]
        elif prop.scalar_type == ScalarType.ENUM:
            assert prop.enum_type is not None, f"no enum_type for {prop!r}"
            enum_cls = ENUM_CLASS_BY_TYPE[prop.enum_type]
            dependencies[enum_cls.__name__] = ENUM_DEFINITION_BY_TYPE[prop.enum_type]

    return dependencies


def _generate_definition(definition: Definition) -> TypescriptDefinition:
    """Generate a Typescript definition."""
    if isinstance(definition, EnumDefinition):
        kind = "ENUM"
        cls = ENUM_CLASS_BY_TYPE[definition.type]
        definition_str = _generate_enum(definition)
        name = definition.name
        dependencies = EMPTY_DICT
    elif isinstance(definition, StructDefinition):
        kind = "STRUCT"
        cls = STRUCT_CLASS_BY_TYPE[definition.type]
        definition_str = _generate_struct(definition)
        name = definition.name
        dependencies = _get_definition_dependencies(cast(type[BuiltinObjectBase], cls))
    elif isinstance(definition, TraitDefinition):
        kind = "TRAIT"
        cls = TRAIT_CLASS_BY_TYPE[definition.type]
        definition_str = _generate_trait(definition)
        name = definition.alias
        dependencies = _get_definition_dependencies(cast(type[BuiltinObjectBase], cls))
    elif isinstance(definition, NodeDefinition):
        kind = "NODE"
        cls = NODE_CLASS_BY_TYPE[definition.type]
        definition_str = _generate_node(definition)
        name = definition.name
        dependencies = _get_definition_dependencies(cast(type[BuiltinObjectBase], cls))
    else:
        assert_never(definition)

    source_definition = TypescriptDefinition(
        name=name,
        cls=cls,
        module=cls.__module__,
        kind=kind,
        id=definition.id,
        definition=definition,
        definition_str=definition_str,
        dependencies=dependencies,
    )
    return source_definition


DEFINITION_START_PATTERN = re.compile(r"/\* ==== DESTACK_GENERATED_START:([^:]+):([^=]+) ==== \*/")
DEFINITION_CUSTOM_START_PATTERN = re.compile(r"/\* ==== DESTACK_GENERATED_CUSTOM_START ==== \*/")
DEFINITION_END_PATTERN = re.compile(r"/\* ==== DESTACK_GENERATED_END:([^:]+):([^=]+) ==== \*/")


def _generate_file(file: TypescriptFile) -> str:
    """Generate the contents of a managed TypescriptFile, merging with existing contents if present."""
    existing_content = file.existing_str or ""

    # parse existing content into blocks
    blocks: list[TypescriptDefinitionBlock | TypescriptCodeBlock] = []
    pos = 0
    while pos < len(existing_content):
        # look for next start marker
        start_match = DEFINITION_START_PATTERN.search(existing_content, pos)
        if start_match is None:
            # no more definition blocks, add remaining content as code block
            if pos < len(existing_content):
                remaining = existing_content[pos:].strip()
                if remaining:
                    blocks.append(TypescriptCodeBlock(content=remaining))
            break

        # add any content before the start marker as a code block
        if start_match.start() > pos:
            content = existing_content[pos : start_match.start()].strip()
            if content:
                blocks.append(TypescriptCodeBlock(content=content))

        # parse the definition block
        kind = start_match.group(1)
        assert kind in ("ENUM", "STRUCT", "TRAIT", "NODE"), f"invalid kind: {kind} in {file.path}"
        kind = cast(Kind, kind)
        id_str = start_match.group(2).strip()
        id_int = int(id_str)

        # find the corresponding end marker
        end_match = DEFINITION_END_PATTERN.search(existing_content, start_match.end())
        if end_match is None or end_match.group(1) != kind or end_match.group(2).strip() != id_str:
            raise ValueError(f"no matching end marker for {kind}:{id_str}")

        # look for custom content within the definition block
        custom_content = ""
        custom_match = DEFINITION_CUSTOM_START_PATTERN.search(existing_content, start_match.end())
        if custom_match is not None and custom_match.start() < end_match.start():
            custom_start = custom_match.end()
            custom_end = end_match.start()
            custom_content = existing_content[custom_start:custom_end].strip()

        # add block
        block = TypescriptDefinitionBlock(kind=kind, id=id_int, custom_content=custom_content)
        blocks.append(block)
        pos = end_match.end()

    # build file by updating existing definition blocks and adding new ones
    file_parts: list[str] = []
    seen_definitions: set[tuple[Kind, int]] = set()
    for block in blocks:
        if isinstance(block, TypescriptCodeBlock):
            file_parts.append(block.content)
        elif isinstance(block, TypescriptDefinitionBlock):
            key = (block.kind, block.id)
            definition = file.get_definition(block.kind, block.id)
            if definition is None:
                raise RuntimeError(f"unknown definition: {key} in {file.path}")
            new_block = f"{MARKER_START.format(kind=definition.kind, id=definition.id)}\n"
            new_block += f"{definition.definition_str}\n"
            if block.custom_content:
                new_block += f"{MARKER_CUSTOM_START}\n{block.custom_content}\n"
            new_block += f"{MARKER_END.format(kind=definition.kind, id=definition.id)}"
            file_parts.append(new_block)
            seen_definitions.add(key)
        else:
            assert_never(block)

    # add any new definitions at the end
    for key, definition in file.definitions.items():
        if key not in seen_definitions:
            new_block = f"{MARKER_START.format(kind=definition.kind, id=definition.id)}\n"
            new_block += f"{definition.definition_str}\n"
            new_block += f"{MARKER_END.format(kind=definition.kind, id=definition.id)}"
            file_parts.append(new_block)

    # add imports to the top (will be auto-merged by linter)
    import_parts: list[str] = []
    imports_by_module: dict[str, list[str]] = defaultdict(list)
    for dependency in file.dependencies.values():
        if dependency.module != file.module:
            imports_by_module[dependency.module].append(dependency.name)
    for module, imports in imports_by_module.items():
        import_path = module.split(".", 1)[-1].replace(".", "/") + ".ts"
        import_parts.append(f"import type {{ {', '.join(imports)} }} from '@/{import_path}';")
    file_parts.insert(0, "\n".join(import_parts))

    new_str = "\n\n".join(file_parts)
    return new_str


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
    for trait_type, trait_cls in TRAIT_CLASS_BY_TYPE.items():
        definition = _generate_definition(TRAIT_DEFINITION_BY_TYPE[trait_type])
        definition_by_cls[trait_cls] = definition
    for node_type, node_cls in NODE_CLASS_BY_TYPE.items():
        definition = _generate_definition(NODE_DEFINITION_BY_TYPE[node_type])
        definition_by_cls[node_cls] = definition

    # organize definitions into files
    definitions_by_name: dict[str, TypescriptDefinition] = {}
    definitions_by_module: dict[str, list[TypescriptDefinition]] = defaultdict(list)
    for cls, definition in definition_by_cls.items():
        module = cls.__module__
        definitions_by_module[module].append(definition)
        existing_definition = definitions_by_name.get(definition.name)
        if existing_definition is not None:
            raise RuntimeError(
                f"duplicate definition name: {definition.name} "
                f"({existing_definition.module}.{existing_definition.cls.__name__} vs "
                f"{definition.module}.{definition.cls.__name__})"
            )
        definitions_by_name[definition.name] = definition

    # generate files
    files_by_module: dict[str, TypescriptFile] = {}
    for module, definitions in definitions_by_module.items():
        # path
        target_path = Path(GENERATION_PATH) / (module.split(".", 2)[-1].replace(".", "/") + ".ts")
        existing_file_str = target_path.read_text() if target_path.exists() else None

        # definitions
        file_definitions_by_name: dict[str, TypescriptDefinition] = {
            definition.definition.name: definition for definition in definitions
        }
        file_dependencies_by_name: dict[str, TypescriptDefinition] = {}
        for definition in definitions:
            for dependency_name in definition.dependencies:
                if dependency_name not in definitions_by_name:
                    raise RuntimeError(f"missing dependency: {dependency_name}")
                file_dependencies_by_name[dependency_name] = definitions_by_name[dependency_name]

        # file
        file = TypescriptFile(
            name=module,
            module=module,
            path=target_path,
            definitions=file_definitions_by_name,
            dependencies=file_dependencies_by_name,
            existing_str=existing_file_str,
        )
        file.new_str = _generate_file(file)
        files_by_module[module] = file

    for file in files_by_module.values():
        print("=" * 80)
        print(file.path)
        print("=" * 80)
        print(file.new_str)
        print("=" * 80)
