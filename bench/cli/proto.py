import re
import shutil
import time
from pathlib import Path
from subprocess import DEVNULL

import structlog
import typer

from bench.cli.utils import _shell
from bench.language import VERSION, Node
from bench.language.setup import FINAL_BENCH_CLASSES, NODE_CLASSES, STRUCT_CLASSES
from bench.proto.engine import generate_proto_schema

LANG_PROTO = "bench/proto/lang.proto"
TEMP_PY_DIR = "bench/proto/wire.tmp"
TEMP_PY_FILE = "bench/proto/wire.py.tmp"
WIRE_PY_FILE = "bench/proto/wire.py"
WIRE_TS_DIR = "bench-web/src/proto/wire"
EXTRA_PROTO_FILES = "bench/proto/common.proto bench/proto/services.proto"

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
        unions={"SomeNode": ("node", node_classes)},
        extras=[],
        message_postfix="Data",
    )
    return proto.to_proto_source()


def _regen_proto_artifacts(schema_str: str) -> None:
    """Regenerate external artifacts from the proto schema."""
    # regenerate python & TS proto files
    Path(LANG_PROTO).write_text(schema_str)

    # python
    logger.info("proto.regen.py")
    Path(TEMP_PY_FILE).unlink(missing_ok=True)
    Path(TEMP_PY_DIR).mkdir(parents=True, exist_ok=True)
    _shell(
        f"protoc -I . --python_betterproto_out={TEMP_PY_DIR} {LANG_PROTO} {EXTRA_PROTO_FILES}",
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
    patch_prefix_code = f"""
from typing import TYPE_CHECKING

VERSION = '{VERSION}'

if TYPE_CHECKING:
    from bench.language import Subject
"""
    patch_postfix_code = f"""
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
    logger.info("proto.regen.py.done")

    # TS
    logger.info("proto.regen.ts")
    shutil.rmtree(WIRE_TS_DIR, ignore_errors=True)
    Path(WIRE_TS_DIR).mkdir(parents=True, exist_ok=True)
    _shell(
        f"bun x protoc --ts_out {WIRE_TS_DIR} --proto_path . {LANG_PROTO} {EXTRA_PROTO_FILES}",
    )
    # patch
    patch_postfix_code = f"""
// extra utility types
export type AnyNodeData = {' | '.join(cls.__name__ + 'Data' for cls in NODE_CLASSES)}
export type AnyStructData = {' | '.join(cls.__name__ + 'Data' for cls in STRUCT_CLASSES)}
"""
    lang_ts = Path(WIRE_TS_DIR + "/bench/proto/lang.ts").read_text()
    Path(WIRE_TS_DIR + "/bench/proto/lang.ts").write_text(lang_ts + "\n\n" + patch_postfix_code)
    Path(WIRE_TS_DIR + "/index.ts").write_text(
        """
// re-export generated wire files
export * from './bench/proto/common';
export * from './bench/proto/lang';
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
    logger.info("proto.regen.ts.done")


@app.command()
def regen():
    logger.info("proto.generate")
    start = time.time()
    schema_str = _generate_proto_schema()
    _regen_proto_artifacts(schema_str)
    logger.info("proto.generate.done", duration=time.time() - start)
