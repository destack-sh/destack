import asyncio
import enum
import importlib
import os
import re
from collections import defaultdict
from dataclasses import dataclass, replace
from datetime import datetime
from pathlib import Path
from textwrap import indent
from typing import (
    TYPE_CHECKING,
    Any,
    Awaitable,
    Callable,
    Collection,
    Mapping,
    Optional,
    cast,
)

import psycopg
import structlog
from more_itertools import first
from psycopg import sql

from bench.sql.core import (
    MIGRATION_TABLE,
    POSTGRES_TYPE_BY_UDT,
    PRIMITIVE_TYPE_BY_POSTGRES_TYPE,
    CascadeAction,
    Column,
    Constraint,
    ConstraintType,
    Extension,
    Index,
    IndexType,
    Object,
    ObjectKind,
    PostgresColumnType,
    Schema,
    Table,
    TableObject,
)
from bench.sql.engine import (
    SqlUndefinedObjectError,
    pg_delete,
    pg_select,
    pg_select_raw,
    pg_upsert,
    sqlstr,
)
from bench.utils.dt import LOCAL_TZ, monotime, utcnow
from bench.utils.env import REPOSITORY_PATH
from bench.utils.func import partition, re_search_or_error
from bench.utils.utils import format_python

if TYPE_CHECKING:
    from bench.language import Store

MIGRATIONS_PATH = REPOSITORY_PATH / "bench/migrations"
MIGRATIONS_TEMPLATE_PATH = REPOSITORY_PATH / "bench/migrations/0000_template.py"

EXTENSIONS = ("pgcrypto",)

logger = structlog.get_logger(__name__)


#
# Managing migrations
#


@dataclass
class Migration:
    id: int
    version: str
    has_global: bool
    has_local: bool
    applied_at: Optional[datetime]
    path: Optional[Path] = None  # not stored
    file: Optional["MigrationFile"] = None  # not stored

    def __str__(self) -> str:
        return (
            f"{self.id} {self.version} (has_global={self.has_global}, has_local={self.has_local})"
        )

    def __repr__(self) -> str:
        return f"<Migration {self}>"


MigratorFunc = Callable[[psycopg.AsyncConnection], Awaitable[None]]


@dataclass
class MigrationFile:
    path: Path
    module: Any


def pack_migration_row(migration: Migration) -> dict[str, Any]:
    return {
        "id": migration.id,
        "version": migration.version,
        "has_global": migration.has_global,
        "has_local": migration.has_local,
        "applied_at": migration.applied_at,
    }


def unpack_migration_row(row: Mapping[str, Any]) -> Migration:
    return Migration(
        id=row["id"],
        version=row["version"],
        has_global=row["has_global"],
        has_local=row["has_local"],
        applied_at=row["applied_at"],
    )


async def read_migrations_from_pg(
    cur: psycopg.AsyncCursor, *, applied: bool | None = None
) -> list[Migration]:
    """Reads the 'bench_migration' table (if it exists) and returns the corresponding Migration."""
    try:
        if applied is not None:
            where = sql.SQL("applied_at IS NOT NULL") if applied else sql.SQL("applied_at IS NULL")
        else:
            where = None
        migrations_rows = await pg_select(cur, MIGRATION_TABLE, where=where, order_by=sql.SQL("id"))
        migrations = [unpack_migration_row(row) for row in migrations_rows]
        return migrations
    except SqlUndefinedObjectError:
        await cur.connection.rollback()
        return []


async def _write_migrations_to_pg(cur: psycopg.AsyncCursor, migrations: list[Migration]):
    """Upserts the given migrations into the table. Errors if the table doesn't exist."""
    migrations_rows = [pack_migration_row(m) for m in migrations]
    await pg_upsert(cur, MIGRATION_TABLE, migrations_rows)


async def delete_migrations_in_pg(cur: psycopg.AsyncCursor, from_id: int, to_id: int) -> None:
    """Deletes migrations from the database."""
    migrations_rows = await pg_delete(
        cur,
        MIGRATION_TABLE,
        where=sqlstr(f"id >= {from_id} AND id <= {to_id}"),
        returning=MIGRATION_TABLE.columns,
    )
    assert migrations_rows is not None
    migrations = [unpack_migration_row(row) for row in migrations_rows]
    if migrations:
        logger.warning("migration.delete", migrations=migrations)


def read_migrations_from_fs() -> list[Migration]:
    """Reads the available migrations from local filesystem. Actually loads each migration file."""
    migrations: list[Migration] = []
    for migration_file in sorted(os.listdir(MIGRATIONS_PATH)):
        if migration_file in ("0000_template.py", "__init__.py") or not migration_file.endswith(
            ".py"
        ):
            continue

        # parse the file
        migration_path = MIGRATIONS_PATH / migration_file
        migration_code = Path(migration_path).read_text()
        migration_metadata: dict[str, str] = {
            match[0]: match[1] for match in re.findall(r"([A-Z_]+) = (.*)", migration_code)
        }
        migration = Migration(
            id=int(migration_metadata["ID"]),
            version=migration_metadata["VERSION"][1:-1],
            has_global=migration_metadata["HAS_GLOBAL"] == "True",
            has_local=migration_metadata["HAS_LOCAL"] == "True",
            applied_at=None,
            path=migration_path,
        )
        migrations.append(migration)
    migrations = sorted(migrations, key=lambda m: m.id)
    return migrations


def delete_migrations_in_fs(from_id: int, to_id: int) -> None:
    """Deletes migrations from the local filesystem."""
    for migration_file in Path.glob(Path(MIGRATIONS_PATH), "*.py"):
        if migration_file.stem in (
            "0000_template",
            "__init__",
        ) or not migration_file.name.endswith(".py"):
            continue
        logger.warning("migration.delete", migration_file=migration_file)
        migration_id = int(migration_file.name.split("_")[0])
        if from_id <= migration_id <= to_id:
            migration_file.unlink()


MIGRATIONS = read_migrations_from_fs()


def has_migration_after(version_a: str, *, is_global: bool) -> bool:
    """Returns whether there is a migration between the two versions."""
    for migration in MIGRATIONS:
        if (is_global and not migration.has_global) or (not is_global and not migration.has_local):
            continue
        if version_a < migration.version:
            return True
    return False


def _load_migration_from_path(migration: Migration) -> MigrationFile:
    assert migration.path is not None, "migration path not set"
    current_path = Path(__file__).parent.parent.parent
    module_path = str(migration.path)[len(str(current_path)) + 1 : -3].replace("/", ".")
    migration_module = importlib.import_module(module_path)
    file = MigrationFile(path=migration.path, module=migration_module)
    return file


async def sql_migrate(
    cur: psycopg.AsyncCursor,
    target: str | int | None,
    *,
    is_global: bool,
    store: Optional["Store"] = None,
) -> list[Migration]:
    """
    Applies missing migrations (up or down) to reach the target migration.
    Also updates the migrations table.
    """

    start = monotime()

    # get target migrations from our source of truth (local file system)
    if target:
        for m in MIGRATIONS:
            if m.id == target or str(m.id) == target or m.version == target:
                target_migration = m
                break
        else:
            target_migration = MIGRATIONS[-1]
    else:
        if len(MIGRATIONS) == 0:
            raise ValueError("no migrations found")
        target_migration = MIGRATIONS[-1]

    logger.trace(
        "migrations.load", target_migration=target_migration, is_global=is_global, store=store
    )
    stored_migrations = await read_migrations_from_pg(cur)
    is_upgrade = all(target_migration.id > m.id for m in stored_migrations if m.applied_at)
    log = logger.bind(target_migration=target_migration, is_upgrade=is_upgrade, is_global=is_global)
    applied_migrations = [m for m in stored_migrations if m.applied_at is not None]
    current_migration = max(applied_migrations, key=lambda m: m.id) if applied_migrations else None
    current_migration_id = current_migration.id if current_migration else -1

    # get the migrations to apply
    migrations_to_apply = []
    for migration in MIGRATIONS:
        if (is_global and not migration.has_global) or (not is_global and not migration.has_local):
            continue
        if (is_upgrade and current_migration_id < migration.id <= target_migration.id) or (
            not is_upgrade and current_migration_id >= migration.id > target_migration.id
        ):
            migrations_to_apply.append(migration)

    # apply the migrations
    if not migrations_to_apply:
        log.debug("migrations.apply.skip", store=store, duration=monotime() - start)
        return []
    else:
        await _do_sql_migrate(
            cur, migrations_to_apply, is_upgrade=is_upgrade, is_global=is_global, store=store
        )
        log.debug(
            "migrations.apply",
            cur=cur,
            migrations=migrations_to_apply,
            duration=monotime() - start,
            store=store,
        )

    # update the migration table (applied + missing)
    missing_migrations = [
        m
        for m in MIGRATIONS
        if not any(m.id == s.id for s in stored_migrations)
        and not any(m.id == a.id for a in applied_migrations)
    ]
    migrations_to_update = [*migrations_to_apply, *missing_migrations]
    if migrations_to_update:
        await _write_migrations_to_pg(cur, migrations_to_update)

    return migrations_to_apply


async def _do_sql_migrate(
    cur: psycopg.AsyncCursor,
    migrations: Collection[Migration],
    *,
    is_upgrade: bool,
    is_global: bool,
    store: Optional["Store"] = None,
):
    """Applies the given migrations in the given order."""
    now = utcnow()
    for migration in migrations:
        start = monotime()
        func_name = (
            f"{(is_upgrade and 'upgrade') or 'downgrade'}_{(is_global and 'global') or 'local'}"
        )
        migration_file = _load_migration_from_path(migration)
        func = getattr(migration_file.module, func_name)
        try:
            await func(cur)
        except Exception as e:
            logger.error(
                "migration.apply.error", cur=cur, migration=migration, store=store, error=e
            )
            raise
        if is_upgrade:
            migration.applied_at = now
        else:
            migration.applied_at = None
        logger.debug(
            "migration.apply",
            cur=cur,
            migration=migration,
            store=store,
            duration=monotime() - start,
        )


#
# Generating migrations
#


class MigrationOpKind(enum.Enum):
    CREATE = "CREATE"
    RENAME = "RENAME"
    UPDATE = "UPDATE"
    DELETE = "DELETE"


@dataclass
class MigrationOp:
    kind: MigrationOpKind
    new_object: Optional[Object]
    old_object: Optional[Object]
    diff_keys: Optional[tuple[str, ...]] = None

    def __str__(self) -> str:
        op_str = f"{self.kind.value} {self.object_kind.name}"
        if self.kind == MigrationOpKind.CREATE:
            assert self.new_object is not None, f"new_object not set for {self!r}"
            return f"{op_str} {self.new_object.qualified_name}"
        elif self.kind == MigrationOpKind.RENAME:
            assert self.old_object is not None, f"old_object not set for {self!r}"
            assert self.new_object is not None, f"new_object not set for {self!r}"
            return f"{op_str} {self.old_object.qualified_name} -> {self.new_object.qualified_name}"
        elif self.kind == MigrationOpKind.UPDATE:
            assert self.diff_keys is not None, f"diff_keys not set for {self!r}"
            assert self.new_object is not None, f"new_object not set for {self!r}"
            diff_str = ", ".join(
                f"{k}:{getattr(self.old_object, k)}->{getattr(self.new_object, k)}"
                for k in self.diff_keys
            )
            return f"{op_str} {self.new_object.qualified_name} ({diff_str})"
        elif self.kind == MigrationOpKind.DELETE:
            assert self.old_object is not None, f"old_object not set for {self!r}"
            return f"{op_str} {self.old_object.qualified_name}"
        else:
            raise RuntimeError(f"unexpected migration op type: {self.kind}")

    def __repr__(self) -> str:
        return f"<MigrationOp {self}>"

    @property
    def object_kind(self) -> ObjectKind:
        if self.new_object is not None:
            return self.new_object.kind
        elif self.old_object is not None:
            return self.old_object.kind
        else:
            raise RuntimeError(f"no object set for {self!r}")

    @property
    def table(self) -> "Table":
        if self.new_object is not None:
            return cast("TableObject", self.new_object).table
        elif self.old_object is not None:
            return cast("TableObject", self.old_object).table
        else:
            raise RuntimeError(f"no object set for {self!r}")

    def invert(self) -> "MigrationOp":
        """Returns the inverse of this operation for undoing migrations."""
        if self.kind == MigrationOpKind.CREATE:
            return MigrationOp(MigrationOpKind.DELETE, None, self.new_object)
        elif self.kind == MigrationOpKind.RENAME:
            return MigrationOp(MigrationOpKind.RENAME, self.old_object, self.new_object)
        elif self.kind == MigrationOpKind.UPDATE:
            return MigrationOp(
                MigrationOpKind.UPDATE, self.old_object, self.new_object, self.diff_keys
            )
        elif self.kind == MigrationOpKind.DELETE:
            return MigrationOp(MigrationOpKind.CREATE, self.old_object, None)
        raise RuntimeError(f"unexpected migration op type: {self.kind}")


def generate_sql_migration_ops(old_schema: Schema, new_schema: Schema) -> list[MigrationOp]:
    """Generates the migration operations to go from the old tables to the new tables."""

    # extensions
    extension_ops: list[MigrationOp] = []
    old_extensions = {ext.name for ext in old_schema.extensions}
    new_extensions = {ext.name for ext in new_schema.extensions}
    for ext_name in new_extensions - old_extensions:
        extension_ops.append(MigrationOp(MigrationOpKind.CREATE, Extension(ext_name), None))
    # NOTE: we don't remove extensions for now

    def _to_id(obj: TableObject) -> str:
        return obj.qualified_name

    # order of walk is table -> column -> index -> constraint
    old_table_objects = [obj for table in old_schema.tables for obj in table.walk()]
    old_table_objects_by_id = {_to_id(obj): obj for obj in old_table_objects}
    new_table_objects = [obj for table in new_schema.tables for obj in table.walk()]
    new_table_objects_by_id = {_to_id(obj): obj for obj in new_table_objects}
    deleted_table_objects = tuple(
        obj for obj in old_table_objects if _to_id(obj) not in new_table_objects_by_id
    )

    # diff objects
    deleted_ids: set[str] = set()
    deleted_table_ops: list[MigrationOp] = []
    for old_object in old_table_objects:
        old_id = _to_id(old_object)
        new_object = new_table_objects_by_id.get(old_id)
        if new_object is None:
            if _to_id(old_object.table) in deleted_ids:
                continue  # skip, table deleted
            if old_object.kind == ObjectKind.INDEX:
                # skip if owning constraint is also deleted
                if any(
                    obj.kind == ObjectKind.CONSTRAINT and obj.name == old_object.qualified_name
                    for obj in deleted_table_objects
                ):
                    continue
            deleted_table_ops.append(MigrationOp(MigrationOpKind.DELETE, None, old_object))
            deleted_ids.add(old_id)
    # regular order: table -> column -> index -> constraint
    cru_ops: list[MigrationOp] = []
    for new_object in new_table_objects:
        new_id = _to_id(new_object)
        old_object = old_table_objects_by_id.get(new_id)
        if old_object is None:
            # create columns only if parent table isn't new
            if (
                new_object.kind == ObjectKind.COLUMN
                and _to_id(new_object.table) in new_table_objects_by_id
                and _to_id(new_object.table) not in old_table_objects_by_id
            ):
                continue
            cru_ops.append(MigrationOp(MigrationOpKind.CREATE, new_object, None))
        elif new_object.name != old_object.name:
            cru_ops.append(MigrationOp(MigrationOpKind.RENAME, new_object, old_object))
        elif old_object.hash_flat() != new_object.hash_flat():
            diff = new_object.diff_keys(old_object)
            if not diff:
                continue  # hashing changed
            cru_ops.append(MigrationOp(MigrationOpKind.UPDATE, new_object, old_object, diff))

    # fix cyclic dependencies between creates & FKs -> split into two passes:
    #  1. create tables without FK columns
    #  2. patch in all the FK columns, create indexes, and constraints
    first_table_cru_ops: list[MigrationOp] = []
    patch_table_cru_ops: list[MigrationOp] = []
    for op in cru_ops:
        if op.kind != MigrationOpKind.CREATE:
            patch_table_cru_ops.append(op)
            continue

        if op.object_kind == ObjectKind.TABLE:
            assert isinstance(op.new_object, Table)
            # only columns are created implicitly in migration ops
            first_columns, deferred_columns = partition(
                lambda col: bool(col.is_foreign_key_to),
                (c.clone() for c in op.new_object.columns),
            )
            first_table = replace(op.new_object, columns=first_columns, constraints=(), indexes=())
            first_table_cru_ops.append(MigrationOp(MigrationOpKind.CREATE, first_table, None))
            for col in deferred_columns:
                col._table = first_table
                patch_table_cru_ops.append(MigrationOp(MigrationOpKind.CREATE, col, None))
        elif op.object_kind == ObjectKind.COLUMN:
            assert isinstance(op.new_object, Column)
            if op.new_object.is_foreign_key_to:
                patch_table_cru_ops.append(op)
            else:
                first_table_cru_ops.append(op)
        else:
            patch_table_cru_ops.append(op)

    ops = [*extension_ops, *deleted_table_ops, *first_table_cru_ops, *patch_table_cru_ops]
    return ops


def generate_sql_migration_code(
    migration: Migration,
    *,
    global_ops: list[MigrationOp],
    local_ops: list[MigrationOp],
    exclude_inverse: bool = False,
) -> str:
    """Generates the Python migration file."""
    migration_code = Path(MIGRATIONS_TEMPLATE_PATH).read_text()

    # impute header/metadata
    today = datetime.now(LOCAL_TZ).date().strftime("%Y.%m.%d")
    metadata_substitutions: dict[str, str] = {
        "# <Header>": f"# This migration was automatically generated on {today}. Edit as needed.",
        '"<ID>"': str(migration.id),
        '"<VERSION>"': f'"{migration.version}"',
        '"<HAS_GLOBAL>"': "True" if migration.has_global else "False",
        '"<HAS_LOCAL>"': "True" if migration.has_local else "False",
    }
    for key, value in metadata_substitutions.items():
        migration_code = migration_code.replace(key, value)

    # impute upgrade/downgrade functions
    global_ops_inverse = [op.invert() for op in global_ops[::-1]] if not exclude_inverse else None
    local_ops_inverse = [op.invert() for op in local_ops[::-1]] if not exclude_inverse else None
    for method_name, ops in [
        ("upgrade_global", global_ops),
        ("downgrade_global", global_ops_inverse),
        ("upgrade_local", local_ops),
        ("downgrade_local", local_ops_inverse),
    ]:
        method_body = _render_migration_body(ops)
        method_placeholder = f"    pass  # <{method_name}>"
        assert method_placeholder in migration_code, f"method placeholder not found: {method_name}"
        migration_code = migration_code.replace(method_placeholder, indent(method_body, "    "))

    migration_code = format_python(migration_code)
    return migration_code


def _render_migration_body(ops: list[MigrationOp] | None) -> str:
    """Renders migration operations into an executable method body."""

    if ops is None:
        return "raise NotImplementedError"

    lines: list[str] = []
    current_table: Optional[Table] = None
    current_table_stmts: list[str] = []

    def _emit_alter(table: Table, statements: list[str]) -> None:
        alter_content = ",\n    ".join(statements)
        lines.append(f'"""\n    ALTER TABLE {table.name}    \n    {alter_content}\n"""')

    def _emit(table: Table, statements: list[str]) -> None:
        # batch successive ALTER TABLE statements, otherwise leave them as-is
        lines.append(f"\n# {table.name}")
        current_alter_statements: list[str] = []
        for stmt in statements:
            if stmt.startswith(f"'ALTER TABLE {table.name}"):
                current_alter_statements.append(
                    stmt[1:-1].replace(f"ALTER TABLE {table.name} ", "")
                )
            elif stmt.startswith(f'"""\nALTER TABLE {table.name}'):
                current_alter_statements.append(
                    stmt[4:-4].replace(f"ALTER TABLE {table.name} ", "")
                )
            else:
                if current_alter_statements:
                    _emit_alter(table, current_alter_statements)
                    current_alter_statements = []
                lines.append(stmt)

        if current_alter_statements:
            _emit_alter(table, current_alter_statements)

    def _wrap_statement(stmt: str) -> str:
        stmt = str(stmt).strip()
        if "\n" in stmt:
            stmt = f'"""\n{stmt}\n"""'
        else:
            if "'" in stmt and "\\'" not in stmt:
                stmt = stmt.replace("'", "\\'")
            stmt = f"'{stmt}'"
        return stmt

    # render statements
    for op in ops:
        stmt = _render_migration_op(op)
        if stmt is None:
            continue
        stmt = _wrap_statement(stmt)

        # extensions are not table objects
        if op.object_kind == ObjectKind.EXTENSION:
            lines.append(stmt)
        else:
            if current_table is None or op.table.table_name != current_table.table_name:
                if current_table is not None and current_table_stmts:
                    _emit(current_table, current_table_stmts)
                current_table = op.table
                current_table_stmts = []
            current_table_stmts.append(stmt)

    if current_table and current_table_stmts:
        _emit(current_table, current_table_stmts)

    # post process lines
    wrapped_lines: list[str] = []
    for line in lines:
        if "#" in line:
            # leave comments as-is
            wrapped_lines.append(line)
            continue

        # pull out (SELECT ...) sub-queries
        subqueries = re.findall(r"\(SELECT.*?\)", line)
        for i, subquery in enumerate(subqueries):
            selected_columns = re.findall(r"SELECT (.*?) FROM", subquery)[0].split(", ")
            var_name = f"_{selected_columns[0]}_{i}"
            line = line.replace(subquery, f"{{{var_name}}}")
            subquery = _wrap_statement(subquery[1:-1])
            wrapped_lines.append(
                f"await cur.execute({subquery})\n{var_name} = (await cur.fetchone())['{selected_columns[0]}']"
            )

        # wrap with execute
        if subqueries:
            # suppress type warning because f string is not a literal
            wrapped_lines.append("# noinspection PyTypeChecker")
            line = "f" + line
        wrapped_lines.append(f"await cur.execute({line})")
    method_body = "\n".join(wrapped_lines)

    if not method_body.strip():
        return "pass"

    return method_body


async def apply_sql_migration_ops(cur: psycopg.AsyncCursor, ops: list[MigrationOp]) -> None:
    """Directly apply the given migration ops."""
    logger.info("apply_migration_ops", ops=ops)
    method_body = _render_migration_body(ops)
    method_body = format_python(method_body)

    # turn it into an async callable
    method = f"async def _apply_inline(cur):\n{indent(method_body, '    ')}"
    method_locals: dict[str, Any] = {}
    exec(method, method_locals)
    _apply_inline = method_locals["_apply_inline"]
    await _apply_inline(cur)


async def force_create_schema(cur: psycopg.AsyncCursor, schema: Schema) -> None:
    """Creates and applies the migrations to create the given objects in the database."""
    # ignore existing tables
    ops = generate_sql_migration_ops(Schema.blank(), schema)
    await apply_sql_migration_ops(cur, ops)


def add_migration_to_fs(migration: Migration, code: str, *, overwrite: bool = False):
    """Writes the Python migration file."""
    migration_path = (
        f"{MIGRATIONS_PATH}/{migration.id:04d}_{migration.version.replace('.', '_')}.py"
    )
    if not overwrite and os.path.exists(migration_path):
        raise RuntimeError(f"migration file already exists: {migration_path} (for {migration!r})")
    Path(migration_path).write_text(code)
    return migration_path


def _render_migration_op(op: MigrationOp) -> str | None:
    """
    Renders the given operation into a SQL string.
    Generally 'flat' - ops do not include nested objects - except for CREATE TABLE.
    """
    if op.kind == MigrationOpKind.CREATE:
        if isinstance(op.new_object, Extension):
            return f'CREATE EXTENSION IF NOT EXISTS "{op.new_object.name}"'
        elif isinstance(op.new_object, Table):
            table_contents = ",\n".join(f"    {col.sql()}" for col in op.new_object.columns)
            return f'CREATE TABLE "{op.new_object.name}" (\n{table_contents}\n)'
        elif isinstance(op.new_object, Column):
            return f'ALTER TABLE "{op.new_object.table.name}" ADD COLUMN {op.new_object.sql()}'
        elif isinstance(op.new_object, Index):
            if op.new_object.is_unique:
                return f"CREATE UNIQUE INDEX {op.new_object.sql()}"
            else:
                return f"CREATE INDEX {op.new_object.sql()}"
        elif isinstance(op.new_object, Constraint):
            return f'ALTER TABLE "{op.new_object.table.name}" ADD CONSTRAINT {op.new_object.sql()}'

    elif op.kind == MigrationOpKind.RENAME:
        assert op.new_object is not None, f"expected a new object: {op!r}"
        if isinstance(op.old_object, Table):
            return f'ALTER TABLE "{op.old_object.name}" RENAME TO "{op.new_object.name}"'
        elif isinstance(op.old_object, Column):
            return f'ALTER TABLE "{op.old_object.table.name}" RENAME COLUMN "{op.old_object.name}" TO "{op.new_object.name}"'
        elif isinstance(op.old_object, Index):
            return f'ALTER INDEX "{op.old_object.name}" RENAME TO "{op.new_object.name}"'
        elif isinstance(op.old_object, Constraint):
            return f'ALTER TABLE "{op.old_object.table.name}" RENAME CONSTRAINT "{op.old_object.name}" TO "{op.new_object.name}"'

    elif op.kind == MigrationOpKind.UPDATE:
        if isinstance(op.old_object, Table):
            # there are no table properties we can update (outside name, which is handled by rename)
            raise NotImplementedError(f"cannot render {op!r}")
        elif isinstance(op.old_object, Column):
            assert isinstance(op.new_object, Column), f"expected a column: {op.new_object!r}"
            updates: list[str] = []
            diff_keys = op.diff_keys or ()
            if "is_unique" in diff_keys:
                pass  # noop, already handled by generated index
            if "is_encrypted" in diff_keys:
                pass  # noop, handled in read/write logic
            if any(k in diff_keys for k in ("type", "is_array", "length")):
                # change type
                updates.append(
                    f"ALTER COLUMN {op.old_object.name}"
                    f" SET DATA TYPE {op.new_object.type_sql()}"
                )
            if "is_nullable" in diff_keys:
                # change nullability
                updates.append(
                    f"ALTER COLUMN {op.old_object.name}"
                    f" {(op.new_object.is_nullable and 'DROP') or 'SET'} NOT NULL"
                )
            if "default" in diff_keys:
                # change default
                if op.new_object.default is None:
                    updates.append(f"ALTER COLUMN {op.old_object.name}" f" DROP DEFAULT")
                else:
                    updates.append(
                        f"ALTER COLUMN {op.old_object.name} SET DEFAULT {op.new_object.default}"
                    )
            if "is_foreign_key_to" in diff_keys or "on_delete" in diff_keys:
                # drop and recreate foreign key constraint
                if op.old_object.is_foreign_key_to:
                    # selects inside DDL aren't technically allowed, so we factor them out in post-processing
                    object_name = op.old_object.qualified_name.replace(".", "_")
                    updates.append(
                        f"DROP CONSTRAINT IF EXISTS"  # may have cascaded
                        f" (SELECT constraint_name FROM information_schema.table_constraints WHERE table_schema = 'public' AND table_name = '{op.old_object.table.name}' AND constraint_type = 'FOREIGN KEY' AND constraint_name LIKE '{object_name}%')"
                    )
                if op.new_object.is_foreign_key_to:
                    assert op.new_object.on_delete is not None, f"no on_delete: {op.new_object!r}"
                    constraint_name = f"{op.new_object.qualified_name.replace('.', '_')}_fk_{op.new_object.is_foreign_key_to}_id"
                    updates.append(
                        f'ADD CONSTRAINT "{constraint_name}"'
                        f' FOREIGN KEY ("{op.new_object.name}")'
                        f" REFERENCES {op.new_object.is_foreign_key_to}(id)"
                        f" ON DELETE {op.new_object.on_delete.value}"
                    )
            if "is_primary_key" in diff_keys:
                if op.old_object.is_primary_key:  # drop it
                    object_name = op.old_object.qualified_name.replace(".", "_")
                    updates.append(
                        f"DROP CONSTRAINT IF EXISTS"  # may have cascaded
                        f" (SELECT constraint_name FROM information_schema.table_constraints WHERE table_schema = 'public' AND table_name = '{op.old_object.table.name}' AND constraint_type = 'PRIMARY KEY' AND constraint_name LIKE '{object_name}%')"
                    )
                else:  # create it
                    updates.append(
                        f"ADD CONSTRAINT {op.new_object.table.name}_pkey"
                        f" PRIMARY KEY ({op.new_object.name})"
                    )
            if not updates:
                return None
            return f"ALTER TABLE {op.old_object.table.name} " + ",\n".join(updates)
        elif isinstance(op.old_object, Index):
            # drop and recreate
            assert isinstance(op.new_object, Index), f"expected an index: {op.new_object!r}"
            drop = f'DROP INDEX "{op.old_object.name}"'
            if op.new_object.is_unique:
                create = f"CREATE UNIQUE INDEX {op.new_object.sql()}"
            else:
                create = f"CREATE INDEX {op.new_object.sql()}"
            return "\n".join([drop, create])
        elif isinstance(op.old_object, Constraint):
            # drop and recreate
            assert op.new_object is not None, f"expected a new object: {op!r}"
            return (
                f'ALTER TABLE "{op.old_object.table.name}"'
                f' DROP CONSTRAINT IF EXISTS "{op.old_object.name}",'  # may have cascaded
                f" ADD CONSTRAINT {op.new_object.sql()}"
            )

    elif op.kind == MigrationOpKind.DELETE:
        if isinstance(op.old_object, Extension):
            return f'DROP EXTENSION IF EXISTS "{op.old_object.name}"'
        elif isinstance(op.old_object, Table):
            return f'DROP TABLE "{op.old_object.name}"'
        elif isinstance(op.old_object, Column):
            return f'ALTER TABLE "{op.old_object.table.name}" DROP COLUMN "{op.old_object.name}"'
        elif isinstance(op.old_object, Index):
            return f'DROP INDEX "{op.old_object.name}"'
        elif isinstance(op.old_object, Constraint):
            return (
                f'ALTER TABLE "{op.old_object.table.name}" DROP CONSTRAINT "{op.old_object.name}"'
            )

    raise RuntimeError(f"unexpected migration op: {op!r}")


#
# Introspection
#


async def introspect_sql_schema(
    cur: psycopg.AsyncCursor,
    *,
    include_columns: bool = True,
    include_constraints: bool = True,
    include_indexes: bool = True,
    table_prefix: str = "bench_",
) -> Schema:
    start = asyncio.get_running_loop().time()

    # extensions
    extensions_query = """
    SELECT
        extname
    FROM
        pg_extension
    """
    extensions_rows = await pg_select_raw(cur, extensions_query)
    extensions = tuple(Extension(name=row["extname"]) for row in extensions_rows)

    def _strip_condition(condition: str) -> str:
        # remove outermost (...) if present until only one (...) remains
        while condition.startswith("((") and condition.endswith("))"):
            condition = condition[1:-1]
        return condition

    # tables
    tables_query = """
    SELECT
        table_name
    FROM
        information_schema.tables
    WHERE
        table_schema = 'public'
        AND table_name LIKE {};
    """
    tables_query = sqlstr(tables_query).format(sql.Literal(table_prefix + "%"))
    tables_rows = await pg_select_raw(cur, tables_query)
    tables_names: list[str] = [str(row["table_name"]) for row in tables_rows]

    # columns
    if include_columns:
        # TODO :Performance: improve introspect tables performance
        #  (maybe the big joins in this query are the bottleneck)
        columns_query = """
SELECT 
   col.table_name, 
   col.column_name, 
   col.data_type, 
   col.udt_name,
   col.is_nullable, 
   col.column_default,
   string_agg(tc.constraint_type, ',') AS constraint_types,
   string_agg(tc.constraint_name, ',') AS constraint_names,
   string_agg(ccu.table_name, ',') AS target_table_names,
   string_agg(rc.delete_rule, ',') AS delete_rules
FROM 
   information_schema.columns col
LEFT JOIN 
   information_schema.key_column_usage kcu 
   ON col.column_name = kcu.column_name AND col.table_name = kcu.table_name
LEFT JOIN 
   information_schema.table_constraints tc 
   ON kcu.constraint_name = tc.constraint_name AND kcu.table_schema = tc.table_schema AND kcu.table_name = tc.table_name
LEFT JOIN 
   information_schema.constraint_column_usage ccu 
   ON ccu.constraint_name = tc.constraint_name AND ccu.table_schema = tc.table_schema
LEFT JOIN 
   information_schema.referential_constraints rc 
   ON tc.constraint_name = rc.constraint_name AND tc.table_schema = rc.constraint_schema
WHERE 
   col.table_schema = 'public' AND col.table_name = ANY({})
-- deduplicate rows (may have multiple constraints)
GROUP BY 
    col.table_name, col.column_name, col.data_type, col.udt_name, col.is_nullable, col.column_default;
               """
        columns_query = sql.SQL(columns_query).format(sql.Literal(tables_names))
        columns_rows = await pg_select_raw(cur, columns_query)
        columns_by_table: dict[str, list[Column]] = defaultdict(list)
        for row in columns_rows:
            udt_name = row["udt_name"]
            if udt_name.startswith("_"):
                udt_name = udt_name[1:]
                is_array = True
            else:
                is_array = False
            postgres_type = POSTGRES_TYPE_BY_UDT[udt_name]
            if postgres_type == PostgresColumnType.TEXT:
                postgres_type = PostgresColumnType.CHARACTER_VARYING  # we don't do TEXT
            primitive_type = PRIMITIVE_TYPE_BY_POSTGRES_TYPE[postgres_type]
            is_foreign_key_to = (
                row["target_table_names"]
                if "FOREIGN KEY" in (row["constraint_types"] or "")
                else None
            )
            cascade_action = (
                CascadeAction(row["delete_rules"].split(",")[0])
                if row.get("delete_rules")
                else None
            )

            # exclude constraints that apply to more than this column (they're handled separately)
            target_table_names = (
                row["target_table_names"].split(",") if row["target_table_names"] else ()
            )
            constraint_names = row["constraint_names"].split(",") if row["constraint_names"] else ()
            constraint_types = row["constraint_types"].split(",") if row["constraint_types"] else ()
            if constraint_names:
                scalar_constraint_types = []
                for target_table_name, constraint_name, constraint_type in zip(
                    target_table_names, constraint_names, constraint_types
                ):
                    if f"{constraint_name},{constraint_name}" not in row["constraint_names"]:
                        scalar_constraint_types.append(constraint_type)
                        if constraint_type == "FOREIGN KEY":
                            is_foreign_key_to = target_table_name
            else:
                is_foreign_key_to = None
                scalar_constraint_types = ()

            column = Column(
                name=row["column_name"],
                type=primitive_type,
                is_primary_key="PRIMARY KEY" in constraint_types,
                is_foreign_key_to=is_foreign_key_to,
                on_delete=cascade_action,
                is_unique="UNIQUE" in scalar_constraint_types,
                is_nullable=row["is_nullable"] == "YES",
                is_array=is_array,
                default=row["column_default"],
            )
            columns_by_table[row["table_name"]].append(column)
    else:
        columns_by_table = {}

    # constraints
    if include_constraints:
        constraints_query = """
SELECT 
    tc.table_name,
    tc.constraint_name,
    tc.constraint_type,
    string_agg(kcu.column_name, ', ') AS column_names,
    chk.check_clause AS condition
FROM 
    information_schema.table_constraints AS tc
LEFT JOIN 
    information_schema.key_column_usage AS kcu 
    ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema
LEFT JOIN 
    information_schema.check_constraints AS chk 
    ON tc.constraint_name = chk.constraint_name AND tc.table_schema = chk.constraint_schema
WHERE 
    tc.table_schema = 'public' 
    AND tc.table_name = ANY({})
    AND tc.constraint_type IS NOT NULL
    AND tc.constraint_type NOT IN ('PRIMARY KEY', 'FOREIGN KEY')
    AND (chk.check_clause IS NULL OR chk.check_clause NOT LIKE '% IS NOT NULL')
GROUP BY 
    tc.table_name, tc.constraint_name, tc.constraint_type, chk.check_clause;
        """
        constraints_query = sql.SQL(constraints_query).format(sql.Literal(tables_names))
        constraints_rows: list[dict[str, str]] = await pg_select_raw(cur, constraints_query)
        constraints_by_table: dict[str, list[Constraint]] = defaultdict(list)
        for row in constraints_rows:
            columns = tuple(row["column_names"].split(", ")) if row["column_names"] else ()
            if not columns:
                columns = None
            constraint_name = row["constraint_name"][len(row["table_name"]) + 1 :]
            condition = row.get("condition")
            if condition:
                condition = _strip_condition(condition)
            constraint = Constraint(
                inner_name=constraint_name,
                type=ConstraintType(row["constraint_type"]),
                columns=columns,
                condition=condition,
            )
            constraints_by_table[row["table_name"]].append(constraint)
    else:
        constraints_by_table = {}

    # indexes
    if include_indexes:
        indexes_by_table: dict[str, list[Index]] = defaultdict(list)
        indexes_query = """\
SELECT 
    idx.tablename AS table_name,
    idx.indexname AS index_name,
    idx.indexdef AS index_definition
FROM 
    pg_indexes idx
WHERE 
    idx.schemaname = 'public' AND idx.tablename = ANY({});
            """
        indexes_query = sql.SQL(indexes_query).format(sql.Literal(tables_names))
        indexes_rows: list[dict[str, str]] = await pg_select_raw(cur, indexes_query)
        for row in indexes_rows:
            definition = row["index_definition"]
            columns_str = definition.split("(")[1].split(")")[0]
            columns = [col.strip() for col in columns_str.split(",")]
            # definition like 'CREATE INDEX index_name ON table_name USING index_type (columns) [WHERE condition]'
            index_type = re_search_or_error(r"USING (\w+)", definition).group(1)
            condition = (
                re_search_or_error(r"WHERE (.+)", definition).group(1)
                if "WHERE" in definition
                else None
            )
            if condition:
                condition = _strip_condition(condition)
            table_name = row["table_name"]
            index_name = row["index_name"][len(table_name) + 1 :]
            index = Index(
                inner_name=index_name,
                _full_name=row["index_name"],
                type=IndexType(index_type.upper()),
                columns=tuple(columns),
                is_unique="UNIQUE" in definition,
                condition=condition,
            )
            # ignore simple primary/foreign key index
            if (
                index.type == IndexType.BTREE
                and len(index.columns) == 1
                and (index.columns[0].endswith("_id") or index.columns[0] == "id")
                and (index.name.endswith("_pkey") or index.name.endswith("_fkey"))
            ):
                column = first(
                    c for c in columns_by_table[row["table_name"]] if c.name == index.columns[0]
                )
                if column.is_primary_key or column.is_foreign_key_to:
                    continue

            indexes_by_table[row["table_name"]].append(index)
    else:
        indexes_by_table = {}

    # assemble the tables
    tables: list[Table] = []
    for table_name in tables_names:
        table = Table(
            name=table_name,
            columns=tuple(columns_by_table.get(table_name, [])),
            indexes=tuple(indexes_by_table.get(table_name, [])),
            constraints=tuple(constraints_by_table.get(table_name, [])),
        )
        tables.append(table)

    duration = asyncio.get_running_loop().time() - start
    logger.debug("introspect", cur=cur, duration=duration, tables=[t.name for t in tables])

    return Schema(extensions=extensions, tables=tuple(tables))
