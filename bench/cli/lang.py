import shutil
import time
from pathlib import Path
from subprocess import DEVNULL
from typing import Optional

import structlog
import typer
from more_itertools import first
from rich import print

from bench.cli.utils import _async_to_sync_blocking, _shell
from bench.language.const import VERSION, NodeType
from bench.language.node import (
    BENCH_CLASSES,
    NODE_CLASS_BY_NODE_TYPE,
    NODE_CLASSES,
    STRUCT_CLASSES,
    Bench,
    Node,
)
from bench.proto.core import Field, Message
from bench.proto.engine import generate_proto_schema
from bench.server.session import detached_session
from bench.sql.client import async_pg_cursor
from bench.sql.core import DEFAULT_GLOBAL_TABLES, DEFAULT_LOCAL_TABLES
from bench.sql.engine import map_node_type_to_pg_table
from bench.sql.migration import (
    Migration,
    add_migration_to_fs,
    generate_migration_code,
    generate_migration_ops,
    introspect_tables_from_pg,
    migrate_to,
    read_migrations_from_fs,
)
from bench.utils.utils import DEBUG, format_python

logger = structlog.get_logger(__name__)
app = typer.Typer(short_help="language state and migrations")

TARGET_PY_DIR = "bench/proto/wire"
TARGET_PY_FILE = TARGET_PY_DIR + ".py"
TARGET_TS_DIR = "frontend/src/proto/wire"
GENERATED_PROTO_FILE = "bench/proto/lang.proto"
EXTRA_PROTO_FILES = "bench/proto/services.proto"


def _generate_proto_schema() -> str:
    """Generate the .proto schema (as a string) describing the current Bench types."""
    proto = generate_proto_schema(
        name="symbolx.bench",
        bench_classes=[*BENCH_CLASSES, Node],
        aliases={Node: "BaseNode"},
        unions={"SomeNode": ("node", NODE_CLASSES), "SomeStruct": ("struct", STRUCT_CLASSES)},
        extras=[
            Message(
                name="ModuleTreeData",
                fields=[
                    Field(id=1, name="module", type="ModuleData"),
                    Field(id=2, name="nodes", type="SomeNodeData", repeated=True),
                ],
            ),
            Message(
                name="SomeNodePointer",
                fields=[
                    Field(id=1, name="metatype", type="NodeType"),
                    Field(id=2, name="id", type="string"),
                    Field(id=3, name="ck", type="string"),
                ],
            ),
            Message(
                name="AbsoluteNodePointer",
                fields=[
                    Field(id=1, name="metatype", type="NodeType"),
                    Field(id=2, name="id", type="string"),
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
        Path(TARGET_PY_FILE).unlink(missing_ok=True)
        Path(TARGET_PY_DIR).mkdir(parents=True, exist_ok=True)
        _shell(
            f"protoc -I . --python_betterproto_out={TARGET_PY_DIR} {GENERATED_PROTO_FILE} {EXTRA_PROTO_FILES}",
        )
        _shell(f"mv {TARGET_PY_DIR}/symbolx/bench/__init__.py {TARGET_PY_FILE}")
        Path(TARGET_PY_FILE).write_text(
            Path(TARGET_PY_FILE).read_text()
            # append AnyNodeData/AnyStructData
            + "\n\nfrom typing import Union # noqa\n"
            + f"AnyNodeData = Union[{', '.join([cls.__name__ + 'Data' for cls in NODE_CLASSES])}]\n"
            + f"AnyStructData = Union[{', '.join([cls.__name__ + 'Data' for cls in STRUCT_CLASSES])}]"
            # append VERSION
            + f"\n\nVERSION = '{VERSION}'"
        )
        shutil.rmtree(TARGET_PY_DIR, ignore_errors=True)
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
def proto(regen: bool = False):
    logger.info("proto.generate")
    start = time.time()
    schema_str = _generate_proto_schema()

    if not regen:
        print(schema_str)
    else:
        _regen_proto_artifacts(schema_str)
    logger.info("proto.generate.done", duration=time.time() - start)


def _generate_pg_schema():
    """Generate the .py custom SQL schema describing the current Bench types."""
    chunks: list[str] = [
        # standardized generated notice
        "# This file was automatically generated by Bench. Do not edit.",
        "from bench.sql.core import Table, CascadeAction, Column, ColumnType, Constraint, ConstraintType, Index, IndexType",
        # append VERSION
        f'VERSION = "{VERSION}"',
    ]
    for node_t in NodeType:
        node_cls = NODE_CLASS_BY_NODE_TYPE[node_t]
        if node_cls.__is_stored__ and not node_cls.__is_stored_custom__:
            table = map_node_type_to_pg_table(node_cls)
            const_name = f"{node_cls.metatype.name}_TABLE"
            table_def = f"{const_name} = {table.source_repr()}"
            chunks.append(table_def)
    source = "\n\n".join(chunks)
    source = format_python(source)
    return source


@app.command()
def sql(regen: bool = False):
    logger.info("sql.generate")
    start = time.time()
    source = _generate_pg_schema()

    if not regen:
        print(source)
    else:
        Path("bench/sql/schema.py").write_text(source)
    logger.info("sql.generate.done", duration=time.time() - start)


@app.command(help="generate global AND local SQL migrations")
@_async_to_sync_blocking
async def makemigrations(
    bench: str = typer.Option(default="symbolx.bench", help="the bench to use as local reference"),
    local_pg_name: Optional[str] = typer.Option(
        default=None, help="the bench to use as local reference (bypass lookup via bench)"
    ),
    no_downgrade: bool = typer.Option(default=False, help="exclude downgrade operations"),
):
    logger.info("makemigrations", bench=bench, local_pg_name=local_pg_name)
    start = time.time()

    if not local_pg_name:
        async with detached_session(read_only=True):
            bench = await Bench.get(slug=bench)
            local_pg_name = bench.pg_name

    # introspect current/old tables from DB, get new from code
    async with async_pg_cursor() as cur:
        old_global_tables = await introspect_tables_from_pg(cur)
    async with async_pg_cursor(local_pg_name=local_pg_name) as cur:
        old_local_tables = await introspect_tables_from_pg(cur)
    node_global_tables = [
        node.__table__
        for node in NODE_CLASS_BY_NODE_TYPE.values()
        if not node.__is_local__ and node.__table__ is not None
    ]
    new_global_tables = [*DEFAULT_GLOBAL_TABLES, *node_global_tables]
    node_local_tables = [
        node.__table__
        for node in NODE_CLASS_BY_NODE_TYPE.values()
        if node.__is_local__ and node.__table__ is not None
    ]
    new_local_tables = [*DEFAULT_LOCAL_TABLES, *node_local_tables]

    # generate migration
    global_migration_ops = generate_migration_ops(old_global_tables, new_global_tables)
    local_migration_ops = generate_migration_ops(old_local_tables, new_local_tables)

    if not global_migration_ops and not local_migration_ops:
        logger.info("makemigrations.noop")
        return

    known_migrations = read_migrations_from_fs()
    conflicting_migration = first((m for m in known_migrations if m.version == VERSION), None)
    if conflicting_migration:
        raise RuntimeError(f"existing migration for version {VERSION}: {conflicting_migration}")

    latest_migration = max(known_migrations, key=lambda m: m.id, default=None)
    new_migration = Migration(
        id=latest_migration.id + 1 if latest_migration is not None else 1,
        version=VERSION,
        has_global=bool(global_migration_ops),
        has_local=bool(local_migration_ops),
        applied_at=None,
    )
    # which cursor we pass doesn't matter, it's just used for formatting SQL
    migration_code = generate_migration_code(
        new_migration,
        cur=cur,
        global_ops=global_migration_ops,
        local_ops=local_migration_ops,
        exclude_inverse=no_downgrade,
    )
    add_migration_to_fs(migration=new_migration, code=migration_code)

    logger.info("makemigrations.done", duration=time.time() - start)


@app.command(help="apply global OR local SQL migrations")
@_async_to_sync_blocking
async def migrate(
    target: Optional[str] = typer.Option(
        default=None, help="the migration to migrate to [default=latest]"
    ),
    bench: Optional[str] = typer.Option(
        default=None, help="the local bench to migrate, global otherwise"
    ),
):
    logger.info("migrate")
    start = time.time()

    # resolve local_pg_name (determine local/global migration)
    if bench is not None:
        async with detached_session(read_only=True):
            bench = await Bench.get(slug=bench)
            local_pg_name = bench.pg_name
    else:
        local_pg_name = None

    # get target migrations from our source of truth (local file system)
    all_migrations = read_migrations_from_fs()
    if target:
        target_migration = first(
            (m for m in all_migrations if str(m.id) == target or m.version == target), None
        )
        if target_migration is None:
            raise ValueError(f"unknown migration '{target}': {all_migrations}")
    else:
        if len(all_migrations) == 0:
            raise ValueError("no migrations found")
        target_migration = all_migrations[-1]

    async with async_pg_cursor(local_pg_name=local_pg_name) as cur:
        await migrate_to(
            cur=cur,
            all_migrations=all_migrations,
            target_migration=target_migration,
            is_global=bench is None,
        )

    logger.info("migrate.done", duration=time.time() - start)


if DEBUG:

    @app.command(help="apply local SQL migrations to ALL benches")
    @_async_to_sync_blocking
    async def migrate_all_local(
        to: Optional[str] = typer.Option(
            default=None, help="the migration to migrate to [default=latest]"
        )
    ):
        async with detached_session(read_only=True):
            for bench in await Bench.all():
                await migrate_to(target=to, bench=bench.slug)
