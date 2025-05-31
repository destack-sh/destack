import os
import re
import shutil
import subprocess
from enum import Enum
from pathlib import Path
from typing import Any

import regex
import structlog
import typer

from bench.language import (
    NODE_TYPES,
    VERSION,
)
from bench.language.registry import NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE
from bench.utils.string import Casing, to_casing

from .map import generate_proto_schema

LANGUAGE_PROTO = "bench-proto/language.proto"
TEMP_PY_DIR = "bench/pb2.tmp"
TARGET_PY_DIR = "bench/pb2"
TEMP_TS_DIR = "bench-ts/src/proto/wire.tmp"
TARGET_TS_DIR = "bench-ts/src/proto/wire"
EXTRA_PROTO_PY_FILES = (
    "bench-proto/common.proto",
    "bench-proto/health.proto",
    "bench-proto/supervisor.proto",
    "bench-proto/host.proto",
    "bench-proto/runtime.proto",
    "bench-proto/google/type/date.proto",
    "bench-proto/google/type/datetime.proto",
    "bench-proto/google/type/timeofday.proto",
)
EXTRA_PROTO_TS_FILES = (
    "bench-proto/common.proto",
    "bench-proto/health.proto",
    "bench-proto/supervisor.proto",
    "bench-proto/host.proto",
    "bench-proto/web.proto",
    "bench-proto/google/type/date.proto",
    "bench-proto/google/type/datetime.proto",
    "bench-proto/google/type/timeofday.proto",
)

logger = structlog.get_logger(__name__)
app = typer.Typer(short_help="proto management")


def run_shell_sync(cmd: str, check=True, **kwargs):
    """Executes a shell command in a subprocess."""
    cwd = os.getcwd()
    logger.trace("shell", cmd=cmd, cwd=cwd, check=check, **kwargs)
    subprocess.run(cmd, shell=True, check=check, **kwargs)


def _gen_proto_schema() -> str:
    """Generate the .proto schema (as a string) describing the current Bench types."""
    proto = generate_proto_schema(
        name="symbol.bench",
        unions={"SomeNode": ("node", NODE_TYPES)},
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


def _gen_proto(schema_str: str) -> None:
    """Regenerate external artifacts from the proto schema."""

    on_apply = []

    # regenerate python & TS proto files
    Path(LANGUAGE_PROTO).write_text(schema_str)

    #
    # Python
    #

    # NOTE: we copy the proto files into the temporary wire directory to ensure the import paths
    #  are correct for protobuf's python generator.
    Path(TEMP_PY_DIR).mkdir(parents=True, exist_ok=True)
    py_proto_files = [LANGUAGE_PROTO, *EXTRA_PROTO_PY_FILES]
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
                r"(?!.*subscribe)(async def )([a-zA-Z0-9_]+)\(self, stream: 'grpclib.server.Stream\[([a-zA-Z0-9_\.]+), ([a-zA-Z0-9_\.]+)\]'\) -> None:",
                r"\1\2(self, request: '\3', session: 'Session', subject: 'IsSubject | None', client: 'Client | None', metadata: 'RpcMetadata') -> '\4':",
                wire_py,
                flags=re.MULTILINE,
            )
            wire_py = regex.sub(
                r"async (def )(subscribe[a-zA-Z0-9_]*)\(self, stream: 'grpclib.server.Stream\[([a-zA-Z0-9_\.]+), ([a-zA-Z0-9_\.]+)\]'\) -> None:",
                r"\1\2(self, request: '\3', session: 'Session', subject: 'IsSubject | None', client: 'Client | None', metadata: 'RpcMetadata') -> AsyncIterator['\4']:",
                wire_py,
                flags=re.MULTILINE,
            )

        # rename XyzStub to XyzClient (stub is a bad name)
        wire_py = regex.sub(r"(?<!Service)Stub", "Client", wire_py)

        # prefix with imports
        patch_prefix_code = """
# type: ignore
# ruff: noqa

from typing import TYPE_CHECKING, Union, AsyncIterator, Mapping

if TYPE_CHECKING:
    from bench.language import Session, Session, IsSubject, Client

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
from .language_grpc import *
from .runtime_grpc import *
from .supervisor_grpc import *
from .host_grpc import *
from .health_grpc import *
from .language_pb2 import *
from .supervisor_pb2 import *
from .host_pb2 import *
from .google.type.date_pb2 import *
from .google.type.timeofday_pb2 import *
from .google.type.datetime_pb2 import *

# extra utility types
AnyNodeData = Union[{', '.join([cls.__name__ + 'Data' for cls in NODE_CLASS_BY_TYPE.values()])}]
AnyStructData = Union[{', '.join([cls.__name__ + 'Data' for cls in STRUCT_CLASS_BY_TYPE.values()])}]
AnyObjectData = AnyNodeData | AnyStructData
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
        f"bun x protoc --ts_out {TEMP_TS_DIR} --proto_path . {LANGUAGE_PROTO} {' '.join(EXTRA_PROTO_TS_FILES)}",
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

    patch_postfix_code = f"""
//
// Extra utility types
//

// Any...
export type AnyNodeData = {' | '.join(cls.__name__ + 'Data' for cls in NODE_CLASS_BY_TYPE.values())}
export type AnyStructData = {' | '.join(cls.__name__ + 'Data' for cls in STRUCT_CLASS_BY_TYPE.values())}

    """
    lang_ts = Path(TEMP_TS_DIR + "/proto/language.ts").read_text()
    Path(TEMP_TS_DIR + "/proto/language.ts").write_text(lang_ts + "\n\n" + patch_postfix_code)

    # index.ts
    Path(TEMP_TS_DIR + "/index.ts").write_text(
        """
// re-export generated wire files
export * from './proto/common';
export * from './proto/language';
export * from './proto/web';
export * from './proto/supervisor';
export * from './proto/supervisor.client';
export * from './proto/host';
export * from './proto/host.client';
export * from './proto/health';
export * from './proto/health.client';
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
