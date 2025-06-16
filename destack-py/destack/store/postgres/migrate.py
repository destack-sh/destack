import enum
import os
import types
from collections import defaultdict
from collections.abc import Awaitable, Collection, Mapping
from dataclasses import dataclass, replace
from datetime import datetime
from itertools import chain
from pathlib import Path
from textwrap import indent
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    Optional,
    cast,
)

import asyncpg
import regex
import structlog
from more_itertools import first
from opentelemetry import trace

from destack.language import StoreType
from destack.utils.code import format_code
from destack.utils.env import REPOSITORY_PATH
from destack.utils.func import partition, re_search_or_error
from destack.utils.oracle import Oracle

from .core import (
    POSTGRES_TYPE_BY_UDT,
    PRIMITIVE_TYPE_BY_POSTGRES_TYPE,
    PostgresCascadeAction,
    PostgresColumn,
    PostgresColumnType,
    PostgresConstraint,
    PostgresConstraintType,
    PostgresExtension,
    PostgresIndex,
    PostgresIndexType,
    PostgresObject,
    PostgresObjectKind,
    PostgresSchema,
    PostgresTable,
    PostgresTableObject,
)

if TYPE_CHECKING:
    from destack.language import Database

MIGRATIONS_PATH = REPOSITORY_PATH / "destack/migrations"
MIGRATIONS_TEMPLATE_PATH = REPOSITORY_PATH / "destack/migrations/0000_template.py"

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass
class Migration:
    id: int
    version: str
    has_global: bool
    has_spatial: bool
    applied_at: Optional[datetime]
    path: Optional[Path] = None  # not databased
    file: Optional["MigrationFile"] = None  # not databased

    def __str__(self) -> str:
        return f"{self.id} {self.version} (has_global={self.has_global}, has_spatial={self.has_spatial})"

    def __repr__(self) -> str:
        return f"<Migration {self}>"

    def has_store_type(self, store_type: StoreType) -> bool:
        return (store_type == StoreType.GLOBAL_ENTITY and self.has_global) or (
            store_type == StoreType.SPATIAL_ENTITY and self.has_spatial
        )


MigratorFunc = Callable[[asyncpg.Connection], Awaitable[None]]


@dataclass
class MigrationFile:
    path: Path
    module: Any


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
            match[0]: match[1] for match in regex.findall(r"([A-Z_]+) = (.*)", migration_code)
        }
        migration = Migration(
            id=int(migration_metadata["ID"]),
            version=migration_metadata["VERSION"][1:-1],
            has_global=migration_metadata["HAS_GLOBAL"] == "True",
            has_spatial=migration_metadata["HAS_SPATIAL"] == "True",
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
        if is_global and not migration.has_global:
            continue
        if version_a < migration.version:
            return True
    return False


def _load_migration_from_path(migration: Migration) -> MigrationFile:
    assert migration.path is not None, "migration path not set"
    migration_code = migration.path.read_text()
    migration_module = types.ModuleType(migration.path.stem)
    exec(migration_code, migration_module.__dict__)
    file = MigrationFile(path=migration.path, module=migration_module)
    return file


@tracer.start_as_current_span("postgres.migrate")
async def postgres_migrate(
    conn: asyncpg.Connection,
    target: str | int | None,
    oracle: Oracle,
    *,
    store_type: StoreType,
    database: "Database | None" = None,
) -> list[Migration]:
    """
    Applies missing migrations (up or down) to reach the target migration.
    Also updates the migrations table.
    """

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
        "migrations.load",
        target_migration=target_migration,
        store_type=store_type,
        database=database,
    )
    databased_migrations = await read_migrations_from_pg(conn)
    is_upgrade = all(target_migration.id > m.id for m in databased_migrations if m.applied_at)
    log = logger.bind(
        target_migration=target_migration,
        is_upgrade=is_upgrade,
        store_type=store_type,
        database=database,
    )
    applied_migrations = [m for m in databased_migrations if m.applied_at is not None]
    current_migration = max(applied_migrations, key=lambda m: m.id) if applied_migrations else None
    current_migration_id = current_migration.id if current_migration else -1

    # get the migrations to apply
    migrations_to_apply = []
    for migration in MIGRATIONS:
        if not migration.has_store_type(store_type):
            continue
        if (is_upgrade and current_migration_id < migration.id <= target_migration.id) or (
            not is_upgrade and current_migration_id >= migration.id > target_migration.id
        ):
            migrations_to_apply.append(migration)

    # apply the migrations
    if not migrations_to_apply:
        log.trace("migrations.apply.skip", database=database)
        return []
    else:
        await _do_migrate(
            conn,
            migrations_to_apply,
            oracle=oracle,
            is_upgrade=is_upgrade,
            store_type=store_type,
            database=database,
        )
        log.debug("migrations.apply", conn=conn, migrations=migrations_to_apply, database=database)

    # update the migration table (applied + missing)
    missing_migrations = [
        m
        for m in MIGRATIONS
        if not any(m.id == s.id for s in databased_migrations)
        and not any(m.id == a.id for a in applied_migrations)
    ]
    migrations_to_update = [*migrations_to_apply, *missing_migrations]
    if migrations_to_update:
        await upsert_migrations(conn, migrations_to_update)

    return migrations_to_apply


async def _do_migrate(
    conn: asyncpg.Connection,
    migrations: Collection[Migration],
    oracle: Oracle,
    *,
    is_upgrade: bool,
    store_type: StoreType,
    database: Optional["Database"] = None,
):
    """Applies the given migrations in the given order."""
    for migration in migrations:
        with tracer.start_as_current_span("postgres.apply_migration"):
            func_name = f"{(is_upgrade and 'upgrade') or 'downgrade'}_{(store_type.name.lower()) or 'local'}"
            migration_file = _load_migration_from_path(migration)
            func = getattr(migration_file.module, func_name)
            try:
                await func(conn)
            except Exception as e:
                logger.error(
                    "migration.apply.error",
                    conn=conn,
                    migration=migration,
                    database=database,
                    error=e,
                    span="current",
                )
                raise
            if is_upgrade:
                migration.applied_at = oracle.utc()
            else:
                migration.applied_at = None
            logger.debug(
                "migration.apply", conn=conn, migration=migration, database=database, span="current"
            )


#
# Generating migrations
#


class PostgresMigrationOpType(enum.Enum):
    CREATE = "CREATE"
    RENAME = "RENAME"
    UPDATE = "UPDATE"
    DELETE = "DELETE"


@dataclass
class PostgresMigrationOp:
    type: PostgresMigrationOpType
    new_object: Optional[PostgresObject]
    old_object: Optional[PostgresObject]
    diff_keys: Optional[tuple[str, ...]] = None

    def __str__(self) -> str:
        op_str = f"{self.type.value} {self.object_kind.name}"
        if self.type == PostgresMigrationOpType.CREATE:
            assert self.new_object is not None, f"new_object not set for {self!r}"
            return f"{op_str} {self.new_object.qualified_name}"
        elif self.type == PostgresMigrationOpType.RENAME:
            assert self.old_object is not None, f"old_object not set for {self!r}"
            assert self.new_object is not None, f"new_object not set for {self!r}"
            return f"{op_str} {self.old_object.qualified_name} -> {self.new_object.qualified_name}"
        elif self.type == PostgresMigrationOpType.UPDATE:
            assert self.diff_keys is not None, f"diff_keys not set for {self!r}"
            assert self.new_object is not None, f"new_object not set for {self!r}"
            diff_str = ", ".join(
                f"{k}:{getattr(self.old_object, k)}->{getattr(self.new_object, k)}"
                for k in self.diff_keys
            )
            return f"{op_str} {self.new_object.qualified_name} ({diff_str})"
        elif self.type == PostgresMigrationOpType.DELETE:
            assert self.old_object is not None, f"old_object not set for {self!r}"
            return f"{op_str} {self.old_object.qualified_name}"
        else:
            raise RuntimeError(f"unexpected migration op type: {self.type}")

    def __repr__(self) -> str:
        return f"<MigrationOp {self}>"

    @property
    def object_kind(self) -> PostgresObjectKind:
        if self.new_object is not None:
            return self.new_object.kind
        elif self.old_object is not None:
            return self.old_object.kind
        else:
            raise RuntimeError(f"no object set for {self!r}")

    @property
    def table(self) -> "PostgresTable":
        if self.new_object is not None:
            return cast("PostgresTableObject", self.new_object).table
        elif self.old_object is not None:
            return cast("PostgresTableObject", self.old_object).table
        else:
            raise RuntimeError(f"no object set for {self!r}")

    def invert(self) -> "PostgresMigrationOp":
        """Returns the inverse of this operation for undoing migrations."""
        if self.type == PostgresMigrationOpType.CREATE:
            return PostgresMigrationOp(PostgresMigrationOpType.DELETE, None, self.new_object)
        elif self.type == PostgresMigrationOpType.RENAME:
            return PostgresMigrationOp(
                PostgresMigrationOpType.RENAME, self.old_object, self.new_object
            )
        elif self.type == PostgresMigrationOpType.UPDATE:
            return PostgresMigrationOp(
                PostgresMigrationOpType.UPDATE, self.old_object, self.new_object, self.diff_keys
            )
        elif self.type == PostgresMigrationOpType.DELETE:
            return PostgresMigrationOp(PostgresMigrationOpType.CREATE, self.old_object, None)
        raise RuntimeError(f"unexpected migration op type: {self.type}")


#
# Diffing/generation
#


@tracer.start_as_current_span("postgres.generate_migration_code")
def generate_migration_code(
    migration: Migration,
    oracle: Oracle,
    *,
    global_ops: list[PostgresMigrationOp],
    main_ops: list[PostgresMigrationOp],
    exclude_inverse: bool = False,
) -> str:
    """Generates the Python migration file."""
    migration_code = Path(MIGRATIONS_TEMPLATE_PATH).read_text()

    # impute header/metadata
    today = oracle.utc().date().strftime("%Y.%m.%d")
    metadata_substitutions: dict[str, str] = {
        "# <Header>": f"# This migration was automatically generated on {today}. Edit as needed.",
        '"<ID>"': str(migration.id),
        '"<VERSION>"': f'"{migration.version}"',
        '"<HAS_GLOBAL>"': "True" if migration.has_global else "False",
        '"<HAS_SPATIAL>"': "True" if migration.has_spatial else "False",
    }
    for key, value in metadata_substitutions.items():
        migration_code = migration_code.replace(key, value)

    # impute upgrade/downgrade functions
    global_ops_inverse = [op.invert() for op in global_ops[::-1]] if not exclude_inverse else None
    main_ops_inverse = [op.invert() for op in main_ops[::-1]] if not exclude_inverse else None
    for method_name, ops in [
        ("upgrade_global", global_ops),
        ("downgrade_global", global_ops_inverse),
        ("upgrade_main", main_ops),
        ("downgrade_main", main_ops_inverse),
    ]:
        method_body = _render_migration_body(ops)
        method_placeholder = f"    pass  # <{method_name}>"
        assert method_placeholder in migration_code, f"method placeholder not found: {method_name}"
        migration_code = migration_code.replace(method_placeholder, indent(method_body, "    "))

    migration_code = format_code(migration_code)
    return migration_code


@tracer.start_as_current_span("postgres.generate_migration_ops")
def generate_migration_ops(
    *,
    old_schema: PostgresSchema,
    new_schema: PostgresSchema,
    include_types: tuple[PostgresMigrationOpType, ...] = tuple(PostgresMigrationOpType),
) -> list[PostgresMigrationOp]:
    """Generates the migration operations to go from the old tables to the new tables."""

    # extensions
    extension_ops: list[PostgresMigrationOp] = []
    old_extensions = {ext.name for ext in old_schema.extensions}
    new_extensions = {ext.name for ext in new_schema.extensions}
    for ext_name in new_extensions - old_extensions:
        extension_ops.append(
            PostgresMigrationOp(PostgresMigrationOpType.CREATE, PostgresExtension(ext_name), None)
        )
    # NOTE: we don't remove extensions for now

    def _to_id(obj: PostgresTableObject) -> str:
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
    deleted_table_ops: list[PostgresMigrationOp] = []
    for old_object in old_table_objects:
        old_id = _to_id(old_object)
        new_object = new_table_objects_by_id.get(old_id)
        if new_object is None:
            if _to_id(old_object.table) in deleted_ids:
                continue  # skip, table deleted
            if old_object.kind == PostgresObjectKind.INDEX:
                # skip if owning constraint is also deleted
                if any(
                    obj.kind == PostgresObjectKind.CONSTRAINT
                    and obj.name == old_object.qualified_name
                    for obj in deleted_table_objects
                ):
                    continue
            deleted_table_ops.append(
                PostgresMigrationOp(PostgresMigrationOpType.DELETE, None, old_object)
            )
            deleted_ids.add(old_id)
    # regular order: table -> column -> index -> constraint
    cru_ops: list[PostgresMigrationOp] = []
    for new_object in new_table_objects:
        new_id = _to_id(new_object)
        old_object = old_table_objects_by_id.get(new_id)
        if old_object is None:
            # create columns only if parent table isn't new
            if (
                new_object.kind == PostgresObjectKind.COLUMN
                and _to_id(new_object.table) in new_table_objects_by_id
                and _to_id(new_object.table) not in old_table_objects_by_id
            ):
                continue
            cru_ops.append(PostgresMigrationOp(PostgresMigrationOpType.CREATE, new_object, None))
        elif new_object.name != old_object.name:
            cru_ops.append(
                PostgresMigrationOp(PostgresMigrationOpType.RENAME, new_object, old_object)
            )
        elif old_object.hash_flat() != new_object.hash_flat():
            diff = new_object.diff_keys(old_object)
            if not diff:
                continue  # hashing changed
            cru_ops.append(
                PostgresMigrationOp(PostgresMigrationOpType.UPDATE, new_object, old_object, diff)
            )

    # fix cyclic dependencies between creates & FKs -> split into two passes:
    #  1. create tables without FK columns
    #  2. patch in all the FK columns, create indexes, and constraints
    first_table_cru_ops: list[PostgresMigrationOp] = []
    patch_table_cru_ops: list[PostgresMigrationOp] = []
    for op in cru_ops:
        if op.type != PostgresMigrationOpType.CREATE:
            patch_table_cru_ops.append(op)
            continue

        if op.object_kind == PostgresObjectKind.TABLE:
            assert isinstance(op.new_object, PostgresTable)
            # only columns are created implicitly in migration ops
            first_columns, deferred_columns = partition(
                lambda col: bool(col.is_foreign_key_to),
                (c.clone() for c in op.new_object.columns),
            )
            first_table = replace(op.new_object, columns=first_columns, constraints=(), indexes=())
            first_table_cru_ops.append(
                PostgresMigrationOp(PostgresMigrationOpType.CREATE, first_table, None)
            )
            for col in deferred_columns:
                col._table = first_table
                patch_table_cru_ops.append(
                    PostgresMigrationOp(PostgresMigrationOpType.CREATE, col, None)
                )
        elif op.object_kind == PostgresObjectKind.COLUMN:
            assert isinstance(op.new_object, PostgresColumn)
            if op.new_object.is_foreign_key_to:
                patch_table_cru_ops.append(op)
            else:
                first_table_cru_ops.append(op)
        else:
            patch_table_cru_ops.append(op)

    # combine and filter ops
    ops: list[PostgresMigrationOp] = []
    for op in chain(extension_ops, deleted_table_ops, first_table_cru_ops, patch_table_cru_ops):
        if op.type in include_types:
            ops.append(op)
    return ops


def _render_migration_body(ops: list[PostgresMigrationOp] | None) -> str:
    """Renders migration operations into an executable method body."""

    if ops is None:
        return "raise NotImplementedError"

    lines: list[str] = []
    current_table: Optional[PostgresTable] = None
    current_table_stmts: list[str] = []

    def _emit_alter(table: PostgresTable, statements: list[str]) -> None:
        alter_content = ",\n    ".join(statements)
        lines.append(f'"""\n    ALTER TABLE "{table.name}"    \n    {alter_content}\n"""')

    def _emit(table: PostgresTable, statements: list[str]) -> None:
        # batch successive ALTER TABLE statements, otherwise leave them as-is
        lines.append(f"\n# {table.name}")
        current_alter_statements: list[str] = []
        for stmt in statements:
            if stmt.startswith(f'\'ALTER TABLE "{table.name}"'):
                current_alter_statements.append(
                    stmt[1:-1].replace(f'ALTER TABLE "{table.name}" ', "")
                )
            elif stmt.startswith(f'"""\nALTER TABLE "{table.name}"'):
                current_alter_statements.append(
                    stmt[4:-4].replace(f'ALTER TABLE "{table.name}" ', "")
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
        if op.object_kind == PostgresObjectKind.EXTENSION:
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
        subqueries = regex.findall(r"\(SELECT.*?\)", line)
        for i, subquery in enumerate(subqueries):
            selected_columns = regex.findall(r"SELECT (.*?) FROM", subquery)[0].split(", ")
            var_name = f"_{selected_columns[0]}_{i}"
            line = line.replace(subquery, f"{{{var_name}}}")
            subquery = _wrap_statement(subquery[1:-1])
            wrapped_lines.append(
                f"await conn.execute({subquery})\n{var_name} = (await conn.fetchrow({subquery}))['{selected_columns[0]}']"
            )

        # wrap with execute
        if subqueries:
            # suppress type warning because f string is not a literal
            wrapped_lines.append("# noinspection PyTypeChecker")
            line = "f" + line
        wrapped_lines.append(f"await conn.execute({line})")
    method_body = "\n".join(wrapped_lines)

    if not method_body.strip():
        return "pass"

    return method_body


@tracer.start_as_current_span("postgres.apply_migration_ops")
async def apply_migration_ops(conn: asyncpg.Connection, ops: list[PostgresMigrationOp]) -> None:
    """Directly apply the given migration ops."""
    method_body = _render_migration_body(ops)
    method_body = format_code(method_body)

    # turn it into an async callable
    method = f"async def _apply_inline(conn):\n{indent(method_body, '    ')}"
    method_locals: dict[str, Any] = {}
    exec(method, method_locals)
    apply_inline = method_locals["_apply_inline"]
    try:
        await apply_inline(conn)
        logger.debug("postgres.apply_migration_ops", ops=ops, conn=conn, span="current")
    except Exception as e:
        logger.error(
            "postgres.apply_migration_ops.error", ops=ops, conn=conn, span="current", error=e
        )
        raise


def add_migration_to_fs(migration: Migration, code: str, *, overwrite: bool = False):
    """Writes the Python migration file."""
    migration_path = (
        f"{MIGRATIONS_PATH}/{migration.id:04d}_{migration.version.replace('.', '_')}.py"
    )
    if not overwrite and os.path.exists(migration_path):
        raise RuntimeError(f"migration file already exists: {migration_path} (for {migration!r})")
    Path(migration_path).write_text(code)
    return migration_path


def _render_migration_op(op: PostgresMigrationOp) -> str | None:
    """
    Renders the given operation into a SQL string.
    Generally 'flat' - ops do not include nested objects - except for CREATE TABLE.
    """
    if op.type == PostgresMigrationOpType.CREATE:
        if isinstance(op.new_object, PostgresExtension):
            return f'CREATE EXTENSION IF NOT EXISTS "{op.new_object.name}"'
        elif isinstance(op.new_object, PostgresTable):
            table_contents = ",\n".join(f"    {col.sql()}" for col in op.new_object.columns)
            return f'CREATE TABLE "{op.new_object.name}" (\n{table_contents}\n)'
        elif isinstance(op.new_object, PostgresColumn):
            return f'ALTER TABLE "{op.new_object.table.name}" ADD COLUMN {op.new_object.sql()}'
        elif isinstance(op.new_object, PostgresIndex):
            if op.new_object.is_unique:
                return f"CREATE UNIQUE INDEX {op.new_object.sql()}"
            else:
                return f"CREATE INDEX {op.new_object.sql()}"
        elif isinstance(op.new_object, PostgresConstraint):
            return f'ALTER TABLE "{op.new_object.table.name}" ADD CONSTRAINT {op.new_object.sql()}'

    elif op.type == PostgresMigrationOpType.RENAME:
        assert op.new_object is not None, f"expected a new object: {op!r}"
        if isinstance(op.old_object, PostgresTable):
            return f'ALTER TABLE "{op.old_object.name}" RENAME TO "{op.new_object.name}"'
        elif isinstance(op.old_object, PostgresColumn):
            return f'ALTER TABLE "{op.old_object.table.name}" RENAME COLUMN "{op.old_object.name}" TO "{op.new_object.name}"'
        elif isinstance(op.old_object, PostgresIndex):
            return f'ALTER INDEX "{op.old_object.name}" RENAME TO "{op.new_object.name}"'
        elif isinstance(op.old_object, PostgresConstraint):
            return f'ALTER TABLE "{op.old_object.table.name}" RENAME CONSTRAINT "{op.old_object.name}" TO "{op.new_object.name}"'

    elif op.type == PostgresMigrationOpType.UPDATE:
        if isinstance(op.old_object, PostgresTable):
            # there are no table properties we can update (outside name, which is handled by rename)
            raise NotImplementedError(f"cannot render {op!r}")
        elif isinstance(op.old_object, PostgresColumn):
            assert isinstance(op.new_object, PostgresColumn), (
                f"expected a column: {op.new_object!r}"
            )
            updates: list[str] = []
            diff_keys = op.diff_keys or ()
            if "is_unique" in diff_keys:
                pass  # noop, already handled by generated index
            if any(k in diff_keys for k in ("type", "is_array", "length")):
                # change type
                updates.append(
                    f'ALTER COLUMN "{op.old_object.name}" SET DATA TYPE {op.new_object.type_sql()}'
                )
            if "is_nullable" in diff_keys:
                # change nullability
                updates.append(
                    f'ALTER COLUMN "{op.old_object.name}"'
                    f" {(op.new_object.is_nullable and 'DROP') or 'SET'} NOT NULL"
                )
            if "default" in diff_keys:
                # change default
                if op.new_object.default is None:
                    updates.append(f'ALTER COLUMN "{op.old_object.name}" DROP DEFAULT')
                else:
                    updates.append(
                        f'ALTER COLUMN "{op.old_object.name}" SET DEFAULT {op.new_object.default}'
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
                        f' PRIMARY KEY ("{op.new_object.name}")'
                    )
            if not updates:
                return None
            return f'ALTER TABLE "{op.old_object.table.name}" ' + ",\n".join(updates)
        elif isinstance(op.old_object, PostgresIndex):
            # drop and recreate
            assert isinstance(op.new_object, PostgresIndex), f"expected an index: {op.new_object!r}"
            drop = f'DROP INDEX "{op.old_object.name}"'
            if op.new_object.is_unique:
                create = f"CREATE UNIQUE INDEX {op.new_object.sql()}"
            else:
                create = f"CREATE INDEX {op.new_object.sql()}"
            return "\n".join([drop, create])
        elif isinstance(op.old_object, PostgresConstraint):
            # drop and recreate
            assert op.new_object is not None, f"expected a new object: {op!r}"
            return (
                f'ALTER TABLE "{op.old_object.table.name}"'
                f' DROP CONSTRAINT IF EXISTS "{op.old_object.name}",'  # may have cascaded
                f" ADD CONSTRAINT {op.new_object.sql()}"
            )

    elif op.type == PostgresMigrationOpType.DELETE:
        if isinstance(op.old_object, PostgresExtension):
            return f'DROP EXTENSION IF EXISTS "{op.old_object.name}"'
        elif isinstance(op.old_object, PostgresTable):
            return f'DROP TABLE "{op.old_object.name}"'
        elif isinstance(op.old_object, PostgresColumn):
            return f'ALTER TABLE "{op.old_object.table.name}" DROP COLUMN "{op.old_object.name}"'
        elif isinstance(op.old_object, PostgresIndex):
            return f'DROP INDEX "{op.old_object.name}"'
        elif isinstance(op.old_object, PostgresConstraint):
            return (
                f'ALTER TABLE "{op.old_object.table.name}" DROP CONSTRAINT "{op.old_object.name}"'
            )

    raise RuntimeError(f"unexpected migration op: {op!r}")


def pack_migration_row(migration: Migration) -> dict[str, Any]:
    return {
        "id": migration.id,
        "version": migration.version,
        "has_global": migration.has_global,
        "has_spatial": migration.has_spatial,
        "applied_at": migration.applied_at,
    }


def unpack_migration_row(row: Mapping[str, Any]) -> Migration:
    return Migration(
        id=row["id"],
        version=row["version"],
        has_global=row["has_global"],
        has_spatial=row["has_spatial"],
        applied_at=row["applied_at"],
    )


async def read_migrations_from_pg(
    conn: asyncpg.Connection, *, applied: bool | None = None
) -> list[Migration]:
    """Reads the 'destack_migration' table (if it exists) and returns the corresponding Migration."""
    query = "SELECT id, version, has_global, has_spatial, applied_at FROM destack_migration"
    if applied is not None:
        where_clause = "applied_at IS NOT NULL" if applied else "applied_at IS NULL"
        query += f" WHERE {where_clause}"
    query += " ORDER BY id"

    rows = await conn.fetch(query)
    migrations = [unpack_migration_row(dict(row)) for row in rows]
    return migrations


async def upsert_migrations(conn: asyncpg.Connection, migrations: list[Migration]):
    """Upserts the given migrations into the table. Errors if the table doesn't exist."""
    if not migrations:
        return

    for migration in migrations:
        await conn.execute(
            """
            INSERT INTO destack_migration (id, version, has_global, has_spatial, applied_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE SET
                version = EXCLUDED.version,
                has_global = EXCLUDED.has_global,
                has_spatial = EXCLUDED.has_spatial,
                applied_at = EXCLUDED.applied_at
            """,
            migration.id,
            migration.version,
            migration.has_global,
            migration.has_spatial,
            migration.applied_at,
        )


async def delete_migrations(conn: asyncpg.Connection, from_id: int, to_id: int) -> None:
    """Deletes migrations from the database."""
    # First read the migrations that will be deleted for logging
    rows = await conn.fetch(
        "SELECT id, version, has_global, has_spatial, applied_at FROM destack_migration WHERE id >= $1 AND id <= $2",
        from_id,
        to_id,
    )
    migrations = [unpack_migration_row(dict(row)) for row in rows]

    # Delete the migrations
    await conn.execute(
        "DELETE FROM destack_migration WHERE id >= $1 AND id <= $2",
        from_id,
        to_id,
    )

    if migrations:
        logger.warning("migration.delete", migrations=migrations)


#
# Introspection
#


@tracer.start_as_current_span("postgres.introspect_schema")
async def introspect_schema(
    conn: asyncpg.Connection,
    *,
    include_columns: bool = True,
    include_constraints: bool = True,
    include_indexes: bool = True,
    include_extensions: bool = True,
    include_table_prefixes: tuple[str, ...],
    exclude_table_prefixes: tuple[str, ...],
) -> PostgresSchema:
    # extensions
    extensions_query = """
    SELECT
        extname
    FROM
        pg_extension
    """
    if include_extensions:
        extensions_rows = await conn.fetch(extensions_query)
        extensions = tuple(PostgresExtension(name=row["extname"]) for row in extensions_rows)
    else:
        extensions = ()

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
    """
    if include_table_prefixes:
        include_patterns = ", ".join(f"'{prefix}%'" for prefix in include_table_prefixes)
        tables_query += f" AND table_name LIKE ANY (ARRAY[{include_patterns}])"
    if exclude_table_prefixes:
        exclude_patterns = ", ".join(f"'{prefix}%'" for prefix in exclude_table_prefixes)
        tables_query += f" AND table_name NOT LIKE ANY (ARRAY[{exclude_patterns}])"

    tables_rows = await conn.fetch(tables_query)
    tables_names: list[str] = [str(row["table_name"]) for row in tables_rows]

    # columns
    if include_columns:
        # NOTE :Performance: improve introspect tables performance
        #  (maybe the big joins in this query are the bottleneck?)
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
   col.table_schema = 'public' AND col.table_name = ANY($1)
-- deduplicate rows (may have multiple constraints)
GROUP BY 
    col.table_name, col.column_name, col.data_type, col.udt_name, col.is_nullable, col.column_default;
               """
        columns_rows: list[asyncpg.Record] = await conn.fetch(columns_query, tables_names)
        columns_by_table: dict[str, list[PostgresColumn]] = defaultdict(list)
        for row in columns_rows:
            udt_name: str = row["udt_name"]
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
                PostgresCascadeAction(row["delete_rules"].split(",")[0])  # type: ignore
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

            column = PostgresColumn(
                name=row["column_name"],
                type=primitive_type,
                prop=None,  # type: ignore # nocheckin: handle postgres reflection node_type/prop
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
    AND tc.table_name = ANY($1)
    AND tc.constraint_type IS NOT NULL
    AND tc.constraint_type NOT IN ('PRIMARY KEY', 'FOREIGN KEY')
    AND (chk.check_clause IS NULL OR chk.check_clause NOT LIKE '% IS NOT NULL')
GROUP BY 
    tc.table_name, tc.constraint_name, tc.constraint_type, chk.check_clause;
        """
        constraints_rows = await conn.fetch(constraints_query, tables_names)
        constraints_by_table: dict[str, list[PostgresConstraint]] = defaultdict(list)
        for row in constraints_rows:
            columns = tuple(row["column_names"].split(", ")) if row["column_names"] else ()
            if not columns:
                columns = None
            constraint_name: str = row["constraint_name"][len(row["table_name"]) + 1 :]  # type: ignore
            condition = row.get("condition")
            if condition:
                condition = _strip_condition(condition)
            constraint = PostgresConstraint(
                inner_name=constraint_name,
                type=PostgresConstraintType(row["constraint_type"]),
                columns=columns,
                condition=condition,
            )
            constraints_by_table[row["table_name"]].append(constraint)
    else:
        constraints_by_table = {}

    # indexes
    if include_indexes:
        indexes_by_table: dict[str, list[PostgresIndex]] = defaultdict(list)
        indexes_query = """\
SELECT 
    idx.tablename AS table_name,
    idx.indexname AS index_name,
    idx.indexdef AS index_definition
FROM 
    pg_indexes idx
WHERE 
    idx.schemaname = 'public' AND idx.tablename = ANY($1);
            """
        indexes_rows: list[asyncpg.Record] = await conn.fetch(indexes_query, tables_names)
        for row in indexes_rows:
            definition: str = row["index_definition"]
            columns_str = definition.split("(")[1].split(")")[0]
            columns = [col.strip() for col in columns_str.split(",")]
            # definition like 'CREATE INDEX index_name ON table_name USING index_type (columns) [INCLUDE (cover)] [WHERE condition]'
            index_type = re_search_or_error(r"USING (\w+)", definition).group(1)
            condition = (
                re_search_or_error(r"WHERE (.+)", definition).group(1)
                if "WHERE" in definition
                else None
            )
            if condition:
                condition = _strip_condition(condition)
            cover = (
                tuple(
                    col.strip()
                    for col in re_search_or_error(r"INCLUDE \((.*?)\)", definition)
                    .group(1)
                    .split(",")
                )
                if "INCLUDE" in definition
                else ()
            )
            table_name: str = row["table_name"]
            index_name: str = row["index_name"][len(table_name) + 1 :]  # type: ignore
            index = PostgresIndex(
                inner_name=index_name,
                _full_name=row["index_name"],
                type=PostgresIndexType(index_type.upper()),
                columns=tuple(columns),
                cover=cover,
                is_unique="UNIQUE" in definition,
                condition=condition,
            )
            # ignore simple primary/foreign key index
            if (
                index.type == PostgresIndexType.BTREE
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
    tables: list[PostgresTable] = []
    for table_name in tables_names:
        table = PostgresTable(
            name=table_name,
            node_type=None,  # type: ignore (see above)
            columns=tuple(columns_by_table.get(table_name, [])),
            indexes=tuple(indexes_by_table.get(table_name, [])),
            constraints=tuple(constraints_by_table.get(table_name, [])),
        )
        tables.append(table)

    logger.debug("postgres.introspect", conn=conn, tables=tables, span="current")

    return PostgresSchema(extensions=extensions, tables=tuple(tables))
