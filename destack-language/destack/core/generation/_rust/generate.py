import collections
import textwrap
from collections.abc import Sequence
from typing import TYPE_CHECKING, Optional, assert_never

from destack.core import (
    EMPTY_LIST,
    VERSION,
    EnumDefinition,
    ModuleDefinition,
    ModuleType,
    NodeDefinition,
    PrimitiveType,
    RuntimeLanguage,
    ScalarType,
    SchemaDefinition,
    StringCasing,
    StructDefinition,
    Type,
    TypeCardinality,
    to_casing,
)
from destack.registry import (
    ENUM_DEFINITION_BY_TYPE,
    STRUCT_DEFINITION_BY_TYPE,
)

from .core import (
    RustAttribute,
    RustFile,
    RustImport,
    RustItemScope,
    RustManagedItem,
    RustManagedType,
    RustModDeclaration,
    RustVisibility,
    ecsape_rust_identifier,
    local_path_to_source_path,
    source_path_to_local_path,
)

if TYPE_CHECKING:
    from destack import MethodDefinition

#
# Types
#

RUST_PRIMITIVE_TYPES: dict[PrimitiveType, tuple[str, Optional[tuple[str, str]]]] = {
    PrimitiveType.BOOLEAN: ("bool", None),
    # integer
    PrimitiveType.INT8: ("i8", None),
    PrimitiveType.INT16: ("i16", None),
    PrimitiveType.INT32: ("i32", None),
    PrimitiveType.INT64: ("i64", None),
    PrimitiveType.INT128: ("i128", None),
    PrimitiveType.UINT8: ("u8", None),
    PrimitiveType.UINT16: ("u16", None),
    PrimitiveType.UINT32: ("u32", None),
    PrimitiveType.UINT64: ("u64", None),
    PrimitiveType.UINT128: ("u128", None),
    # time
    PrimitiveType.DATETIME: ("DateTime", ("destack_time", "DateTime")),
    PrimitiveType.DATE: ("Date", ("destack_time", "Date")),
    PrimitiveType.TIME: ("Time", ("destack_time", "Time")),
    PrimitiveType.TIMESTAMP: ("Timestamp", ("destack_time", "Timestamp")),
    PrimitiveType.DURATION: ("Duration", ("destack_time", "Duration")),
    # float
    PrimitiveType.FLOAT32: ("f32", None),
    PrimitiveType.FLOAT64: ("f64", None),
    # string
    PrimitiveType.CHARACTER: ("char", None),
    PrimitiveType.STRING: ("String", None),
    PrimitiveType.UUID: ("Uuid", ("destack_uuid", "Uuid")),
    PrimitiveType.JSON: ("JsonValue", ("destack_json", "JsonValue")),
}


def _generate_type_scalar(type: Type, dependencies: set[str]) -> str:
    """Map a Destack type to its Rust declaration."""
    assert type.scalar_type is not None, f"no scalar type for {type!r}"

    # primitive
    if type.scalar_type == ScalarType.PRIMITIVE:
        assert type.primitive_type is not None, f"no primitive type for {type!r}"
        if type.primitive_type == PrimitiveType.NONE:
            return "() /* TODO */ "
        primitive_mapping = RUST_PRIMITIVE_TYPES.get(type.primitive_type)
        assert primitive_mapping is not None, f"no Rust type for {type!r}"
        type_str, rust_dep = primitive_mapping
        if rust_dep:
            dependencies.add(rust_dep[1])
        return type_str

    # enum
    elif type.scalar_type == ScalarType.ENUM:
        assert type.enum_type is not None, f"no enum type for {type!r}"
        enum_definition = ENUM_DEFINITION_BY_TYPE[type.enum_type]
        dependencies.add(enum_definition.name)
        return enum_definition.name

    # node
    elif type.scalar_type in (
        ScalarType.NODE,
        ScalarType.NODE_RAW,
        ScalarType.NODE_IDENTITY,
        ScalarType.NODE_SPATIAL,
        ScalarType.NODE_TEMPORAL,
    ):
        return "i64 /* TODO */ "

    # struct
    elif type.scalar_type == ScalarType.STRUCT:
        assert type.struct_type is not None, f"no struct type for {type!r}"
        struct_definition = STRUCT_DEFINITION_BY_TYPE[type.struct_type]
        dependencies.add(struct_definition.name)
        return struct_definition.name

    # handle
    elif type.scalar_type == ScalarType.HANDLE:
        raise NotImplementedError(f"cannot generate type for {type!r}")

    # union
    elif type.scalar_type == ScalarType.UNION:
        raise NotImplementedError(f"cannot generate type for {type!r}")

    #
    else:
        assert_never(type.scalar_type)


def _generate_type(type: Type, dependencies: set[str]) -> str:
    """Map a Destack scalar type to its Rust declaration."""

    # scalar
    if type.cardinality == TypeCardinality.SCALAR:
        type_str = _generate_type_scalar(type, dependencies)
    # list
    elif type.cardinality == TypeCardinality.LIST:
        assert type.value_type is not None, f"no value type for {type!r}"
        scalar_type_str = _generate_type_scalar(type.value_type, dependencies)
        type_str = f"Vec<{scalar_type_str}>"
    # tuple
    elif type.cardinality == TypeCardinality.TUPLE:
        assert type.element_types is not None, f"no element types for {type!r}"
        element_types_str = ", ".join(
            _generate_type_scalar(element_type, dependencies) for element_type in type.element_types
        )
        type_str = f"({element_types_str})"
    # map
    elif type.cardinality == TypeCardinality.MAP:
        assert type.key_type is not None, f"no key type for {type!r}"
        assert type.value_type is not None, f"no value type for {type!r}"
        key_type_str = _generate_type_scalar(type.key_type, dependencies)
        value_type_str = _generate_type_scalar(type.value_type, dependencies)
        type_str = f"HashMap<{key_type_str}, {value_type_str}>"
        dependencies.add("HashMap")
    #
    else:
        assert_never(type.cardinality)

    # wrap in Option if not required
    if not type.is_required:
        type_str = f"Option<{type_str}>"

    return type_str


#
# Functions
#


def _generate_method_definition(
    object: ModuleDefinition | StructDefinition | NodeDefinition,
    method: "MethodDefinition",
) -> RustManagedItem:
    """Map a Method to its Rust definition item."""

    object_key = object.name
    inner_key = method.name
    dependencies: set[str] = set()

    # arguments
    arguments_parts: list[str] = []
    for prop in method.input_properties:
        arguments_parts.append(f"{prop.name}: {_generate_type(prop.type, dependencies)}")
    arguments_str = ", ".join(arguments_parts)
    # return type
    if method.output_property:
        return_type_str = _generate_type(method.output_property.type, dependencies)
    else:
        return_type_str = None
    # signature
    if return_type_str:
        signature = f"{method.name}({arguments_str}) -> {return_type_str}"
    else:
        signature = f"{method.name}({arguments_str})"

    # outer content
    outer_content = f"""\
{_generate_doc_comment(object_key, method.description)}
pub fn {signature} {{
    todo!("{"nocheck" + "in"}: implement {method.name}")
}}
"""

    item = RustManagedItem(
        type=RustManagedType.PARTIAL,
        scope=RustItemScope.BLOCK,
        object_key=object_key,
        inner_key=inner_key,
        children=EMPTY_LIST,
        outer_content=outer_content,
        inner_content=outer_content,
        dependencies=list(dependencies),
    )
    return item


#
# Struct
#


def _generate_doc_comment(object_key: str, doc: str) -> str:
    """Generate a Rust doc comment."""
    if doc:
        return "\n".join(f"/// {line.strip()}" for line in doc.splitlines() if line)
    else:
        return f"/// {object_key}"


def _generate_struct_definition(struct: StructDefinition) -> RustManagedItem:
    """Map a Struct to its Rust definition item."""

    object_key = struct.name

    # inner content
    inner_content_parts: list[str] = []
    dependencies: set[str] = set()
    for prop in struct.properties:
        if prop.is_static:
            continue
        prop_name = prop.name
        prop_type_str = _generate_type(prop.type, dependencies)
        if prop.type.struct_type == struct.type:
            # auto-box self references
            prop_type_str = f"Box<{prop_type_str}>"
        inner_content_parts.append(f"pub {ecsape_rust_identifier(prop_name)}: {prop_type_str}")
    inner_content = ",\n".join(inner_content_parts)

    # outer content
    outer_content = f"""\
{_generate_doc_comment(object_key, struct.description)}
pub struct {struct.name} {{
{textwrap.indent(inner_content, " " * 4)}
}}
"""

    item = RustManagedItem(
        type=RustManagedType.GENERATED,
        scope=RustItemScope.BLOCK,
        object_key=object_key,
        inner_key="",
        children=EMPTY_LIST,
        outer_content=outer_content,
        inner_content=inner_content,
        dependencies=list(dependencies),
    )
    return item


def _generate_struct_debug(struct: StructDefinition) -> RustManagedItem:
    """Generate a Rust impl Debug for a Struct."""

    object_key = struct.name

    # inner content
    inner_content_parts: list[str] = [
        # just the struct name for now
        f'write!(f, "{struct.name}")'
    ]
    inner_content = ",\n".join(inner_content_parts)

    # outer content
    outer_content = f"""\
impl std::fmt::Debug for {struct.name} {{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
{textwrap.indent(inner_content, " " * 8)}
    }}
}}
"""

    item = RustManagedItem(
        type=RustManagedType.GENERATED,
        scope=RustItemScope.BLOCK,
        object_key=object_key,
        inner_key="Debug",
        children=EMPTY_LIST,
        outer_content=outer_content,
        inner_content=inner_content,
        dependencies=(struct.name,),
    )
    return item


#
# Enum
#


def _generate_enum_definition(enum: EnumDefinition) -> RustManagedItem:
    """Map an Enum to its Rust definition item."""

    object_key = enum.name

    # inner content
    inner_content_parts: list[str] = []
    for option in enum.options:
        rs_name = to_casing(option.name, StringCasing.UPPER_CAMEL)
        rs_name = ecsape_rust_identifier(rs_name)
        option_declaration = f"{rs_name} = {option.id}"
        if option.description:
            option_declaration = (
                f"{_generate_doc_comment(option.name, option.description)}\n{option_declaration}"
            )
        inner_content_parts.append(option_declaration)
    inner_content = ",\n".join(inner_content_parts)

    # outer content
    outer_content = f"""\
{_generate_doc_comment(object_key, enum.description)}
pub enum {enum.name} {{
{textwrap.indent(inner_content, " " * 4)}
}}
"""

    item = RustManagedItem(
        type=RustManagedType.GENERATED,
        scope=RustItemScope.BLOCK,
        object_key=enum.name,
        inner_key="",
        children=EMPTY_LIST,
        outer_content=outer_content,
        inner_content=inner_content,
    )
    return item


def _generate_enum_debug(enum: EnumDefinition) -> RustManagedItem:
    """Generate a Rust impl Debug for an Enum."""

    object_key = enum.name

    # inner content
    inner_content_parts: list[str] = []
    for option in enum.options:
        rs_name = to_casing(option.name, StringCasing.UPPER_CAMEL)
        inner_content_parts.append(f'{enum.name}::{rs_name} => write!(f, "{option.name}"),')
    inner_content = f"""\
fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
    match self {{
{textwrap.indent("\n".join(inner_content_parts), " " * 8)}
    }}
}}"""

    # outer content
    outer_content = f"""\
impl std::fmt::Debug for {enum.name} {{
{textwrap.indent(inner_content, " " * 4)}
}}
"""

    item = RustManagedItem(
        type=RustManagedType.GENERATED,
        scope=RustItemScope.BLOCK,
        object_key=object_key,
        inner_key="Debug",
        children=EMPTY_LIST,
        outer_content=outer_content,
        inner_content=inner_content,
        dependencies=(enum.name,),
    )
    return item


#
# Module
#


def _get_imports(items: Sequence[RustManagedItem]) -> Sequence[RustImport]:
    """Get all imports from a list of items."""
    # collect dependencies
    dependencies: set[str] = set()
    for item in items:
        if item.dependencies:
            dependencies.update(item.dependencies)
    # discard self
    dependencies.difference_update(item._key for item in items)
    # generate imports
    if dependencies:
        imp = RustImport(
            source="crate",
            imports=list(dependencies),
            is_internal=True,
            visibility=RustVisibility.PRIVATE,
            is_glob=False,
        )
        return (imp,)
    else:
        return ()


def _generate_object_module(
    module: ModuleDefinition,
) -> tuple[list[RustManagedItem], list[RustManagedItem]]:
    """Generate the partial and generated Rust items for an object module."""

    partial_items: list[RustManagedItem] = []
    gen_items: list[RustManagedItem] = []

    # generate structs
    for struct_type in module.struct_types:
        struct_def = STRUCT_DEFINITION_BY_TYPE[struct_type]
        if struct_def.is_abstract:
            continue  # ignore abstract structs
        struct_item = _generate_struct_definition(struct_def)
        partial_items.append(struct_item)
        struct_debug_item = _generate_struct_debug(struct_def)
        gen_items.append(struct_debug_item)

    # generate enums
    for enum_type in module.enum_types:
        enum_def = ENUM_DEFINITION_BY_TYPE[enum_type]
        enum_item = _generate_enum_definition(enum_def)
        partial_items.append(enum_item)
        enum_debug_item = _generate_enum_debug(enum_def)
        gen_items.append(enum_debug_item)

    # generate methods
    for method in module.methods:
        if method.languages and RuntimeLanguage.RUST not in method.languages:
            continue
        method_item = _generate_method_definition(module, method)
        partial_items.append(method_item)

    return partial_items, gen_items


#
# Files
#


def generate_files(schema: SchemaDefinition) -> dict[str, RustFile]:
    """Generate the new clean files for a given Destack schema (by local path)."""
    files: dict[str, RustFile] = {}

    #
    # Object Modules
    #

    # map object modules to files
    for module in schema.modules:
        if module.type != ModuleType.OBJECT:
            continue  # ignore index modules

        # map
        partial_items, gen_items = _generate_object_module(module)

        # partial file
        partial_imports = _get_imports(partial_items)
        partial_file = RustFile(
            type=RustManagedType.PARTIAL,
            source_path=module.path,
            local_path=source_path_to_local_path(module.path, is_gen=False),
            imports=partial_imports,
            items=partial_items,
            comment=f"//! {module.name}@{VERSION}",
            attributes=[
                RustAttribute(
                    content=f"#![destack::{RustManagedType.PARTIAL.value}({module.path}, file)]"
                ),
            ],
        )
        files[partial_file.local_path] = partial_file

        # generated file
        if gen_items:
            gen_imports = _get_imports(gen_items)
            gen_file = RustFile(
                type=RustManagedType.GENERATED,
                source_path=module.path,
                local_path=source_path_to_local_path(module.path, is_gen=True),
                imports=gen_imports,
                items=gen_items,
                comment=f"//! {module.name}@{VERSION}",
                attributes=[
                    RustAttribute(
                        content=f"#![destack::{RustManagedType.GENERATED.value}({module.path}, file)]"
                    ),
                ],
            )
            files[gen_file.local_path] = gen_file

    #
    # mod.rs files
    #

    # group files by directory
    mods_by_directory: dict[str, set[str]] = collections.defaultdict(set)
    for file in files.values():
        directory = file.local_path.rsplit("/", 1)[0]
        inner_name = file.local_path.split("/")[-1].split(".", maxsplit=1)[0]
        mods_by_directory[directory].add(inner_name)
    # group directories "recursively"
    # (e.g. for basics/access/membership we also want basics->access and access->membership)
    for directory in list(mods_by_directory.keys()):
        directory_parts = directory.split("/")
        for i in range(len(directory_parts) - 1):
            parent_directory = "/".join(directory_parts[: i + 1])
            mods_by_directory[parent_directory].add(directory_parts[i + 1])

    # gather all declared items per directory
    items_by_directory: dict[str, set[str]] = collections.defaultdict(set)
    for file in files.values():
        directory = file.local_path.split(".", maxsplit=1)[0]
        for item in file.items:
            if isinstance(item, RustManagedItem) and not item.inner_key:  # main item only
                items_by_directory[directory].add(item.object_key)
    # accumulate declared items up to the root
    # (e.g. basics/access/membership -> basics/access -> basics)
    for directory in list(items_by_directory.keys()):
        directory_parts = directory.split("/")
        for i in range(len(directory_parts) - 1):
            parent_directory = "/".join(directory_parts[: i + 1])
            items_by_directory[parent_directory].update(items_by_directory[directory])

    # generate mod.rs files
    for directory, directory_files in mods_by_directory.items():
        # generate items
        mods: list[RustModDeclaration] = []
        for inner_name in directory_files:
            # mod declaration
            mod = RustModDeclaration(name=inner_name, visibility=RustVisibility.PUBLIC)
            mods.append(mod)

        # accumulated imports from subdirectories
        imports: list[RustImport] = []
        for inner_name in directory_files:
            if inner_name.startswith("_"):
                continue  # skip internal items
            full_name = f"crate/{directory}/{inner_name}".replace("/", "::")
            inner_items = items_by_directory[f"{directory}/{inner_name}"]
            if not inner_items:
                continue  # nothing to re-export
            imp = RustImport(
                source=full_name,
                imports=list(inner_items),
                is_glob=False,
                is_internal=True,
                visibility=RustVisibility.PUBLIC,
            )
            imports.append(imp)

        # generate mod file
        local_path = directory + "/mod.rs"
        source_path = local_path_to_source_path(local_path)[:-7]  # minus mod.rs
        mod_file = RustFile(
            type=RustManagedType.PARTIAL,
            source_path=source_path,
            local_path=local_path,
            imports=imports,
            mods=mods,
            items=(),
            is_mod_rs=True,
            comment=f"//! {source_path}@{VERSION}",
            attributes=[
                RustAttribute(
                    content=f"#![destack::{RustManagedType.PARTIAL.value}({source_path}, file)]"
                ),
                RustAttribute(content="#![allow(unused_imports)]"),
                RustAttribute(content="#![allow(unreachable_pub)]"),
            ],
        )
        files[mod_file.local_path] = mod_file

    return files
