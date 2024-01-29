import shutil
import time
from pathlib import Path
from subprocess import DEVNULL

import structlog
import typer

from bench.cli.utils import _shell
from bench.language import VERSION
from bench.language.node import FINAL_BENCH_CLASSES, NODE_CLASSES, STRUCT_CLASSES, Node
from bench.proto.core import Field, Message
from bench.proto.engine import generate_proto_schema

TARGET_PY_DIR = "bench/proto/wire"
TARGET_PY_FILE = TARGET_PY_DIR + ".py"
TARGET_TS_DIR = "frontend/src/proto/wire"
GENERATED_PROTO_FILE = "bench/proto/lang.proto"
EXTRA_PROTO_FILES = "bench/proto/services.proto"

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
        extras=[
            Message(
                name="PackageTreeData",
                fields=[
                    Field(id=1, name="package", type="PackageData"),
                    Field(id=2, name="nodes", type="SomeNodeData", repeated=True),
                ],
            ),
        ],
        message_postfix="Data",
    )
    return proto.to_proto_source()


def _regen_proto_artifacts(schema_str: str) -> None:
    """Regenerate external artifacts from the proto schema."""
    # regenerate python & TS proto files
    Path(GENERATED_PROTO_FILE).write_text(schema_str)

    # python
    logger.info("proto.regen.py")
    try:
        # backup existing target
        # (only needed for Python since we need the source to compile to regenerate to retry)
        shutil.copy(TARGET_PY_FILE, TARGET_PY_FILE + ".bak")

        # generate
        Path(TARGET_PY_FILE).unlink(missing_ok=True)
        Path(TARGET_PY_DIR).mkdir(parents=True, exist_ok=True)
        _shell(
            f"protoc -I . --python_betterproto_out={TARGET_PY_DIR} {GENERATED_PROTO_FILE} {EXTRA_PROTO_FILES}",
        )
        _shell(f"mv {TARGET_PY_DIR}/symbolx/bench/__init__.py {TARGET_PY_FILE}")

        # add/patch our extra stuff
        betterproto_code = Path(TARGET_PY_FILE).read_text()
        patch_postfix_code = f"""
import bench.proto.monkey # noqa

from typing import Union # noqa
AnyNodeData = Union[{', '.join([cls.__name__ + 'Data' for cls in NODE_CLASSES])}]
AnyStructData = Union[{', '.join([cls.__name__ + 'Data' for cls in STRUCT_CLASSES])}]

VERSION = '{VERSION}'
        """
        Path(TARGET_PY_FILE).write_text(betterproto_code + "\n\n" + patch_postfix_code)

        # and fix it up
        shutil.rmtree(TARGET_PY_DIR, ignore_errors=True)
        _shell(f"ruff {TARGET_PY_FILE} --fix", check=False, stdout=DEVNULL)
        _shell(f"pre-commit run black --files {TARGET_PY_FILE}", check=False, stdout=DEVNULL)
    except Exception as e:
        # restore backup
        Path(TARGET_PY_FILE).unlink(missing_ok=True)
        shutil.copy(TARGET_PY_FILE + ".bak", TARGET_PY_FILE)
        raise e
    finally:
        Path(TARGET_PY_FILE + ".bak").unlink(missing_ok=True)
    logger.info("proto.regen.py.done")

    # TS
    logger.info("proto.regen.ts")
    shutil.rmtree(TARGET_TS_DIR, ignore_errors=True)
    Path(TARGET_TS_DIR).mkdir(parents=True, exist_ok=True)
    _shell(
        f"npx protoc --ts_out {TARGET_TS_DIR} --ts_opt long_type_string --proto_path . {GENERATED_PROTO_FILE} {EXTRA_PROTO_FILES}",
    )
    # prepend every TS file in $TARGET_TS_DIR with /* eslint-disable */
    for path in Path(TARGET_TS_DIR).glob("**/*.ts"):
        path.write_text("/* eslint-disable */\n" + path.read_text())
    logger.info("proto.regen.ts.done")


@app.command()
def regen():
    logger.info("proto.generate")
    start = time.time()
    schema_str = _generate_proto_schema()
    _regen_proto_artifacts(schema_str)
    logger.info("proto.generate.done", duration=time.time() - start)
