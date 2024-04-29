import re
import shutil
import time
from itertools import chain
from pathlib import Path
from subprocess import DEVNULL

import structlog
import typer

from bench.cli.utils import _shell
from bench.language import VERSION, Node
from bench.language.const import ENUM_TYPES, NODE_TYPES, STRUCT_TYPES, UNSET
from bench.language.property import Property
from bench.language.setup import (
    ANCESTOR_NODE_TYPES,
    BENCH_CLASS_BY_TYPE,
    CHILD_NODE_TYPES,
    DESCENDANT_NODE_TYPES,
    ENUM_CLASS_BY_TYPE,
    ENUM_TYPE_BY_CLASS,
    FINAL_BENCH_CLASSES,
    NODE_CLASS_BY_TYPE,
    NODE_CLASSES,
    PARENT_NODE_TYPES,
    STRUCT_CLASS_BY_TYPE,
    STRUCT_CLASSES,
)
from bench.proto.engine import generate_proto_schema
from bench.utils.casing import Casing, to_casing

LANG_PROTO = "bench/proto/lang.proto"
TEMP_PY_DIR = "bench/proto/wire.tmp"
TEMP_PY_FILE = "bench/proto/wire.py.tmp"
WIRE_PY_FILE = "bench/proto/wire.py"
TEMP_TS_DIR = "bench-web/src/proto/wire.tmp"
WIRE_TS_DIR = "bench-web/src/proto/wire"
EXTRA_PROTO_PY_FILES = "bench/proto/common.proto bench/proto/services.proto"
EXTRA_PROTO_TS_FILES = "bench/proto/common.proto bench/proto/services.proto bench/proto/web.proto"

logger = structlog.get_logger(__name__)
app = typer.Typer(short_help="proto management")


def _generate_proto_schema() -> str:
    """Generate the .proto schema (as a string) describing the current Bench types."""
    node_classes = list(NODE_CLASSES)
    node_classes.sort(key=lambda cls: cls.metatype.id)
    proto = generate_proto_schema(
        name="symbolx.bench",
        bench_classes=[*FINAL_BENCH_CLASSES, Node],
        aliases={Node: "BaseNode"},
        # TODO :Performance: improve hetero node wrapping
        #  (SomeNodeData union seems inefficient)
        unions={"SomeNode": ("node", node_classes)},
        extras=[],
        message_postfix="Data",
    )
    return proto.to_proto_source()


def _regen_proto_artifacts(schema_str: str) -> None:
    """Regenerate external artifacts from the proto schema."""

    on_apply = []

    # regenerate python & TS proto files
    Path(LANG_PROTO).write_text(schema_str)

    #
    # Python (betterproto)
    #

    logger.info("proto.regen.py")
    Path(TEMP_PY_FILE).unlink(missing_ok=True)
    Path(TEMP_PY_DIR).mkdir(parents=True, exist_ok=True)
    _shell(
        f"protoc -I . --python_betterproto_out={TEMP_PY_DIR} {LANG_PROTO} {EXTRA_PROTO_PY_FILES}",
    )
    _shell(f"mv {TEMP_PY_DIR}/symbolx/bench/__init__.py {TEMP_PY_FILE}")

    # add/patch our extra stuff
    wire_py = Path(TEMP_PY_FILE).read_text()
    wire_py = re.sub(
        # rename all request parameters to 'request', add subject parameter
        # * is used only in stub signatures by betterproto (we only want bases here)
        r"self, [a-z_]+_request:(?! \"[a-zA-Z]\", \*)",
        'self, subject: "Subject", request:',
        wire_py,
    )
    wire_py = re.sub(r"\w[a-z_]+request,", "request,", wire_py)
    wire_py = re.sub(r"\w[a-z_]+request:", "request:", wire_py)
    patch_prefix_code = """
"""
    patch_postfix_code = f"""
from typing import TYPE_CHECKING # noqa: E402

VERSION = '{VERSION}'

if TYPE_CHECKING:
    from bench.language import Subject
    
# extra utility types
import bench.proto.monkey # noqa

from typing import Union # noqa
AnyNodeData = Union[{', '.join([cls.__name__ + 'Data' for cls in NODE_CLASSES])}]
AnyStructData = Union[{', '.join([cls.__name__ + 'Data' for cls in STRUCT_CLASSES])}]
    """
    Path(TEMP_PY_FILE).write_text(
        patch_prefix_code + "\n\n" + wire_py + "\n\n" + patch_postfix_code
    )
    shutil.rmtree(TEMP_PY_DIR, ignore_errors=True)
    _shell(f"ruff {TEMP_PY_FILE} --fix", check=True, stdout=DEVNULL)
    _shell(f"black {TEMP_PY_FILE}", check=True, stdout=DEVNULL)
    _shell(f"isort {TEMP_PY_FILE}", check=True, stdout=DEVNULL)
    on_apply.append(lambda: shutil.move(TEMP_PY_FILE, WIRE_PY_FILE))

    #
    # TypeScript (protobuf-ts)
    #

    logger.info("proto.regen.ts")
    shutil.rmtree(TEMP_TS_DIR, ignore_errors=True)
    Path(TEMP_TS_DIR).mkdir(parents=True, exist_ok=True)
    _shell(
        f"bun x protoc --ts_out {TEMP_TS_DIR} --proto_path . {LANG_PROTO} {EXTRA_PROTO_TS_FILES}",
    )

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
    message_type_map_str = (
        "export const MESSAGE_TYPE_BY_OBJECT_TYPE: Partial<Record<ObjectType, MessageType<any>>> = {\n"
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
    struct_mapping_parts = [
        "export interface StructTypeMapping extends Record<StructType, AnyStructData> {\n"
    ]
    for struct_t in STRUCT_TYPES:
        struct_cls = STRUCT_CLASS_BY_TYPE[struct_t]
        struct_mapping_parts.append(f"  [StructType.{struct_t.name}]: {struct_cls.__name__}Data,\n")
    struct_mapping_parts.append("}\n")
    struct_mapping_str = "".join(struct_mapping_parts)
    node_mapping_parts = [
        "export interface NodeTypeMapping extends Record<NodeType, AnyNodeData> {\n"
    ]
    for node_t in NODE_TYPES:
        node_cls = NODE_CLASS_BY_TYPE[node_t]
        node_mapping_parts.append(f"  [NodeType.{node_t.name}]: {node_cls.__name__}Data,\n")
    node_mapping_parts.append("}\n")
    node_mapping_str = "".join(node_mapping_parts)
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
    for cls in chain(NODE_CLASSES, STRUCT_CLASSES):
        props_strs: list[str] = []
        props = list(cls.__wired_properties__.values())
        for prop in sorted(props, key=lambda p: p.id):
            ts_name = to_casing(prop.name, Casing.CAMEL)
            ts_name = ts_name[0].lower() + ts_name[1:]
            props_strs.append(f"  {ts_name} = {prop.id},")
        property_enums_parts.append(
            f"export enum {cls.__name__}Property {{\n" + "\n".join(props_strs) + "\n}\n"
        )
    property_enums_str = "\n".join(property_enums_parts)
    property_enum_maps_parts: list[str] = []
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
        property_enum_maps_parts.append(property_enum_map_str)
    property_enum_maps_str = "\n".join(property_enum_maps_parts)

    object_info_type_str = """
export type PropertyKind = 'primitive' | 'enum' | 'reference';
export type PropertyInfo = {
    // basics
    id: number;
    name: string;
    component: ObjectType;
    kind: PropertyKind;
    primitiveType?: PrimitiveType;
    enumType?: EnumType;
    default?: any;
    
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
    secretValuePackedId?: number;
    
    // references
    referenceKind?: ReferenceKind;
    referenceNodes?: NodeType[];
    referenceStruct?: StructType;
}
    """
    type_info_definitions_parts = []
    for object_type in chain(STRUCT_TYPES, NODE_TYPES):
        bench_cls = BENCH_CLASS_BY_TYPE[object_type]
        prop_infos_strs: list[str] = []
        properties = list(bench_cls.__properties__.values())
        for prop in sorted(properties, key=lambda p: p.id or 0):
            if not prop.is_wired:
                continue

            prop_info_parts: dict[str, str] = {
                "id": str(prop.id),
                "name": repr(prop.name),
                "component": f"ObjectType.{bench_cls.metatype.name}",
            }
            if prop.reference_kind:
                kind = "reference"
            elif prop.is_enum:
                kind = "enum"
                enum_type = ENUM_TYPE_BY_CLASS.get(prop.py_type_stripped)
                if enum_type:
                    prop_info_parts["enumType"] = f"EnumType.{enum_type.name}"
            else:
                kind = "primitive"
            prop_info_parts["kind"] = repr(kind)
            if prop.primitive_type and prop.primitive_type is not UNSET:
                prop_info_parts["primitiveType"] = f"PrimitiveType.{prop.primitive_type.name}"

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
            if prop.secret_value_packed_ptr:
                assert isinstance(prop.secret_value_packed_ptr, Property)
                prop_info_parts["secretValuePackedId"] = str(prop.secret_value_packed_ptr.id)

            if prop.reference_kind:
                prop_info_parts["referenceKind"] = f"ReferenceKind.{prop.reference_kind.name}"
            if prop.reference_nodes:
                nodes_str_parts = [f"NodeType.{node.name}" for node in prop.reference_nodes]
                prop_info_parts["referenceNodes"] = f"[{', '.join(nodes_str_parts)}]"
            if prop.reference_struct:
                prop_info_parts["referenceStruct"] = f"StructType.{prop.reference_struct.name}"

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

    # map
    type_info_map_parts = [
        "export const PROPERTY_INFOS_BY_TYPE: Record<ObjectType, Record<any, PropertyInfo>> = {\n"
        "  [ObjectType.UNSPECIFIED]: {},\n"
    ]
    for object_type in chain(STRUCT_TYPES, NODE_TYPES):
        bench_cls = BENCH_CLASS_BY_TYPE[object_type]
        type_info_map_parts.append(
            f"  [ObjectType.{object_type.name}]: {bench_cls.__name__}DataInfo,\n"
        )
    type_info_map_parts.append("}\n")
    object_info_map_str = "".join(type_info_map_parts)

    patch_postfix_code = f"""
//
// Extra utility types
//

// Any...
export type AnyNodeData = {' | '.join(cls.__name__ + 'Data' for cls in NODE_CLASSES)}
export type AnyStructData = {' | '.join(cls.__name__ + 'Data' for cls in STRUCT_CLASSES)}
export type AnyNodeDataType = {' | '.join('typeof ' + cls.__name__ + 'Data' for cls in NODE_CLASSES)}
export type AnyStructDataType = {' | '.join('typeof ' + cls.__name__ + 'Data' for cls in STRUCT_CLASSES)}

// Ancestry maps
{ancestry_maps_str}

// Message types
{message_type_map_str}
{message_type_inv_map_str}
{enum_by_type_str}

// Type mappings
{struct_mapping_str}
{node_mapping_str}
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
{property_enum_maps_str}

// Type info
{object_info_type_str}
{object_info_definitions_str}
{object_info_map_str}
    """
    lang_ts = Path(TEMP_TS_DIR + "/bench/proto/lang.ts").read_text()
    Path(TEMP_TS_DIR + "/bench/proto/lang.ts").write_text(lang_ts + "\n\n" + patch_postfix_code)

    # index.ts
    Path(TEMP_TS_DIR + "/index.ts").write_text(
        """
// re-export generated wire files
export * from './bench/proto/common';
export * from './bench/proto/lang';
export * from './bench/proto/web';
export * from './bench/proto/services';
export * from './bench/proto/services.client';
export * from './google/protobuf/descriptor';
export * from './google/protobuf/struct';
export * from './google/protobuf/timestamp';
        """
    )

    # prepend every TS file in $TARGET_TS_DIR with /* eslint-disable */
    for path in Path(TEMP_TS_DIR).glob("**/*.ts"):
        path.write_text("/* eslint-disable */\n" + path.read_text())

    # amend every .client.ts file with our OperationOptions
    for path in Path(TEMP_TS_DIR).glob("**/*.client.ts"):
        patched_file = path.read_text().replace(": RpcOptions", ": OperationOptions")
        # append import
        patched_file = patched_file + '\nimport type { OperationOptions } from "@/proto/services";'
        path.write_text(patched_file)

    # overwrite WIRE_TS_DIR with TEMP_TS_DIR
    on_apply.append(
        lambda: (
            shutil.rmtree(WIRE_TS_DIR, ignore_errors=True),
            shutil.move(TEMP_TS_DIR, WIRE_TS_DIR),
        )
    )

    # and apply
    logger.info("proto.regen.apply")
    for apply in on_apply:
        apply()


@app.command()
def regen():
    start = time.time()
    schema_str = _generate_proto_schema()
    _regen_proto_artifacts(schema_str)
    logger.info("proto.generate", duration=time.time() - start)
