import shutil
import subprocess
from pathlib import Path
from typing import Any

import regex

from destack.language import Enum
from destack.language.registry import NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE

from ..core import LANGUAGE_PROTO

EXTRA_PROTO_TS_FILES = (
    "destack-proto/common.proto",
    "destack-proto/health.proto",
    "destack-proto/universe.proto",
    "destack-proto/space.proto",
    "destack-proto/google/type/date.proto",
    "destack-proto/google/type/datetime.proto",
    "destack-proto/google/type/timeofday.proto",
)


TEMP_TS_DIR = "destack-ts/src/proto.tmp"
TARGET_TS_DIR = "destack-ts/src/proto"


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


def run_shell_sync(cmd: str, check=True, **kwargs):
    """Executes a shell command in a subprocess."""
    print(f"{cmd} {' '.join(f'{k}={v}' for k, v in kwargs.items())}")  # noqa: T201
    subprocess.run(cmd, shell=True, check=check, **kwargs)


def generate():
    """Generate the Typescript Proto code."""

    shutil.rmtree(TEMP_TS_DIR, ignore_errors=True)
    Path(TEMP_TS_DIR).mkdir(parents=True, exist_ok=True)
    run_shell_sync(
        f"bun x protoc --ts_out {TEMP_TS_DIR} --proto_path . {LANGUAGE_PROTO} "
        f"{' '.join(EXTRA_PROTO_TS_FILES)}",
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
export type AnyNodeProto = {" | ".join(cls.__name__ + "Proto" for cls in NODE_CLASS_BY_TYPE.values())}
export type AnyStructProto = {" | ".join(cls.__name__ + "Proto" for cls in STRUCT_CLASS_BY_TYPE.values())}

    """
    lang_ts = Path(TEMP_TS_DIR + "/destack-proto/language.ts").read_text()
    Path(TEMP_TS_DIR + "/destack-proto/language.ts").write_text(
        lang_ts + "\n\n" + patch_postfix_code
    )

    # index.ts
    Path(TEMP_TS_DIR + "/index.ts").write_text(
        """
// re-export generated wire files
export * from './destack-proto/common';
export * from './destack-proto/language';
export * from './destack-proto/universe';
export * from './destack-proto/universe.client';
export * from './destack-proto/space';
export * from './destack-proto/space.client';
export * from './destack-proto/health';
export * from './destack-proto/health.client';
export * from './destack-proto/google/type/date';
export * from './destack-proto/google/type/timeofday';
export * from './destack-proto/google/type/datetime';
export * from './google/protobuf/descriptor';
export * from './google/protobuf/struct';
export * from './google/protobuf/timestamp';
        """
    )

    # prepend every TS file in $TARGET_TS_DIR with /* eslint-disable */ and required imports
    for path in Path(TEMP_TS_DIR).glob("**/*.ts"):
        path.write_text("/* eslint-disable */\n" + path.read_text())

    # amend every .client.ts file with our OperationOptions
    for path in Path(TEMP_TS_DIR).glob("**/*.client.ts"):
        patched_file = path.read_text().replace(": RpcOptions", ": OperationOptions")
        # append import
        patched_file = patched_file + '\nimport type { OperationOptions } from "@/proto/services";'
        path.write_text(patched_file)

    # overwrite WIRE_TS_DIR with TEMP_TS_DIR
    shutil.rmtree(TARGET_TS_DIR, ignore_errors=True)
    shutil.move(TEMP_TS_DIR, TARGET_TS_DIR)
