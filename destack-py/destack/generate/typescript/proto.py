import shutil
import subprocess
from pathlib import Path

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


def run_shell_sync(cmd: str, check=True, **kwargs):
    """Executes a shell command in a subprocess."""
    print(f"{cmd} {' '.join(f'{k}={v}' for k, v in kwargs.items())}")  # noqa: T201
    subprocess.run(cmd, shell=True, check=check, **kwargs)


def generate():
    """Generate the Typescript Proto code."""

    for target_ts_dir, temp_ts_dir, extra_proto_ts_files, extra_flags in [
        ("destack-ts/src/proto", "destack-ts/src/proto.tmp", EXTRA_PROTO_TS_FILES, ""),
        (
            "destack-ts-system/src/proto",
            "destack-ts-system/src/proto.tmp",
            EXTRA_PROTO_TS_FILES,
            "--ts_opt server_grpc1 --ts_opt client_grpc1",
        ),
    ]:
        shutil.rmtree(temp_ts_dir, ignore_errors=True)
        Path(temp_ts_dir).mkdir(parents=True, exist_ok=True)
        run_shell_sync(
            f"bun x protoc --ts_out {temp_ts_dir} {extra_flags} --proto_path . {LANGUAGE_PROTO} "
            f"{' '.join(extra_proto_ts_files)}",
        )

        patch_postfix_code = f"""
    export type AnyNodeProto = {" | ".join(cls.__name__ + "Proto" for cls in NODE_CLASS_BY_TYPE.values())}
    export type AnyStructProto = {" | ".join(cls.__name__ + "Proto" for cls in STRUCT_CLASS_BY_TYPE.values())}
        """
        lang_ts = Path(temp_ts_dir + "/destack-proto/language.ts").read_text()
        Path(temp_ts_dir + "/destack-proto/language.ts").write_text(
            lang_ts + "\n\n" + patch_postfix_code
        )

        # index.ts
        Path(temp_ts_dir + "/index.ts").write_text(
            """
    // re-export generated proto files
    export * from './destack-proto/common';
    export * from './destack-proto/language';
    export * from './destack-proto/universe';
    export * from './destack-proto/space';
    export * from './destack-proto/health';
    export * from './destack-proto/google/type/date';
    export * from './destack-proto/google/type/timeofday';
    export * from './destack-proto/google/type/datetime';
    export * from './google/protobuf/descriptor';
    export * from './google/protobuf/struct';
    export * from './google/protobuf/timestamp';
            """
        )

        # prepend every TS file in $TARGET_TS_DIR with /* eslint-disable */ and required imports
        for path in Path(temp_ts_dir).glob("**/*.ts"):
            path.write_text("/* eslint-disable */\n" + path.read_text())

        # overwrite WIRE_TS_DIR with TEMP_TS_DIR
        shutil.rmtree(target_ts_dir, ignore_errors=True)
        shutil.move(temp_ts_dir, target_ts_dir)
