import re
import shutil
import subprocess
from pathlib import Path

import regex
import structlog
import typer

from destack.language import VERSION
from destack.language.registry import NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE
from destack.utils.string import Casing, to_casing

LANGUAGE_PROTO = "destack-proto/language.proto"
TEMP_PY_DIR = "destack-py/destack/proto.tmp"
TARGET_PY_DIR = "destack-py/destack/proto"
EXTRA_PROTO_PY_FILES = (
    "destack-proto/common.proto",
    "destack-proto/health.proto",
    "destack-proto/universe.proto",
    "destack-proto/space.proto",
    "destack-proto/google/type/date.proto",
    "destack-proto/google/type/datetime.proto",
    "destack-proto/google/type/timeofday.proto",
)


logger = structlog.get_logger(__name__)
app = typer.Typer(short_help="proto management")


def run_shell_sync(cmd: str, check=True, **kwargs):
    """Executes a shell command in a subprocess."""
    print(f"{cmd} {' '.join(f'{k}={v}' for k, v in kwargs.items())}")  # noqa: T201
    subprocess.run(cmd, shell=True, check=check, **kwargs)


def generate() -> None:
    """Regenerate external artifacts from the proto schema."""

    # NOTE: we copy the proto files into the temporary wire directory to ensure the import paths
    #  are correct for protobuf's python generator.
    Path(TEMP_PY_DIR).mkdir(parents=True, exist_ok=True)
    py_proto_files = [LANGUAGE_PROTO, *EXTRA_PROTO_PY_FILES]
    # replace 'import "destack-proto/..." with 'import "..." in all files in wire
    py_proto_files = [p.replace("destack-proto/", "") for p in py_proto_files]
    run_shell_sync("cp -r destack-proto wire")
    for path in Path("wire").rglob("*.proto"):
        path.write_text(regex.sub(r"import \"destack-proto/", 'import "', path.read_text()))
    run_shell_sync(
        "source destack-py/venv/bin/activate && "
        f"protoc -I wire "
        f"--python_out={TEMP_PY_DIR} "
        f"--pyi_out={TEMP_PY_DIR} "
        f"--grpclib_python_out={TEMP_PY_DIR} "
        f"{' '.join(py_proto_files)}"
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
                r"\1\2(self, request: '\3', session: 'Session', actor: 'IsActor | None', client: 'Client | None', metadata: 'RpcMetadata') -> '\4':",
                wire_py,
                flags=re.MULTILINE,
            )
            wire_py = regex.sub(
                r"async (def )(subscribe[a-zA-Z0-9_]*)\(self, stream: 'grpclib.server.Stream\[([a-zA-Z0-9_\.]+), ([a-zA-Z0-9_\.]+)\]'\) -> None:",
                r"\1\2(self, request: '\3', session: 'Session', actor: 'IsActor | None', client: 'Client | None', metadata: 'RpcMetadata') -> AsyncIterator['\4']:",
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
    from destack.language import Session, Session, IsActor, Client

"""
        path.write_text(patch_prefix_code + "\n\n" + wire_py)

    Path(TEMP_PY_DIR + "/__init__.py").write_text(f"""
# ruff: noqa

from typing import TYPE_CHECKING, Union

VERSION = '{VERSION}'

# import from all generated files
from .health_pb2 import *
from .common_pb2 import *
from .common_grpc import *
from .language_grpc import *
from .universe_grpc import *
from .space_grpc import *
from .health_grpc import *
from .language_pb2 import *
from .universe_pb2 import *
from .space_pb2 import *
from .google.type.date_pb2 import *
from .google.type.timeofday_pb2 import *
from .google.type.datetime_pb2 import *

# extra utility types
AnyNodeProto = Union[{", ".join([cls.__name__ + "Proto" for cls in NODE_CLASS_BY_TYPE.values()])}]
AnyStructProto = Union[{", ".join([cls.__name__ + "Proto" for cls in STRUCT_CLASS_BY_TYPE.values()])}]
AnyObjectProto = AnyNodeProto | AnyStructProto
""")
    shutil.rmtree(TARGET_PY_DIR, ignore_errors=True)
    shutil.copytree(TEMP_PY_DIR, TARGET_PY_DIR)
    shutil.rmtree(TEMP_PY_DIR, ignore_errors=True)
