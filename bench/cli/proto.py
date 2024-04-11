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
from bench.language.const import NODE_TYPES, STRUCT_TYPES
from bench.language.setup import (
    ANCESTOR_NODE_TYPES,
    CHILD_NODE_TYPES,
    DESCENDANT_NODE_TYPES,
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
    _shell(f"ruff {TEMP_PY_FILE} --fix", check=False, stdout=DEVNULL)
    _shell(f"pre-commit run black --files {TEMP_PY_FILE}", check=False, stdout=DEVNULL)
    _shell(f"mv {TEMP_PY_FILE} {WIRE_PY_FILE}")

    #
    # TypeScript (protobuf-ts)
    #

    logger.info("proto.regen.ts")
    shutil.rmtree(WIRE_TS_DIR, ignore_errors=True)
    Path(WIRE_TS_DIR).mkdir(parents=True, exist_ok=True)
    _shell(
        f"bun x protoc --ts_out {WIRE_TS_DIR} --proto_path . {LANG_PROTO} {EXTRA_PROTO_TS_FILES}",
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
        message_type_map_parts.append(f"  [BenchType.{cls.metatype.name}]: {cls.__name__}Data,\n")
    message_type_map_str = (
        "export const MESSAGE_TYPE_BY_BENCH_TYPE: Partial<Record<BenchType, MessageType<any>>> = {\n"
        + "".join(message_type_map_parts)
        + "}\n"
    )
    message_type_inv_map_str = (
        "export const BENCH_TYPE_BY_MESSAGE_TYPE_NAME: Record<string, BenchType> = {\n"
        + "".join(
            f'  ["symbolx.bench.{cls.__name__}Data"]: BenchType.{cls.metatype.name},\n'
            for cls in chain(NODE_CLASSES, STRUCT_CLASSES)
        )
        + "}\n"
    )

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
        "export interface AnyTypeMapping extends Record<BenchType, AnyStructData | AnyNodeData> {\n"
    ]
    for cls in chain(NODE_CLASSES, STRUCT_CLASSES):
        any_mapping_parts.append(f"  [BenchType.{cls.metatype.name}]: {cls.__name__}Data,\n")
    any_mapping_parts.append("}\n")
    any_mapping_str = "".join(any_mapping_parts)

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
                f"  [BenchType.{cls.metatype.name}]: {cls.__name__}Property,\n"
            )
        property_enum_map_str = (
            f"export const {upper_prefix}PROPERTY_ENUM_BY_TYPE: Partial<Record<BenchType, Any{camel_prefix}PropertyType>> = {{\n"
            + "".join(property_enum_map_parts)
            + "}\n"
        )
        property_enum_maps_parts.append(property_enum_map_str)
    property_enum_maps_str = "\n".join(property_enum_maps_parts)

    patch_postfix_code = f"""
//
// Extra utility types
//

// Any...
export type AnyNodeData = {' | '.join(cls.__name__ + 'Data' for cls in NODE_CLASSES)}
export type AnyStructData = {' | '.join(cls.__name__ + 'Data' for cls in STRUCT_CLASSES)}
export type AnyNodeDataType = {' | '.join('typeof ' + cls.__name__ + 'Data' for cls in NODE_CLASSES)}
export type AnyStructDataType = {' | '.join('typeof ' + cls.__name__ + 'Data' for cls in STRUCT_CLASSES)}

// type lists
export const BENCH_TYPES: BenchType[] = Object.values(BenchType).filter(v => typeof v === 'number' && v > 0) as BenchType[]
export const NODE_TYPES: NodeType[] = Object.values(NodeType).filter(v => typeof v === 'number' && v > 0) as NodeType[]
export const STRUCT_TYPES: StructType[] = Object.values(StructType).filter(v => typeof v === 'number' && v > 0) as StructType[]

// ancestry maps
{ancestry_maps_str}

// Message types
{message_type_map_str}
{message_type_inv_map_str}

// Type mappings
{struct_mapping_str}
{node_mapping_str}
{any_mapping_str}

// Property enums
{property_enums_str}
export type AnyNodeProperty = {' | '.join('typeof ' + cls.__name__ + 'Property' for cls in NODE_CLASSES)}
export type AnyStructProperty = {' | '.join('typeof ' + cls.__name__ + 'Property' for cls in STRUCT_CLASSES)}
export type AnyProperty = AnyNodeProperty | AnyStructProperty
export type AnyNodePropertyType = {' | '.join('typeof ' + cls.__name__ + 'Property' for cls in NODE_CLASSES)}
export type AnyStructPropertyType = {' | '.join('typeof ' + cls.__name__ + 'Property' for cls in STRUCT_CLASSES)}
export type AnyPropertyType = {' | '.join('typeof ' + cls.__name__ + 'Property' for cls in chain(NODE_CLASSES, STRUCT_CLASSES))}
{property_enum_maps_str}
    """
    lang_ts = Path(WIRE_TS_DIR + "/bench/proto/lang.ts").read_text()
    Path(WIRE_TS_DIR + "/bench/proto/lang.ts").write_text(lang_ts + "\n\n" + patch_postfix_code)
    Path(WIRE_TS_DIR + "/index.ts").write_text(
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
    for path in Path(WIRE_TS_DIR).glob("**/*.ts"):
        path.write_text("/* eslint-disable */\n" + path.read_text())

    # amend every .client.ts file with our OperationOptions
    for path in Path(WIRE_TS_DIR).glob("**/*.client.ts"):
        patched_file = path.read_text().replace(": RpcOptions", ": OperationOptions")
        # append import
        patched_file = patched_file + '\nimport type { OperationOptions } from "@/proto/services";'
        path.write_text(patched_file)


@app.command()
def regen():
    start = time.time()
    schema_str = _generate_proto_schema()
    _regen_proto_artifacts(schema_str)
    logger.info("proto.generate", duration=time.time() - start)
