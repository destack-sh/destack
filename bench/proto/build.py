import os
import re
import shutil
import subprocess
from enum import Enum
from itertools import chain
from pathlib import Path
from typing import Any

import regex
import structlog
import typer

from bench.language import (
    ANCESTOR_NODE_TYPES,
    CHILD_NODE_TYPES,
    DESCENDANT_NODE_TYPES,
    ENUM_CLASS_BY_TYPE,
    ENUM_TYPES,
    FIELD_BASE_NODE_TYPES,
    FILE_FORMAT_BY_EXTENSION,
    FILE_FORMAT_BY_MIME_TYPE,
    NODE_CLASS_BY_TYPE,
    NODE_CLASSES,
    NODE_TYPES,
    PARENT_NODE_TYPES,
    STRUCT_CLASS_BY_TYPE,
    STRUCT_CLASSES,
    STRUCT_TYPES,
    SUBNODE_CLASSES,
    TYPE_BASE_NODE_TYPES,
    TYPE_CONSTRAINT_BY_FORMAT,
    UNSET,
    VERSION,
    BenchNode,
    DynamicResource,
    EnumType,
    InlineSourceNode,
    Node,
    Property,
    Resource,
    SourceNode,
    StaticResource,
    TypeConstraint,
    TypeConstraintIn,
    TypeFormat,
)
from bench.language.core.node import IsInlinable
from bench.utils.string import Casing, to_casing

from .engine import generate_proto_schema

LANG_PROTO = "proto/lang.proto"
TEMP_PY_DIR = "bench/pb2.tmp"
TARGET_PY_DIR = "bench/pb2"
TEMP_TS_DIR = "bench-web/src/proto/wire.tmp"
TARGET_TS_DIR = "bench-web/src/proto/wire"
EXTRA_PROTO_PY_FILES = (
    "proto/common.proto",
    "proto/health.proto",
    "proto/system.proto",
    "proto/runtime.proto",
    "proto/google/type/date.proto",
    "proto/google/type/datetime.proto",
    "proto/google/type/timeofday.proto",
)
EXTRA_PROTO_TS_FILES = (
    "proto/common.proto",
    "proto/health.proto",
    "proto/system.proto",
    "proto/web.proto",
    "proto/google/type/date.proto",
    "proto/google/type/datetime.proto",
    "proto/google/type/timeofday.proto",
)

logger = structlog.get_logger(__name__)
app = typer.Typer(short_help="proto management")


def run_shell_sync(cmd: str, check=True, **kwargs):
    """Executes a shell command in a subprocess."""
    cwd = os.getcwd()
    logger.trace("shell", cmd=cmd, cwd=cwd, check=check, **kwargs)
    subprocess.run(cmd, shell=True, check=check, **kwargs)


def _build_proto_schema() -> str:
    """Generate the .proto schema (as a string) describing the current Bench types."""
    node_classes = list(NODE_CLASSES)
    node_classes.sort(key=lambda cls: cls.metatype.id)
    proto = generate_proto_schema(
        name="symbolx.bench",
        unions={"SomeNode": ("node", node_classes)},
        extras=[],
        message_postfix="Data",
    )
    return proto.to_proto_source()


_PUBLIC_SERVICES = ("Graph", "Supervisor", "Host")  # :ServiceKind


def _render_js_value(value: Any) -> str:
    if isinstance(value, (list, tuple)):
        return f"[{', '.join(_render_js_value(v) for v in value)}]"
    elif isinstance(value, bool):
        return "true" if value else "false"
    elif isinstance(value, (int, float)):
        return str(value)
    elif isinstance(value, str):
        # escape string
        value = value.replace("\\", "\\\\").replace('"', '\\"')
        return f'"{value}"'
    elif isinstance(value, Enum):
        return f"{value.__class__.__name__}.{value.name}"
    else:
        raise RuntimeError(f"unexpected value: {value}")


def _render_js_constraint(constraint: TypeConstraint | TypeConstraintIn) -> str:
    constraint_parts = []
    for p in TypeConstraint.__declared_properties__.values():
        if p.reference_wired_ptr:
            p = p.reference_wired_ptr
        p_value = getattr(constraint, p.name, None)
        if p_value is not None:
            js_value = _render_js_value(p_value)
            constraint_parts.append(f"{to_casing(p.name, Casing.LOWER_CAMEL)}: {js_value}")
    constraint_js = "{ " + ", ".join(constraint_parts) + " }"
    return constraint_js


def _build_proto(schema_str: str) -> None:
    """Regenerate external artifacts from the proto schema."""

    on_apply = []

    # regenerate python & TS proto files
    Path(LANG_PROTO).write_text(schema_str)

    #
    # Python
    #

    # NOTE: we copy the proto files into the temporary wire directory to ensure the import paths
    #  are correct for protobuf's python generator.
    Path(TEMP_PY_DIR).mkdir(parents=True, exist_ok=True)
    py_proto_files = [LANG_PROTO, *EXTRA_PROTO_PY_FILES]
    # replace 'import "proto/..." with 'import "..." in all files in wire
    py_proto_files = [p.replace("proto/", "") for p in py_proto_files]
    run_shell_sync("cp -r proto wire")
    for path in Path("wire").rglob("*.proto"):
        path.write_text(regex.sub(r"import \"proto/", 'import "', path.read_text()))
    run_shell_sync(
        f"protoc -I wire --python_out={TEMP_PY_DIR} --pyi_out={TEMP_PY_DIR} --grpclib_python_out={TEMP_PY_DIR} {' '.join(py_proto_files)}"
    )
    run_shell_sync("rm -r wire")

    # patch in our extra stuff into every file
    generated_py_files = list(Path(TEMP_PY_DIR).rglob("*.py")) + list(
        Path(TEMP_PY_DIR).rglob("*.pyi")
    )
    for path in generated_py_files:
        wire_py = path.read_text()
        # replace 'import <name>' with 'from .<name> import <name>' (if name is one of generated_py_files)
        wire_py = regex.sub(
            rf"^import ({'|'.join(p.stem for p in generated_py_files)})",
            r"from . import \1",
            wire_py,
            flags=regex.MULTILINE,
        )
        if "_grpc" in path.stem:
            # snake case all methods (replace def <MyName> with def <my_name>, also for self.MyName)
            rpc_names = re.findall(r"async def ([a-zA-Z0-9_]+)\(", wire_py)
            for name in rpc_names:
                new_name = to_casing(name, Casing.SNAKE)
                wire_py = wire_py.replace(f"async def {name}(", f"async def {new_name}(")
                wire_py = wire_py.replace(f"self.{name}", f"self.{new_name}")
            # replace service methods Method(Stream) -> None with Method(Request, Metadata) -> Response | AsyncIterator[Response]
            wire_py = regex.sub(
                r"(?!.*watch)(async def )([a-zA-Z0-9_]+)\(self, stream: 'grpclib.server.Stream\[([a-zA-Z0-9_\.]+), ([a-zA-Z0-9_\.]+)\]'\) -> None:",
                r"\1\2(self, request: '\3', headers: Mapping) -> '\4':",
                wire_py,
                flags=re.MULTILINE,
            )
            wire_py = regex.sub(
                r"async (def )(watch[a-zA-Z0-9_]*)\(self, stream: 'grpclib.server.Stream\[([a-zA-Z0-9_\.]+), ([a-zA-Z0-9_\.]+)\]'\) -> None:",
                r"\1\2(self, request: '\3', headers: Mapping) -> AsyncIterator['\4']:",
                wire_py,
                flags=re.MULTILINE,
            )

        # rename XyzStub to XyzClient (stub is a bad name)
        wire_py = regex.sub(r"(?<!Service)Stub", "Client", wire_py)
        patch_prefix_code = """
# type: ignore
# ruff: noqa

from typing import TYPE_CHECKING, Union, AsyncIterator, Mapping

"""
        path.write_text(patch_prefix_code + "\n\n" + wire_py)

    Path(TEMP_PY_DIR + "/__init__.py").write_text(f"""
# ruff: noqa

from typing import TYPE_CHECKING, Union

VERSION = '{VERSION}'

# import from all generated files
from .runtime_pb2 import *
from .health_pb2 import *
from .common_pb2 import *
from .common_grpc import *
from .lang_grpc import *
from .runtime_grpc import *
from .system_grpc import *
from .health_grpc import *
from .lang_pb2 import *
from .system_pb2 import *
from .google.type.date_pb2 import *
from .google.type.timeofday_pb2 import *
from .google.type.datetime_pb2 import *

# extra utility types
AnyNodeData = Union[{', '.join([cls.__name__ + 'Data' for cls in NODE_CLASSES])}]
AnyStructData = Union[{', '.join([cls.__name__ + 'Data' for cls in STRUCT_CLASSES])}]
AnyObjectData = AnyNodeData | AnyStructData
BenchNodeData = Union[{', '.join([cls.__name__ + 'Data' for cls in NODE_CLASSES if issubclass(cls, BenchNode)])}]
ResourceNodeData = Union[{', '.join([cls.__name__ + 'Data' for cls in NODE_CLASSES if issubclass(cls, Resource)])}]
DynamicResourceNodeData = Union[{', '.join([cls.__name__ + 'Data' for cls in NODE_CLASSES if issubclass(cls, DynamicResource)])}]
StaticResourceNodeData = Union[{', '.join([cls.__name__ + 'Data' for cls in NODE_CLASSES if issubclass(cls, StaticResource)])}]
SourceNodeData = Union[{', '.join([cls.__name__ + 'Data' for cls in NODE_CLASSES if issubclass(cls, SourceNode)])}]
InlineSourceNodeData = Union[{', '.join([cls.__name__ + 'Data' for cls in NODE_CLASSES if issubclass(cls, InlineSourceNode)])}]
InlineNodeData = Union[{', '.join([cls.__name__ + 'Data' for cls in NODE_CLASSES if issubclass(cls, IsInlinable)])}]
TypeBaseNodeData = Union[{', '.join([cls.__name__ + 'Data' for cls in NODE_CLASSES if cls.metatype in TYPE_BASE_NODE_TYPES])}]
FieldBaseNodeData = Union[{', '.join([cls.__name__ + 'Data' for cls in NODE_CLASSES if cls.metatype in FIELD_BASE_NODE_TYPES])}]
StateNodeData = Union[{', '.join([cls.__name__ + 'Data' for cls in NODE_CLASSES if cls.metatype.is_state])}]
RuntimeNodeData = Union[{', '.join([cls.__name__ + 'Data' for cls in NODE_CLASSES if cls.metatype.is_runtime])}]
""")
    on_apply.append(lambda: shutil.rmtree(TARGET_PY_DIR, ignore_errors=True))  # noqa: FURB113
    on_apply.append(lambda: shutil.copytree(TEMP_PY_DIR, TARGET_PY_DIR))
    on_apply.append(lambda: shutil.rmtree(TEMP_PY_DIR, ignore_errors=True))

    #
    # TypeScript (protobuf-ts)
    #

    shutil.rmtree(TEMP_TS_DIR, ignore_errors=True)
    Path(TEMP_TS_DIR).mkdir(parents=True, exist_ok=True)
    run_shell_sync(
        f"bun x protoc --ts_out {TEMP_TS_DIR} --proto_path . {LANG_PROTO} {' '.join(EXTRA_PROTO_TS_FILES)}",
    )

    # magic replace code so that Value is transparently encoded/decoded :MagicJsValuePacking
    #  (like in ts-proto, but protobuf-ts doesn't have an option for this unfortunately)
    generated_ts_files = list(Path(TEMP_TS_DIR).rglob("*.ts"))
    for path in generated_ts_files:
        wire_ts = path.read_text()
        original_wire_ts = wire_ts
        # replace valuePacked?: Value with valuePacked?: JsonValue
        wire_ts = regex.sub(
            r"(\w+)Packed\?: Value", r"\1Packed?: JsonValue", wire_ts, flags=regex.MULTILINE
        )
        if len(wire_ts) == len(original_wire_ts):
            continue  # nothing to do
        # replace guard with undefined check
        # if (message.valuePacked)
        # -> if (message.valuePacked !== undefined)
        wire_ts = regex.sub(
            r"if \(message\.(\w+)Packed\)",
            r"if (message.\1Packed !== undefined)",
            wire_ts,
        )
        # replace write with wrapped write
        # Value.internalBinaryWrite(message.valuePacked, writer.tag(42, WireType.LengthDelimited).fork(), options).join();
        # -> Value.internalBinaryWrite(Value.fromJson(message.valuePacked), writer.tag(36, WireType.LengthDelimited).fork(), options).join();
        wire_ts = regex.sub(
            r"Value\.internalBinaryWrite\(message\.(\w+)Packed, writer\.tag\((\d+), WireType\.LengthDelimited\)\.fork\(\), options\)\.join\(\);",
            r"Value.internalBinaryWrite(Value.fromJson(message.\1Packed), writer.tag(\2, WireType.LengthDelimited).fork(), options).join();",
            wire_ts,
        )
        # replace read with wrapped read
        # message.valuePacked = Value.internalBinaryRead(reader, reader.uint32(), options, message.valuePacked);
        # -> message.valuePacked = Value.toJson(Value.internalBinaryRead(reader, reader.uint32(), options, undefined));
        wire_ts = regex.sub(
            r"message\.(\w+)Packed = Value\.internalBinaryRead\(reader, reader\.uint32\(\), options, message\.\1Packed\);",
            r"message.\1Packed = Value.toJson(Value.internalBinaryRead(reader, reader.uint32(), options, undefined));",
            wire_ts,
        )

        wire_ts = (
            wire_ts
            + """

export type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | { [key: string]: JsonValue } | JsonValue[];
"""
        )
        path.write_text(wire_ts)

    # ancestry maps
    ancestry_maps_parts = []
    for name, map in (
        ("PARENT_NODE_TYPES", PARENT_NODE_TYPES),
        ("CHILD_NODE_TYPES", CHILD_NODE_TYPES),
        ("ANCESTOR_NODE_TYPES", ANCESTOR_NODE_TYPES),
        ("DESCENDANT_NODE_TYPES", DESCENDANT_NODE_TYPES),
    ):
        map_parts = [
            f"export const {name}: Record<NodeType, NodeType[]> = {{\n",
            "  [NodeType.UNSPECIFIED]: [],\n",
        ]
        for node_t, node_ts in map.items():
            map_parts.append(
                f"  [NodeType.{node_t.name}]: [{', '.join(f'NodeType.{t.name}' for t in node_ts)}],\n"
            )
        map_parts.append("}\n")
        ancestry_maps_parts.append("".join(map_parts))
    ancestry_maps_str = "\n".join(ancestry_maps_parts)

    # message type mappings
    message_type_map_parts = []
    for cls in chain(NODE_CLASSES, STRUCT_CLASSES):
        message_type_map_parts.append(f"  [ObjectType.{cls.metatype.name}]: {cls.__name__}Data,\n")
    # (we have a MessageType of our own, so MessageType from protobuf is aliased to MessageType$)
    message_type_map_str = (
        "export const MESSAGE_TYPE_BY_OBJECT_TYPE: Partial<Record<ObjectType, MessageType$<any>>> = {\n"
        + "".join(message_type_map_parts)
        + "}\n"
    )
    message_type_inv_map_str = (
        "export const OBJECT_TYPE_BY_MESSAGE_TYPE_NAME: Record<string, ObjectType> = {\n"
        + "".join(
            f'  ["symbolx.bench.{cls.__name__}Data"]: ObjectType.{cls.metatype.name},\n'
            for cls in chain(NODE_CLASSES, STRUCT_CLASSES)
        )
        + "}\n"
    )
    enum_by_type_parts = [
        "export const ENUM_BY_TYPE: Record<EnumType, Record<number | string, number | string>> = {\n",
        "  [EnumType.UNSPECIFIED]: {},\n",
    ]
    for enum_t in ENUM_TYPES:
        enum_cls = ENUM_CLASS_BY_TYPE[enum_t]
        enum_by_type_parts.append(f"  [EnumType.{enum_t.name}]: {enum_cls.__name__},\n")
    enum_by_type_parts.append("}\n")
    enum_by_type_str = "".join(enum_by_type_parts)

    # type mappings
    # struct mappings
    struct_mapping_parts = [
        "export interface StructTypeMapping extends Record<StructType, AnyStructData> {\n"
    ]
    for struct_t in STRUCT_TYPES:
        struct_cls = STRUCT_CLASS_BY_TYPE[struct_t]
        struct_mapping_parts.append(f"  [StructType.{struct_t.name}]: {struct_cls.__name__}Data,\n")
    struct_mapping_parts.append("}\n")
    struct_mapping_str = "".join(struct_mapping_parts)
    # node mappings
    node_mapping_parts = [
        "export interface NodeTypeMapping extends Record<NodeType, AnyNodeData> {\n"
    ]
    for node_t in NODE_TYPES:
        node_cls = NODE_CLASS_BY_TYPE[node_t]
        node_mapping_parts.append(f"  [NodeType.{node_t.name}]: {node_cls.__name__}Data,\n")
    node_mapping_parts.append("}\n")
    node_mapping_str = "".join(node_mapping_parts)
    # node subtype mappings (per base node)
    subnode_mappings_parts: list[str] = []
    for node_cls in NODE_CLASSES:
        if not node_cls.__has_subtypes__ or not node_cls.__subclass_by_subtype__:
            continue
        subtype_prop = node_cls.__subtype_base_property__
        assert subtype_prop is not None
        subtype_enum = ENUM_CLASS_BY_TYPE[subtype_prop.enum_type]  # type: ignore
        subnode_mapping_parts = [
            f"export type {node_cls.__name__}Subtype = {' | '.join(cls.__name__ + 'Data' for cls in node_cls.__subclass_by_subtype__.values())};\n",
            f"export interface {node_cls.__name__}SubtypeMapping extends Record<{subtype_enum.__name__}, {node_cls.__name__}Subtype> {{\n",
        ]
        for subtype, subnode_cls in node_cls.__subclass_by_subtype__.items():
            subnode_mapping_parts.append(
                f"  [{subtype_enum.__name__}.{subtype.name}]: {subnode_cls.__name__}Data,\n"
            )
        subnode_mapping_parts.append("}\n")
        subnode_mappings_parts.append("".join(subnode_mapping_parts))
    subnode_base_mapping_parts = [
        "export interface NodeSubtypeMapping extends Record<NodeType, object> {\n"
    ]
    for node_cls in NODE_CLASSES:
        if node_cls.__has_subtypes__ and node_cls.__subclass_by_subtype__:
            subnode_base_mapping_parts.append(
                f"  [NodeType.{node_cls.metatype.name}]: {node_cls.__name__}SubtypeMapping,\n"
            )
    subnode_base_mapping_parts.append("}\n")
    subnode_mappings_parts.append("".join(subnode_base_mapping_parts))
    subnode_mappings_str = "\n".join(subnode_mappings_parts)

    # combined node/struct mappings
    any_mapping_parts = [
        "export interface AnyTypeMapping extends Record<ObjectType, AnyStructData | AnyNodeData> {\n"
    ]
    for cls in chain(NODE_CLASSES, STRUCT_CLASSES):
        any_mapping_parts.append(f"  [ObjectType.{cls.metatype.name}]: {cls.__name__}Data,\n")
    any_mapping_parts.append("}\n")
    object_mapping_str = "".join(any_mapping_parts)
    enum_mapping_parts = [
        "export interface EnumTypeMapping extends Record<EnumType, any> {\n",
        "  [EnumType.UNSPECIFIED]: {},\n",
    ]
    for enum_t in ENUM_TYPES:
        enum_cls = ENUM_CLASS_BY_TYPE[enum_t]
        enum_mapping_parts.append(f"  [EnumType.{enum_t.name}]: {enum_cls.__name__},\n")
    enum_mapping_parts.append("}\n")
    enum_mapping_str = "".join(enum_mapping_parts)

    # property enum for each class
    property_enums_parts: list[str] = []
    for cls in chain(NODE_CLASSES, SUBNODE_CLASSES, STRUCT_CLASSES):
        props_strs: list[str] = []
        if issubclass(cls, Node) and cls.__subtype__:
            properties = [p for p in cls.__subtype_extra_properties__.values() if p.is_wired]
        else:
            properties = list(cls.__wired_properties__.values())
        for prop in sorted(properties, key=lambda p: p.id):
            ts_name = to_casing(prop.name, Casing.CAMEL)
            ts_name = ts_name[0].lower() + ts_name[1:]
            props_strs.append(f"  {ts_name} = {prop.id},")
        property_enums_parts.append(
            f"export enum {cls.__name__}Property {{\n" + "\n".join(props_strs) + "\n}\n"
        )
    property_enums_str = "\n".join(property_enums_parts)

    # property enum maps for final classes
    property_enum_maps_final_parts: list[str] = []
    for camel_prefix, upper_prefix, classes in (
        ("Node", "NODE_", NODE_CLASSES),
        ("Struct", "STRUCT_", STRUCT_CLASSES),
        ("", "", chain(NODE_CLASSES, STRUCT_CLASSES)),
    ):
        property_enum_map_parts: list[str] = []
        for cls in classes:
            property_enum_map_parts.append(
                f"  [ObjectType.{cls.metatype.name}]: {cls.__name__}Property,\n"
            )
        property_enum_map_str = (
            f"export const {upper_prefix}PROPERTY_ENUM_BY_TYPE: Partial<Record<ObjectType, Any{camel_prefix}PropertyType>> = {{\n"
            + "".join(property_enum_map_parts)
            + "}\n"
        )
        property_enum_maps_final_parts.append(property_enum_map_str)
    # property enum maps for subnodes
    for node_cls in NODE_CLASSES:
        if not node_cls.__has_subtypes__:
            continue
        property_enum_map_parts: list[str] = [
            f"export const {node_cls.metatype.name}_PROPERTY_ENUM_BY_SUBTYPE: Partial<Record<{node_cls.__name__}Type, any>> = {{\n"
        ]
        for subnode_type, subnode_cls in node_cls.__subclass_by_subtype__.items():
            property_enum_map_parts.append(
                f"  [{node_cls.__name__}Type.{subnode_type.name}]: {subnode_cls.__name__}Property,\n"
            )
        property_enum_map_parts.append("}\n")
        property_enum_maps_final_parts.append("".join(property_enum_map_parts))
    # property enum map into subnode maps
    property_enum_map_parts: list[str] = [
        "export const PROPERTY_ENUM_BY_SUBTYPE: Partial<Record<NodeType, Record<any, any>>> = {\n"
    ]
    for node_cls in NODE_CLASSES:
        if node_cls.__has_subtypes__:
            property_enum_map_parts.append(
                f"  [NodeType.{node_cls.metatype.name}]: {node_cls.metatype.name}_PROPERTY_ENUM_BY_SUBTYPE,\n"
            )
    property_enum_map_parts.append("}\n")
    property_enum_maps_final_parts.append("".join(property_enum_map_parts))
    property_enum_maps_final_str = "\n".join(property_enum_maps_final_parts)

    object_info_type_str = """
export type TypeConstraintIn = Partial<Omit<TypeConstraintData, "metatype">>;
export type PropertyKind = 'primitive' | 'enum' | 'reference';
export type PropertyInfo = {
    // basics
    id: number;
    name: string;
    component: ObjectType;
    componentSubtype?: number;
    kind: PropertyKind;
    primitiveType?: PrimitiveType;
    enumType?: EnumType;
    default?: any;
    constraint?: TypeConstraintIn;
    fieldType?: FieldType;

    // flags
    isList?: boolean;
    isRequired?: boolean;
    isInternal?: boolean;
    isSystem?: boolean;
    isKernel?: boolean;
    isAutoset?: boolean;
    isComputed?: boolean;
    isRuntime?: boolean;
    isWired?: boolean;
    isStored?: boolean;
    isUnique?: boolean;
    isDeferred?: boolean;
    isSensitive?: boolean;
    isEncrypted?: boolean;
        
    // value
    isValueRuntime?: boolean;
    isValuePacked?: boolean;
    valuePackedId?: number;
    valueIsPartial?: boolean;
    
    // references
    referenceKind?: ReferenceKind;
    referenceNodes?: NodeType[] | "any";
    referenceStruct?: StructType;
    referenceIsNodeData?: boolean;
}
    """
    type_info_definitions_parts = []
    for bench_cls in chain(NODE_CLASSES, SUBNODE_CLASSES, STRUCT_CLASSES):
        prop_infos_strs: list[str] = []
        if issubclass(bench_cls, Node) and bench_cls.__subtype__:
            properties = list(bench_cls.__subtype_extra_properties__.values())
        else:
            properties = list(bench_cls.__properties__.values())
        for prop in sorted(properties, key=lambda p: p.id or 0):
            if not prop.is_wired:
                continue

            prop_info_parts: dict[str, str] = {
                "id": str(prop.id),
                "name": repr(prop.name),
                "component": f"ObjectType.{bench_cls.metatype.name}",
            }
            if issubclass(bench_cls, Node) and bench_cls.__subtype__:
                prop_info_parts["componentSubtype"] = f"{bench_cls.__subtype__.value}"
            if prop.reference_kind:
                kind = "reference"
            elif prop.enum_type:
                kind = "enum"
                prop_info_parts["enumType"] = f"EnumType.{prop.enum_type.name}"
            else:
                kind = "primitive"
            prop_info_parts["kind"] = repr(kind)
            if prop.primitive_type and prop.primitive_type is not UNSET:
                prop_info_parts["primitiveType"] = f"PrimitiveType.{prop.primitive_type.name}"
            if prop.default is not None and prop.default is not UNSET:
                prop_info_parts["default"] = _render_js_value(prop.default)
            if prop.constraint:
                prop_info_parts["constraint"] = _render_js_constraint(prop.constraint)
            if prop.field_type is not None:
                prop_info_parts["fieldType"] = f"FieldType.{prop.field_type.name}"

            for flag in (
                "isList",
                "isRequired",
                "isInternal",
                "isSystem",
                "isKernel",
                "isAutoset",
                "isComputed",
                "isRuntime",
                "isWired",
                "isStored",
                "isUnique",
                "isDeferred",
                "isSensitive",
                "isEncrypted",
            ):
                if getattr(prop, to_casing(flag, Casing.SNAKE)):
                    prop_info_parts[flag] = "true"
            for value_flag in ("isValueRuntime", "isValuePacked"):
                if getattr(prop, to_casing(value_flag, Casing.SNAKE)):
                    prop_info_parts[value_flag] = "true"
            if prop.value_packed_ptr:
                assert isinstance(prop.value_packed_ptr, Property)
                prop_info_parts["valuePackedId"] = str(prop.value_packed_ptr.id)
            if prop.reference_kind:
                prop_info_parts["referenceKind"] = f"ReferenceKind.{prop.reference_kind.name}"
            if prop.reference_nodes:
                if prop.reference_nodes != "any":
                    nodes_str_parts = [f"NodeType.{node.name}" for node in prop.reference_nodes]
                    prop_info_parts["referenceNodes"] = f"[{', '.join(nodes_str_parts)}]"
                else:
                    prop_info_parts["referenceNodes"] = '"any"'
            if prop.reference_struct:
                prop_info_parts["referenceStruct"] = f"StructType.{prop.reference_struct.name}"
            if prop.reference_is_node_data:
                prop_info_parts["referenceIsNodeData"] = "true"
            if prop.value_is_partial:
                prop_info_parts["valueIsPartial"] = "true"

            prop_info_str = ", ".join(f"{k}: {v}" for k, v in prop_info_parts.items())
            prop_infos_strs.append(
                f"  [{bench_cls.__name__}Property.{to_casing(prop.name, Casing.LOWER_CAMEL)}]: {{ {prop_info_str} }},"
            )

        # convert to string
        type_info_parts = [
            f"export const {bench_cls.__name__}DataInfo: Record<{bench_cls.__name__}Property, PropertyInfo> = {{",
            "\n".join(prop_infos_strs),
            "}",
        ]
        type_info_definitions_parts.append("\n".join(type_info_parts))

    object_info_definitions_str = "\n".join(type_info_definitions_parts)

    # property info mappings
    object_type_info_map_parts = [
        "export const PROPERTY_INFOS_BY_TYPE: Record<ObjectType, Record<any, PropertyInfo>> = {\n"
        "  [ObjectType.UNSPECIFIED]: {},\n"
    ]
    for bench_cls in chain(NODE_CLASSES, STRUCT_CLASSES):
        object_type_info_map_parts.append(
            f"  [ObjectType.{bench_cls.metatype.name}]: {bench_cls.__name__}DataInfo,\n"
        )
    object_type_info_map_parts.append("}\n")
    # subnode property info mappings
    subnode_type_info_maps: list[str] = []
    for node_cls in NODE_CLASSES:
        if node_cls.__has_subtypes__:
            subnode_type_info_parts: list[str] = [
                f"export const {node_cls.__name__}SubtypePropertyInfo: Partial<Record<{node_cls.__name__}Type, Record<any, PropertyInfo>>> = {{\n"
            ]
            for subnode_type, subnode_cls in node_cls.__subclass_by_subtype__.items():
                subnode_type_info_parts.append(
                    f"  [{node_cls.__name__}Type.{subnode_type.name}]: {subnode_cls.__name__}DataInfo,\n"
                )
            subnode_type_info_parts.append("}\n")
            subnode_type_info_maps.append("".join(subnode_type_info_parts))
    subnode_type_info_maps.append(
        "export const PROPERTY_INFOS_BY_SUBTYPE: Partial<Record<NodeType, Record<any, Record<any, PropertyInfo>>>> = {\n"
    )
    for node_cls in NODE_CLASSES:
        if node_cls.__has_subtypes__:
            subnode_type_info_maps.append(
                f"  [NodeType.{node_cls.metatype.name}]: {node_cls.__name__}SubtypePropertyInfo,\n"
            )
    subnode_type_info_maps.append("}\n")
    object_info_map_str = (
        "".join(object_type_info_map_parts) + "\n" + "".join(subnode_type_info_maps)
    )

    # node subtype keys
    node_subtype_info_parts: list[str] = []
    for node_cls in NODE_CLASSES:
        if node_cls.__subtype_base_property__:
            node_subtype_info_parts.append(
                f"  [NodeType.{node_cls.metatype.name}]: {node_cls.__subtype_base_property__.id},"
            )
    node_subtype_info_str = f"""
export const NODE_SUBTYPE_PROPERTY_ID: Partial<Record<NodeType, number>> = {{
{'\n'.join(node_subtype_info_parts)}
}}
"""

    # enum options
    enum_option_info_type_str = """
export type EnumOptionInfo = {
    id: number;
    name: string;
    title?: string;
    text?: string;
    color?: ColorType;
    icon?: string;
}
"""
    enum_option_info_parts: list[str] = []
    enum_option_info_map_parts: list[str] = []
    for enum_type in EnumType:
        enum_cls = ENUM_CLASS_BY_TYPE[enum_type]
        # only include if one of the options has something
        if not any(
            option.text or option.title or option.color or option.icon for option in enum_cls
        ):
            continue
        option_info_parts: list[str] = []
        for option in enum_cls:
            if option.text or option.title or option.color or option.icon:
                option_str_parts = [f"id: {option.value}, name: {option.name!r}"]
                if option.text:
                    option_str_parts.append(f"text: {option.text!r}")
                if option.title:
                    option_str_parts.append(f"title: {option.title!r}")
                if option.color:
                    option_str_parts.append(f"color: ColorType.{option.color.name}")
                if option.icon:
                    option_str_parts.append(f"icon: {option.icon!r}")
                option_info_parts.append(
                    f"  [{enum_cls.__name__}.{option.name}]: {{ {', '.join(option_str_parts)} }},"
                )
        enum_option_info_parts.extend(
            (
                f"export const {enum_cls.__name__}OptionInfo: Partial<Record<{enum_cls.__name__}, EnumOptionInfo>> = {{",
                "\n".join(option_info_parts),
                "}\n",
            )
        )
        enum_option_info_map_parts.append(
            f"  [EnumType.{enum_type.name}]: {enum_cls.__name__}OptionInfo,"
        )
    enum_option_info_str = "\n".join(enum_option_info_parts)
    enum_option_info_map_str = f"""
export const ENUM_OPTION_INFO_BY_TYPE: Partial<Record<EnumType, Record<any, EnumOptionInfo>>> = {{
{'\n'.join(enum_option_info_map_parts)}
}}
"""

    # file mapping enums
    file_format_by_extension_str_inner = "\n".join(
        f'  "{extension}": FileFormat.{file_format.name.upper()},'
        for extension, file_format in FILE_FORMAT_BY_EXTENSION.items()
    )
    file_format_by_extension_str = f"""
export const FILE_FORMAT_BY_EXTENSION: Record<string, FileFormat> = {{
{file_format_by_extension_str_inner}
}}
export const EXTENSIONS_BY_FILE_FORMAT = groupByList(Object.keys(FILE_FORMAT_BY_EXTENSION), ext => FILE_FORMAT_BY_EXTENSION[ext]);
export const EXTENSION_BY_FILE_FORMAT: Partial<Record<FileFormat, string>> = Object.fromEntries(
    Object.entries(FILE_FORMAT_BY_EXTENSION).map(([k, v]) => [v, k]),
);
"""
    file_format_by_mime_type_str_inner = "\n".join(
        f'  "{mime_type}": FileFormat.{file_format.name.upper()},'
        for mime_type, file_format in FILE_FORMAT_BY_MIME_TYPE.items()
    )
    file_format_by_mime_type_str = f"""
export const FILE_FORMAT_BY_MIME_TYPE: Record<string, FileFormat> = {{
{file_format_by_mime_type_str_inner}
}}
export const MIME_TYPES_BY_FILE_FORMAT = groupByList(Object.keys(FILE_FORMAT_BY_MIME_TYPE), mimeType => FILE_FORMAT_BY_MIME_TYPE[mimeType]);
export const MIME_TYPE_BY_FILE_FORMAT: Partial<Record<FileFormat, string>> = Object.fromEntries(
    Object.entries(FILE_FORMAT_BY_MIME_TYPE).map(([k, v]) => [v, k]),
);
"""
    file_mapping_enums_str = f"""
{file_format_by_extension_str}
{file_format_by_mime_type_str}
"""

    # type formats :TypeFormat
    type_formats_str_inner = "\n".join(
        f"  [TypeFormat.{t.name.upper()}]: {_render_js_constraint(TYPE_CONSTRAINT_BY_FORMAT[t])},"
        for t in TypeFormat
    )
    type_formats_str = f"""
export const TYPE_CONSTRAINT_BY_FORMAT: Partial<Record<TypeFormat, TypeConstraintIn>> = {{
{type_formats_str_inner}
}}
"""

    patch_postfix_code = f"""
//
// Extra utility types
//

// Any...
export type AnyNodeData = {' | '.join(cls.__name__ + 'Data' for cls in NODE_CLASSES)}
export type AnyStructData = {' | '.join(cls.__name__ + 'Data' for cls in STRUCT_CLASSES)}
export type AnyNodeDataType = {' | '.join('typeof ' + cls.__name__ + 'Data' for cls in NODE_CLASSES)}
export type AnyStructDataType = {' | '.join('typeof ' + cls.__name__ + 'Data' for cls in STRUCT_CLASSES)}
export type BenchNodeData = {' | '.join(cls.__name__ + 'Data' for cls in NODE_CLASSES if issubclass(cls, BenchNode))}
export type ResourceNodeData = {' | '.join(cls.__name__ + 'Data' for cls in NODE_CLASSES if issubclass(cls, Resource))}
export type SourceNodeData = {' | '.join(cls.__name__ + 'Data' for cls in NODE_CLASSES if issubclass(cls, SourceNode))}
export type InlineSourceNodeData = {' | '.join(cls.__name__ + 'Data' for cls in NODE_CLASSES if issubclass(cls, InlineSourceNode))}
export type InlineNodeData = {' | '.join(cls.__name__ + 'Data' for cls in NODE_CLASSES if issubclass(cls, IsInlinable))}
export type TypeBaseNodeData = {' | '.join(cls.__name__ + 'Data' for cls in NODE_CLASSES if cls.metatype in TYPE_BASE_NODE_TYPES)}
export type FieldBaseNodeData = {' | '.join(cls.__name__ + 'Data' for cls in NODE_CLASSES if cls.metatype in FIELD_BASE_NODE_TYPES)}
export type StateNodeData = {' | '.join(cls.__name__ + 'Data' for cls in NODE_CLASSES if cls.metatype.is_state)}
export type RuntimeNodeData = {' | '.join(cls.__name__ + 'Data' for cls in NODE_CLASSES if cls.metatype.is_runtime)}
export type StaticResourceNodeData = {' | '.join(cls.__name__ + 'Data' for cls in NODE_CLASSES if issubclass(cls, StaticResource))}
export type DynamicResourceNodeData = {' | '.join(cls.__name__ + 'Data' for cls in NODE_CLASSES if issubclass(cls, DynamicResource))}

// Ancestry maps
{ancestry_maps_str}

// Message types
{message_type_map_str}
{message_type_inv_map_str}
{enum_by_type_str}

// Type mappings
{struct_mapping_str}
{node_mapping_str}
{subnode_mappings_str}
{object_mapping_str}
{enum_mapping_str}

// Property enums
{property_enums_str}
export type AnyNodeProperty = {' | '.join('typeof ' + cls.__name__ + 'Property' for cls in NODE_CLASSES)}
export type AnyStructProperty = {' | '.join('typeof ' + cls.__name__ + 'Property' for cls in STRUCT_CLASSES)}
export type AnyProperty = AnyNodeProperty | AnyStructProperty
export type AnyNodePropertyType = {' | '.join('typeof ' + cls.__name__ + 'Property' for cls in NODE_CLASSES)}
export type AnyStructPropertyType = {' | '.join('typeof ' + cls.__name__ + 'Property' for cls in STRUCT_CLASSES)}
export type AnyPropertyType = {' | '.join('typeof ' + cls.__name__ + 'Property' for cls in chain(NODE_CLASSES, STRUCT_CLASSES))}
{property_enum_maps_final_str}

// Type info
{object_info_type_str}
{object_info_definitions_str}
{object_info_map_str}
{node_subtype_info_str}
// Enum options
{enum_option_info_type_str}
{enum_option_info_str}
{enum_option_info_map_str}

// Misc
{file_mapping_enums_str}
{type_formats_str}
    """
    lang_ts = Path(TEMP_TS_DIR + "/proto/lang.ts").read_text()
    Path(TEMP_TS_DIR + "/proto/lang.ts").write_text(lang_ts + "\n\n" + patch_postfix_code)

    # index.ts
    Path(TEMP_TS_DIR + "/index.ts").write_text(
        """
// re-export generated wire files
export * from './proto/common';
export * from './proto/lang';
export * from './proto/web';
export * from './proto/system';
export * from './proto/system.client';
export * from './google/protobuf/descriptor';
export * from './google/protobuf/struct';
export * from './google/protobuf/timestamp';
export * from './proto/google/type/date';
export * from './proto/google/type/timeofday';
export * from './proto/google/type/datetime';
        """
    )

    # prepend every TS file in $TARGET_TS_DIR with /* eslint-disable */ and required imports
    extra_imports = ["import { groupByList } from '@/utils/functools';"]
    for path in Path(TEMP_TS_DIR).glob("**/*.ts"):
        path.write_text(
            "/* eslint-disable */\n" + "\n".join(extra_imports) + "\n" + path.read_text()
        )

    # amend every .client.ts file with our OperationOptions
    for path in Path(TEMP_TS_DIR).glob("**/*.client.ts"):
        patched_file = path.read_text().replace(": RpcOptions", ": OperationOptions")
        # append import
        patched_file = patched_file + '\nimport type { OperationOptions } from "@/proto/services";'
        path.write_text(patched_file)

    # overwrite WIRE_TS_DIR with TEMP_TS_DIR
    on_apply.append(
        lambda: (
            shutil.rmtree(TARGET_TS_DIR, ignore_errors=True),
            shutil.move(TEMP_TS_DIR, TARGET_TS_DIR),
        )
    )

    # and apply
    for apply in on_apply:
        apply()
