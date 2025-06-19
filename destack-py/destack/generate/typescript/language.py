import re
import textwrap
from collections import defaultdict
from pathlib import Path
from typing import Any, assert_never, cast

from destack.language import (
    EMPTY_DICT,
    NODE_TYPES,
    UNSET,
    BuiltinObjectBase,
    Enum,
    EnumDefinition,
    IntoType,
    Node,
    NodeBase,
    NodeDefinition,
    NodeType,
    PrimitiveType,
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
    TypescriptImport,
    TypescriptImportBlock,
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


def _generate_property_scalar_type(prop: IntoType, as_ptr: bool = True) -> str:
    """Generate a scalar property Typescript type annotation."""
    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        if as_ptr:
            return "NodeReference"
        else:
            node_types = get_node_types(prop.node_types)
            node_classes = tuple(NODE_CLASS_BY_TYPE[node_type] for node_type in node_types or ())
            if node_classes and len(node_classes) < len(NODE_TYPES):
                return " | ".join(node_cls.__name__ for node_cls in node_classes)
            else:
                return "Node"
    elif prop.scalar_type == ScalarType.ENUM:
        assert prop.enum_type is not None, f"no enum_type for {prop!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[prop.enum_type]
        return f"{enum_cls.__name__}"
    elif prop.scalar_type == ScalarType.PRIMITIVE:
        assert prop.primitive_type is not None, f"no primitive_type for {prop!r}"
        return TYPESCRIPT_TYPE_BY_PRIMITIVE_TYPE[prop.primitive_type]
    elif prop.scalar_type == ScalarType.STRUCT:
        assert prop.struct_type is not None, f"no struct_type for {prop!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
        return f"{struct_cls.__name__}"
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        raise ValueError(f"unsupported scalar_type: {prop!r}")
    else:
        assert_never(prop.scalar_type)


def _generate_property_type(prop: Property, as_ptr: bool = True) -> str:
    """Generate a property Typescript type annotation."""
    type_str = _generate_property_scalar_type(prop, as_ptr=as_ptr)
    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_optional:
            type_str = f"{type_str} | null"
    elif prop.cardinality == TypeCardinality.LIST:
        type_str = f"Array<{type_str}>"
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key_type for {prop!r}"
        key_type_str = _generate_property_scalar_type(prop.key_type)
        type_str = f"Map<{key_type_str}, {type_str}>"
    else:
        assert_never(prop.cardinality)
    return type_str


def _generate_value(type: IntoType, value: Any) -> str:
    """Generate a Typescript value literal."""
    if type.scalar_type == ScalarType.PRIMITIVE:
        if type.primitive_type == PrimitiveType.BOOLEAN:
            return "true" if value else "false"
        elif type.primitive_type in (
            PrimitiveType.INT16,
            PrimitiveType.INT32,
            PrimitiveType.INT64,
            PrimitiveType.FLOAT32,
            PrimitiveType.FLOAT64,
        ):
            return str(value)
        elif type.primitive_type == PrimitiveType.STRING:
            return f'"{value}"'
        else:
            raise ValueError(f"unsupported primitive type: {type.primitive_type!r}")
    elif type.scalar_type == ScalarType.ENUM:
        if type.cardinality == TypeCardinality.SCALAR:
            assert type.enum_type is not None, f"no enum_type for {type!r}"
            enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
            return f"{enum_cls.__name__}.{value.name}"
        else:
            raise ValueError(f"unsupported enum cardinality: {type.cardinality!r}")
    else:
        raise ValueError(f"unsupported value type: {type.scalar_type!r}")


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
        ptr_type_str = _generate_property_type(prop, as_ptr=True)
        ptr_prop_name = f"{ts_name}Ptr"
        ptr_prop_str = f"{ptr_prop_name}: {ptr_type_str}"
        node_type_str = _generate_property_scalar_type(prop, as_ptr=False)
        if prop.is_optional:
            node_type_str = f"{node_type_str} | null"

        # node getter/setter
        if is_interface:
            # interface declaration
            node_getter_str = f"get {ts_name}(): {node_type_str}"
            if not is_readonly:
                node_setter_str = f"set {ts_name}(value: {node_type_str})"
                node_prop_str = f"{node_getter_str}\n{node_setter_str}"
            else:
                node_prop_str = node_getter_str
        else:
            # actual getter/setter
            if is_node:
                node_getter_str = f"""\
get {ts_name}(): {node_type_str} | null {{
    const nodePtr: NodeReference | null = this.{ptr_prop_name};
    if (nodePtr !== null) {{
        return this._supergraph.get(nodePtr.id) as {node_type_str} | null;
    }}
    return null;
}}
"""
                if not is_readonly:
                    node_setter_str = f"""\
set {ts_name}(value: {node_type_str}) {{
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
get {ts_name}(): {node_type_str} {{
    const nodePtr: NodeReference | null = this.{ptr_prop_name};
    if (nodePtr !== null) {{
        if (this._supergraph === null) {{
            return null;
        }}
        return this._supergraph.get(nodePtr.id) as {node_type_str};
    }}
    return null;
}}
"""
                if not is_readonly:
                    node_setter_str = f"""\
set {ts_name}(value: {node_type_str}) {{
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

        return f"{node_prop_str};\n{ptr_prop_str}"

    else:
        type_str = _generate_property_type(prop)
        node_prop_str = f"{ts_name}: {type_str}"
        if is_readonly:
            node_prop_str = f"readonly {node_prop_str}"
        if node_prop_str.count("\n") < 1:
            node_prop_str = f"{node_prop_str};"
        return node_prop_str


def _generate_init(cls: type[BuiltinObjectBase]) -> str:
    """
    Generate a Typescript constructor/create method.
    Constructor is private with direct assignments, no defaults or anything else.
    Create is a public factory with convenient conversion, defaults and validation.
    """

    properties = {p.name: p for p in _get_properties(cls)}

    # constructor
    constructor_header_parts: list[str] = []
    constructor_super_parts: list[str] = []
    constructor_body_parts: list[str] = []

    # constructor main properties
    for prop in properties.values():
        if prop.ptr_prop is not None:
            prop = prop.ptr_prop
        ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)
        prop_header_str = f"{ts_name}: {_generate_property_type(prop)}"
        constructor_header_parts.append(prop_header_str)
        prop_body_str = f"this.{ts_name} = {ts_name};"
        constructor_body_parts.append(prop_body_str)

    # constructor extra runtime properties
    if issubclass(cls, Node):
        runtime_props = [
            ("_session", "Session"),
            ("_supergraph", "Supergraph"),
            ("_graph", "Graph"),
            ("_connection", "QueryConnection | null"),
        ]
        constructor_super_parts.append("id")  # already in regular properties
    else:
        runtime_props = [
            ("_supergraph", "Supergraph | null"),
        ]
    for prop_name, prop_type in runtime_props:
        constructor_header_parts.append(f"{prop_name}: {prop_type}")
        constructor_super_parts.append(prop_name)

    # assemble constructor
    constructor_header_str = ",\n".join(constructor_header_parts)
    constructor_super_str = ", ".join(constructor_super_parts)
    constructor_body_str = "\n".join(constructor_body_parts)
    constructor_str = f"""\
constructor(
{textwrap.indent(constructor_header_str, "  ")}
) {{
  super({constructor_super_str});
{textwrap.indent(constructor_body_str, "  ")}
}}
"""

    # create
    create_header_parts: list[str] = []
    create_body_parts: list[str] = []
    create_constructor_parts: list[str] = []

    # create main properties
    for prop in properties.values():
        if prop.is_managed:
            continue  # ignore

        ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)
        is_required = (
            prop.is_required
            and prop.default is UNSET
            and prop.default_factory is None
            and prop.cardinality == TypeCardinality.SCALAR
        )
        if prop.scalar_type == ScalarType.NODE_REFERENCE:
            # can be passed either as Node or NodeReference
            node_type_str = _generate_property_scalar_type(prop, as_ptr=False)
            ptr_type_str = _generate_property_scalar_type(prop, as_ptr=True)
            type_str = f"{node_type_str} | {ptr_type_str}"
            if prop.is_optional:
                type_str = f"{type_str} | null"
            create_constructor_parts.append(
                f"options.{ts_name} != null ? (options.{ts_name}.metatype == StructType.NODE_REFERENCE ? options.{ts_name} : options.{ts_name}.toRef()) : null"
            )
        else:
            # can be passed as value
            type_str = _generate_property_type(prop)
            if prop.default is not UNSET and prop.default is not None:
                create_constructor_parts.append(
                    f"options.{ts_name} ?? {_generate_value(prop, prop.default)}"
                )
            elif prop.default_factory is not None:
                create_constructor_parts.append(f"options.{ts_name}")  # nocheckin
            elif prop.cardinality == TypeCardinality.LIST:
                create_constructor_parts.append(f"options.{ts_name} ?? []")
            elif prop.cardinality == TypeCardinality.MAP:
                create_constructor_parts.append(f"options.{ts_name} ?? new Map()")
            elif not is_required:
                create_constructor_parts.append(f"options.{ts_name} ?? null")
            else:
                create_constructor_parts.append(f"options.{ts_name}")

        if is_required:
            create_header_parts.append(f"{ts_name}: {type_str}")
        else:
            create_header_parts.append(f"{ts_name}?: {type_str}")

    create_header_parts.append("_session?: Session | null")  # noqa: FURB113
    create_header_parts.append("_supergraph?: Supergraph | null")
    if issubclass(cls, Node):
        create_header_parts.append("_graph?: Graph | null")  # noqa: FURB113
        create_header_parts.append("_connection?: QueryConnection | null")
        create_constructor_parts.append("session")
        create_constructor_parts.append("supergraph")
        create_constructor_parts.append("options._graph")
        create_constructor_parts.append("options._connection")
    else:
        create_constructor_parts.append("supergraph")

    create_body_parts.append("const session = options._session ?? ACTIVE_SESSION.get();")  # noqa: FURB113
    create_body_parts.append("const supergraph = options._supergraph ?? session.supergraph;")

    # assemble create
    create_header_str = ",\n".join(create_header_parts)
    create_body_str = "\n".join(create_body_parts)
    create_constructor_str = ",\n".join(create_constructor_parts)
    create_str = f"""\
static create(options: {{
{textwrap.indent(create_header_str, "  ")}
}}): {cls.__name__} {{
{textwrap.indent(create_body_str, "  ")}
  return new {cls.__name__}(
{textwrap.indent(create_constructor_str, "    ")}
  );
}}
"""

    # assemble
    init_str = "\n\n".join((constructor_str, create_str)).strip()
    return init_str


def _generate_equals(cls: type[BuiltinObjectBase]) -> str:
    """Generate a Typescript equals method."""
    equals_str = """\
equals(other: any): boolean {
  throw new Error("Not implemented");
}
"""
    return equals_str.strip()


def _generate_hash(cls: type[BuiltinObjectBase]) -> str:
    """Generate a Typescript hash method."""
    hash_str = """\
hash(): number {
  throw new Error("Not implemented");
}
"""
    return hash_str.strip()


def _generate_validate(cls: type[BuiltinObjectBase]) -> str:
    """Generate a Typescript validate method."""
    validate_str = """\
validate(): void {
  throw new Error("Not implemented");
}
"""
    return validate_str.strip()


def _generate_to_ref(cls: type["Node"]) -> str:
    """Generate a Typescript toRef method."""

    node_type = cls.metatype
    if node_type == NodeType.SPACE:
        ref_impl = f"""\
__toRef__(): NodeReference {{
  return new NodeReference(NodeType.{node_type.name}, this.id, this.id, null, this._supergraph);
}}
"""
    elif TraitType.CUSTOM_NODE in cls.__traits__:
        ref_impl = f"""\
__toRef__(): NodeReference {{
  return new NodeReference(NodeType.{node_type.name}, this.id, this.spacePtr?.id ?? null, this.definitionPtr?.id ?? null, this._supergraph);
}}
"""
    elif TraitType.SPATIAL in cls.__traits__:
        ref_impl = f"""\
__toRef__(): NodeReference {{
  return new NodeReference(NodeType.{node_type.name}, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
}}
"""
    else:
        ref_impl = f"""\
__toRef__(): NodeReference {{
  return new NodeReference(NodeType.{node_type.name}, this.id, null, null, this._supergraph);
"""
    return ref_impl.strip()


def _generate_path(cls: type[Node]) -> str:
    """Generate a Typescript path method."""
    # Node._path_key
    if "slug" in cls.__properties__:
        if "name" in cls.__properties__:
            path_key_str = "this.slug ?? this.name"
        else:
            path_key_str = f'this.slug ?? "{cls.__name__}[id={{this.id}}]"'
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
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {{
        pathParts.push(node._pathKey);
        node = node.parent;
    }}
    if (!this._isAttached) {{
        pathParts.push("<detached>");
    }}
    return pathParts.reverse().join("/");
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
    prop_parts: list[str] = []
    for prop in _get_properties(struct_cls):
        prop_str = _generate_property(
            prop,
            is_readonly=definition.is_frozen,
            is_node=False,
            is_interface=False,
        )
        prop_parts.append(prop_str)
    struct_parts.append("\n".join(prop_parts))

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
export class {definition.name} extends {"StructFrozen" if definition.is_frozen else "Struct"} {{
{textwrap.indent("\n\n".join(struct_parts), "  ")}
}}"""
    return struct_str.strip()


def _generate_trait(definition: TraitDefinition) -> str:
    """Generate a Typescript Trait definition."""

    trait_cls = TRAIT_CLASS_BY_TYPE[definition.type]
    trait_parts: list[str] = []

    # properties
    prop_parts: list[str] = []
    for prop in _get_properties(trait_cls):
        if prop.name == "parent":
            continue  # ignore parent for traits
        prop_str = _generate_property(
            prop,
            is_readonly=_is_property_readonly(prop),
            is_node=False,
            is_interface=True,
        )
        prop_parts.append(prop_str)
    trait_parts.append("\n".join(prop_parts))

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
export interface {definition.alias}{implements_str} {{
{textwrap.indent("\n".join(trait_parts), "  ")}
}}"""
    return trait_str.strip()


def _generate_node(definition: NodeDefinition) -> str:
    """Generate a Typescript Node definition."""

    node_cls = NODE_CLASS_BY_TYPE[definition.type]
    node_parts: list[str] = []

    # properties
    prop_parts: list[str] = []
    for prop in _get_properties(node_cls):
        prop_str = _generate_property(
            prop,
            is_readonly=_is_property_readonly(prop),
            is_node=True,
            is_interface=False,
        )
        prop_parts.extend(prop_str.splitlines())
    node_parts.append("\n".join(prop_parts))

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
{textwrap.indent("\n\n".join(node_parts), "  ")}
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

    # parse existing content into blocks
    existing_content = file.existing_str or ""
    lines = existing_content.split("\n")
    blocks: list[TypescriptDefinitionBlock | TypescriptCodeBlock | TypescriptImportBlock] = []
    char_pos = 0

    # imports
    existing_import_lines: list[str] = []
    existing_imports: list[TypescriptImport] = []

    for line in lines:
        stripped = line.strip()
        if not stripped:
            continue
        if not stripped.startswith("import "):
            break

        # parse import
        existing_import_lines.append(line)
        is_type = "import type " in stripped

        # extract path
        path_match = re.search(r'from\s+["\']([^"\']+)["\']', stripped)
        if not path_match:
            continue
        path = path_match.group(1)

        # extract names
        names: list[str] = []
        if "{" in stripped and "}" in stripped:  # named imports
            names_match = re.search(r"\{\s*([^}]+)\s*\}", stripped)
            if names_match:
                names_str = names_match.group(1)
                names = [name.strip() for name in names_str.split(",") if name.strip()]
        elif " * as " in stripped:  # namespace import
            namespace_match = re.search(r"\*\s+as\s+(\w+)", stripped)
            if namespace_match:
                names = [namespace_match.group(1)]
        else:  # default import
            default_match = re.search(r"import\s+(?:type\s+)?(\w+)", stripped)
            if default_match:
                names = [default_match.group(1)]

        import_ = TypescriptImport(content=line, path=path, is_type=is_type, names=names)
        existing_imports.append(import_)
    char_pos = sum(len(line) + 1 for line in existing_import_lines)  # +1 for newline

    # add import block
    import_block = TypescriptImportBlock(
        content="\n".join(existing_import_lines), imports=existing_imports
    )
    blocks.append(import_block)

    # regular blocks
    while char_pos < len(existing_content):
        # look for next start marker
        start_match = DEFINITION_START_PATTERN.search(existing_content, char_pos)
        if start_match is None:
            # no more definition blocks, add remaining content as code block
            if char_pos < len(existing_content):
                remaining = existing_content[char_pos:].strip()
                if remaining:
                    blocks.append(TypescriptCodeBlock(content=remaining))
            break

        # add any content before the start marker as a code block
        if start_match.start() > char_pos:
            content = existing_content[char_pos : start_match.start()].strip()
            if content:
                blocks.append(TypescriptCodeBlock(content=content))

        # parse the definition block
        kind = start_match.group(1)
        assert kind in ("ENUM", "STRUCT", "TRAIT", "NODE"), f"invalid kind: {kind} in {file.path}"
        kind = cast(Kind, kind)
        id_str = start_match.group(2).strip()
        id = int(id_str)

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
        block = TypescriptDefinitionBlock(kind=kind, id=id, custom_content=custom_content)
        blocks.append(block)
        char_pos = end_match.end()

    # build file by updating existing definition blocks and adding new ones
    file_parts: list[str] = []
    seen_definitions: set[tuple[Kind, int]] = set()
    for block in blocks:
        if isinstance(block, TypescriptCodeBlock):
            file_parts.append(block.content)
        elif isinstance(block, TypescriptDefinitionBlock):
            key = (block.kind, block.id)
            if key in seen_definitions:
                continue  # duplicate block (for some reason)
            definition = file.get_definition(block.kind, block.id)
            if definition is None:
                raise RuntimeError(
                    f"unknown definition: {key} in {file.path} (defines: {list(file.definitions.keys())})"
                )
            new_block = f"{MARKER_START.format(kind=definition.kind, id=definition.id)}\n"
            new_block += f"{definition.definition_str}\n"
            if block.custom_content:
                new_block += f"{MARKER_CUSTOM_START}\n{block.custom_content}\n"
            new_block += f"{MARKER_END.format(kind=definition.kind, id=definition.id)}"
            file_parts.append(new_block)
            seen_definitions.add(key)
        elif isinstance(block, TypescriptImportBlock):
            pass  # re-assembled below
        else:
            assert_never(block)

    # add any new definitions at the end
    for definition in file.definitions.values():
        key = (definition.kind, definition.id)
        if key not in seen_definitions:
            new_block = f"{MARKER_START.format(kind=definition.kind, id=definition.id)}\n"
            new_block += f"{definition.definition_str}\n"
            new_block += f"{MARKER_END.format(kind=definition.kind, id=definition.id)}"
            file_parts.append(new_block)

    # add imports to the top (will be auto-merged by linter)
    inner_file_content = "\n\n".join(file_parts[1:])
    imports = {
        "NodeType",
        "StructType",
        "EnumType",
        "BuiltinObject",
        "Struct",
        "StructFrozen",
        "Node",
        "NodeReference",
        "Graph",
        "Supergraph",
        "Session",
        "QueryConnection",
        *(d.name for d in file.dependencies.values() if d.module != file.module),
    }
    # remove any imports that are already defined in this file
    imports.difference_update(definition.name for definition in file.definitions.values())
    import_parts: list[str] = [f"import {{ {', '.join(imports)} }} from '@/language';"]
    if "Temporal" in inner_file_content and not any(
        "Temporal" in import_.content for import_ in import_block.imports
    ):
        # until Temporal ships natively
        import_parts.append("import { Temporal } from 'temporal-polyfill';")
    for import_ in import_block.imports:
        if not import_.path.startswith("@/language"):
            import_parts.append(import_.content)
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

    # write files
    for file in files_by_module.values():
        assert file.new_str, f"empty {file.path}"
        file.path.parent.mkdir(parents=True, exist_ok=True)
        file.path.write_text(file.new_str)

    # write index files
    module_paths = list({file.path.parent for file in files_by_module.values()})
    module_paths.append(Path(GENERATION_PATH))
    for module_path in sorted(module_paths):
        # generate index.ts file that re-exports every subfile/subfolder
        index_path = module_path / "index.ts"
        index_path.parent.mkdir(parents=True, exist_ok=True)
        index_lines = []
        # collect all .ts files in this directory (excluding index.ts itself)
        ts_files = [f for f in module_path.glob("*.ts") if f.name != "index.ts"]
        for ts_file in sorted(ts_files):
            module_name = ts_file.stem
            index_lines.append(f"export * from './{module_name}';")
        # collect all subdirectories that contain .ts files
        for subdir in sorted(module_path.iterdir()):
            if subdir.is_dir() and any(subdir.rglob("*.ts")):
                subdir_name = subdir.name
                index_lines.append(f"export * from './{subdir_name}';")

        index_content = "\n".join(index_lines) + "\n"
        index_path.write_text(index_content)
