import asyncio
import enum
import importlib
import os
import re
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from textwrap import indent
from typing import Any, Awaitable, Callable, Collection, Optional

import psycopg
import structlog
from more_itertools import first
from psycopg import sql

from bench.sql.core import (
    COLUMN_TYPE_BY_POSTGRES_TYPE,
    MIGRATION_TABLE,
    POSTGRES_TYPE_BY_UDT,
    CascadeAction,
    Column,
    Constraint,
    ConstraintType,
    Index,
    IndexType,
    ObjectKind,
    PostgresColumnType,
    Table,
    TableObject,
)
from bench.sql.engine import SqlUndefinedObject, pg_select, pg_select_raw, pg_upsert
from bench.utils.utils import format_python

MIGRATIONS_PATH = "bench/sql/migrations"
MIGRATIONS_TEMPLATE_PATH = "bench/sql/migrations/0000_template.py"

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
    path: Optional[str] = None  # not stored
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
    path: str
    module: Any


def pack_migration_row(migration: Migration) -> dict[str, Any]:
    return {
        "id": migration.id,
        "version": migration.version,
        "has_global": migration.has_global,
        "has_local": migration.has_local,
        "applied_at": migration.applied_at,
    }


def unpack_migration_row(row: dict[str, Any]) -> Migration:
    return Migration(
        id=row["id"],
        version=row["version"],
        has_global=row["has_global"],
        has_local=row["has_local"],
        applied_at=row["applied_at"],
    )


async def read_migrations_from_pg(
    cur: psycopg.AsyncCursor, *, applied: bool = None
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
    except SqlUndefinedObject:
        await cur.connection.rollback()
        return []


async def _write_migrations_to_pg(cur: psycopg.AsyncCursor, migrations: list[Migration]):
    """Upserts the given migrations into the table. Errors if the table doesn't exist."""
    migrations_rows = [pack_migration_row(m) for m in migrations]
    await pg_upsert(cur, MIGRATION_TABLE, migrations_rows)


def read_migrations_from_fs() -> list[Migration]:
    """Reads the available migrations from local filesystem. Actually loads each migration file."""
    migrations: list[Migration] = []
    for migration_file in sorted(os.listdir(MIGRATIONS_PATH)):
        if migration_file in ("0000_template.py", "__init__.py") or not migration_file.endswith(
            ".py"
        ):
            continue

        # parse the file
        migration_path = MIGRATIONS_PATH + "/" + migration_file
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
    return migrations


def _load_migration_from_path(migration: Migration) -> MigrationFile:
    assert migration.path is not None, "migration path not set"
    module_path = migration.path.replace("/", ".").replace(".py", "")
    migration_module = importlib.import_module(module_path)
    file = MigrationFile(path=migration.path, module=migration_module)
    return file


async def migrate_to(
    cur: psycopg.AsyncCursor,
    target: str | int,
    *,
    is_global: bool,
) -> list[Migration]:
    """
    Applies missing migrations (up or down) to reach the target migration.
    Also updates the migrations table.
    """

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

    logger.debug("migration.load", target_migration=target_migration, is_global=is_global)
    stored_migrations = await read_migrations_from_pg(cur)
    is_upgrade = all(m.id > target_migration.id for m in stored_migrations if m.applied_at)
    log = logger.bind(target_migration=target_migration, is_upgrade=is_upgrade, is_global=is_global)
    logger.info("migration.apply_missing")
    applied_migrations = [m for m in stored_migrations if m.applied_at is not None]
    current_migration = max(applied_migrations, key=lambda m: m.id) if applied_migrations else None
    current_migration_id = current_migration.id if current_migration else -1

    # get the migrations to apply
    migrations_to_apply = []
    for migration in all_migrations:
        if is_global and not migration.has_global or not is_global and not migration.has_local:
            continue
        if (is_upgrade and current_migration_id < migration.id <= target_migration.id) or (
            not is_upgrade and current_migration_id >= migration.id > target_migration.id
        ):
            migrations_to_apply.append(migration)

    # apply the migrations
    if not migrations_to_apply:
        log.info("migration.apply_missing.noop")
        return []
    else:
        log.info("migration.apply_missing.start", migrations_to_apply=migrations_to_apply)
        await _do_migrate(cur, migrations_to_apply, is_upgrade=is_upgrade, is_global=is_global)

    # update the migration table (applied + missing)
    missing_migrations = [
        m
        for m in all_migrations
        if not any(m.id == s.id for s in stored_migrations)
        and not any(m.id == a.id for a in applied_migrations)
    ]
    migrations_to_update = [*migrations_to_apply, *missing_migrations]
    if migrations_to_update:
        await _write_migrations_to_pg(cur, migrations_to_update)

    return migrations_to_apply


async def _do_migrate(
    cur: psycopg.AsyncCursor,
    migrations: Collection[Migration],
    *,
    is_upgrade: bool,
    is_global: bool,
):
    """Applies the given migrations in the given order."""

    now = datetime.utcnow()
    for migration in migrations:
        func_name = f"{is_upgrade and 'upgrade' or 'downgrade'}_{is_global and 'global' or 'local'}"
        migration_file = _load_migration_from_path(migration)
        func = getattr(migration_file.module, func_name)
        logger.info("migration.apply", migration=migration, func=func, func_name=func_name)
        try:
            await func(cur)
        except Exception as e:
            logger.error(
                "migration.apply.error",
                migration=migration,
                func=func,
                func_name=func_name,
                error=e,
            )
            raise
        if is_upgrade:
            migration.applied_at = now
        else:
            migration.applied_at = None
        logger.info("migration.apply.done", migration=migration, func=func, func_name=func_name)


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
    new_object: Optional[TableObject]
    old_object: Optional[TableObject]
    diff_keys: Optional[tuple[str, ...]] = None

    def __str__(self) -> str:
        op_str = f"{self.kind.value} {self.object_kind.name}"
        if self.kind == MigrationOpKind.CREATE:
            return f"{op_str} {self.new_object.qualified_name}"
        elif self.kind == MigrationOpKind.RENAME:
            return f"{op_str} {self.old_object.qualified_name} -> {self.new_object.qualified_name}"
        elif self.kind == MigrationOpKind.UPDATE:
            diff_str = ", ".join(
                f"{k}:{getattr(self.old_object, k)}->{getattr(self.new_object, k)}"
                for k in self.diff_keys
            )
            return f"{op_str} {self.new_object.qualified_name} ({diff_str})"
        elif self.kind == MigrationOpKind.DELETE:
            return f"{op_str} {self.old_object.qualified_name}"

    def __repr__(self) -> str:
        return f"<MigrationOp {self}>"

    @property
    def object_kind(self) -> ObjectKind:
        if self.new_object is not None:
            return self.new_object.kind
        else:
            return self.old_object.kind

    @property
    def table(self) -> "Table":
        return self.new_object.table if self.new_object else self.old_object.table

    def invert(self) -> "MigrationOp":
        """Returns the inverse of this operation."""
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


def generate_migration_ops(
    old_tables: list[Table], new_tables: list[Table], *, use_source_as_id: bool = False
) -> list[MigrationOp]:
    """Generates the migration operations to go from the old tables to the new tables."""

    def _to_id(obj: TableObject) -> str:
        if use_source_as_id:
            return obj._source or obj.qualified_name
        else:
            return obj.qualified_name

    ops: list[MigrationOp] = []
    old_objects: list[TableObject] = [obj for table in old_tables for obj in table.walk()]
    old_objects_by_id: dict[str, TableObject] = {_to_id(obj): obj for obj in old_objects}
    new_objects: list[TableObject] = [obj for table in new_tables for obj in table.walk()]
    new_objects_by_id: dict[str, TableObject] = {_to_id(obj): obj for obj in new_objects}

    # diff objects
    for new_object in new_objects:
        new_id = _to_id(new_object)
        old_object = old_objects_by_id.get(new_id)
        if old_object is None:
            # create columns only if parent table isn't new
            if (
                new_object.kind == ObjectKind.COLUMN
                and _to_id(new_object.table) in new_objects_by_id
                and _to_id(new_object.table) not in old_objects_by_id
            ):
                continue
            ops.append(MigrationOp(MigrationOpKind.CREATE, new_object, None))
        elif new_object.name != old_object.name:
            ops.append(MigrationOp(MigrationOpKind.RENAME, new_object, old_object))
        elif old_object.hash_flat() != new_object.hash_flat():
            diff = new_object.diff_keys(old_object)
            ops.append(MigrationOp(MigrationOpKind.UPDATE, new_object, old_object, diff))
    deleted_tables: set[str] = set()
    for old_object in old_objects:
        old_id = _to_id(old_object)
        new_object = new_objects_by_id.get(old_id)
        if new_object is None:
            if old_object.kind == ObjectKind.TABLE:
                deleted_tables.add(old_id)
            elif _to_id(old_object.table) in deleted_tables:
                continue  # skip, table deleted
            ops.append(MigrationOp(MigrationOpKind.DELETE, None, old_object))

    return ops


def generate_migration_code(
    migration: Migration,
    *,
    global_ops: list[MigrationOp],
    local_ops: list[MigrationOp],
    exclude_inverse: bool = False,
) -> str:
    """Generates the Python migration file."""
    migration_code = Path(MIGRATIONS_TEMPLATE_PATH).read_text()

    # impute header/metadata
    today = datetime.today().date().strftime("%Y.%m.%d")
    metadata_substitutions: dict[str, str] = {
        "# <Header>": f"# This file was automatically generated by Bench on {today}. Edit as needed.",
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
        return "raise NotImplementedError()"

    lines: list[str] = []
    current_table: Optional[Table] = None
    current_statements: list[str] = []

    def _emit_alter(table: Table, statements: list[str]) -> None:
        alter_content = ",\n    ".join(statements)
        lines.append(f'"""\n    ALTER TABLE {table.name}    \n    {alter_content}\n"""')

    def _emit(table: Table, statements: list[str]) -> None:
        # batch successive ALTER TABLE statements, otherwise leave them as-is
        lines.append(f"\n# {table.name}")
        current_alter_statements: list[str] = []
        for i, stmt in enumerate(statements):
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

        if op.table != current_table:
            if current_statements:
                _emit(current_table, current_statements)
            current_table = op.table
            current_statements = []
        current_statements.append(stmt)

    if current_table and current_statements:
        _emit(current_table, current_statements)

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
    return method_body


def add_migration_to_fs(migration: Migration, code: str, *, overwrite: bool = False):
    """Writes the Python migration file."""
    migration_path = (
        f"{MIGRATIONS_PATH}/{migration.id:04d}_{migration.version.replace('.', '_')}.py"
    )
    if not overwrite and os.path.exists(migration_path):
        raise RuntimeError(f"migration file already exists: {migration_path} (for {migration!r})")
    Path(migration_path).write_text(code)
    return migration_path


def _render_migration_op(op: MigrationOp) -> Optional[str | tuple[str, str]]:
    """
    Renders the given operation into a SQL operation.
    Generally 'flat' (does not include nested objects) except for table create.
    """
    if op.kind == MigrationOpKind.CREATE:
        if isinstance(op.new_object, Table):
            table_contents = ",\n".join(f"    {col.sql()}" for col in op.new_object.columns)
            return f"CREATE TABLE {op.new_object.name} (\n{table_contents}\n)"
        elif isinstance(op.new_object, Column):
            return f"ALTER TABLE {op.new_object.table.name} ADD COLUMN {op.new_object.sql()}"
        elif isinstance(op.new_object, Index):
            return f"CREATE INDEX {op.new_object.sql()}"
        elif isinstance(op.new_object, Constraint):
            return f"ALTER TABLE {op.new_object.table.name} ADD CONSTRAINT {op.new_object.sql()}"

    elif op.kind == MigrationOpKind.RENAME:
        if isinstance(op.old_object, Table):
            return f"ALTER TABLE {op.old_object.name} RENAME TO {op.new_object.name}"
        elif isinstance(op.old_object, Column):
            return f"ALTER TABLE {op.old_object.table.name} RENAME COLUMN {op.old_object.name} TO {op.new_object.name}"
        elif isinstance(op.old_object, Index):
            return f"ALTER INDEX {op.old_object.name} RENAME TO {op.new_object.name}"
        elif isinstance(op.old_object, Constraint):
            return f"ALTER TABLE {op.old_object.table.name} RENAME CONSTRAINT {op.old_object.name} TO {op.new_object.name}"

    elif op.kind == MigrationOpKind.UPDATE:
        if isinstance(op.old_object, Table):
            # there are no table properties we can update (outside name, which is handled by rename)
            raise NotImplementedError(f"cannot render {op!r}")
        elif isinstance(op.old_object, Column):
            assert isinstance(op.new_object, Column), f"expected a column: {op.new_object!r}"
            updates: list[str] = []
            if "is_unique" in op.diff_keys:
                pass  # noop, already handled by generated index
            if "is_encrypted" in op.diff_keys:
                pass  # noop, handled in read/write logic
            if any(k in op.diff_keys for k in ("type", "is_array", "length")):
                # change type
                updates.append(
                    f"ALTER COLUMN {op.old_object.name}"
                    f" SET DATA TYPE {op.new_object.type_sql()}"
                )
            if "is_nullable" in op.diff_keys:
                # change nullability
                updates.append(
                    f"ALTER COLUMN {op.old_object.name}"
                    f" {op.new_object.is_nullable and 'DROP' or 'SET'} NOT NULL"
                )
            if "default" in op.diff_keys:
                # change default
                if op.new_object.default is None:
                    updates.append(f"ALTER COLUMN {op.old_object.name}" f" DROP DEFAULT")
                else:
                    updates.append(
                        f"ALTER COLUMN {op.old_object.name} SET DEFAULT {op.new_object.default}"
                    )
            if "is_foreign_key_to" in op.diff_keys or "on_delete" in op.diff_keys:
                # drop and recreate foreign key constraint
                if op.old_object.is_foreign_key_to:
                    # selects inside DDL aren't technically allowed, so we factor them out in post-processing
                    updates.append(
                        f"DROP CONSTRAINT"
                        f" (SELECT constraint_name FROM information_schema.table_constraints WHERE table_schema = 'public' AND table_name = '{op.old_object.table.name}' AND constraint_type = 'FOREIGN KEY' AND constraint_name LIKE '%{op.old_object.name}%')"
                    )
                if op.new_object.is_foreign_key_to:
                    constraint_name = f"{op.new_object.qualified_name.replace('.', '_')}_fk_{op.new_object.is_foreign_key_to}_id"
                    updates.append(
                        f"ADD CONSTRAINT {constraint_name}"
                        f" FOREIGN KEY ({op.new_object.name})"
                        f" REFERENCES {op.new_object.is_foreign_key_to}(id)"
                        f" ON DELETE {op.new_object.on_delete.value}"
                    )
            if "is_primary_key" in op.diff_keys:
                if op.old_object.is_primary_key:  # drop it
                    updates.append(
                        f"DROP CONSTRAINT"
                        f" (SELECT constraint_name FROM information_schema.table_constraints WHERE table_schema = 'public' AND table_name = '{op.old_object.table.name}' AND constraint_type = 'PRIMARY KEY' AND constraint_name LIKE '%{op.old_object.name}%')"
                    )
                else:  # create it
                    updates.append(
                        f"ADD CONSTRAINT {op.new_object.table}_pkey"
                        f" PRIMARY KEY ({op.new_object.name})"
                    )
            if not updates:
                return None
            return f"ALTER TABLE {op.old_object.table.name} " + ",\n".join(updates)
        elif isinstance(op.old_object, Index):
            # drop and recreate
            drop = f"DROP INDEX {op.old_object.name}"
            create = f"CREATE INDEX {op.new_object.sql()}"
            return "\n".join([drop, create])
        elif isinstance(op.old_object, Constraint):
            # drop and recreate
            drop = f"ALTER TABLE {op.old_object.table.name} DROP CONSTRAINT {op.old_object.name}"
            create = f"ALTER TABLE {op.new_object.table.name} ADD CONSTRAINT {op.new_object.sql()}"
            return "\n".join([drop, create])

    elif op.kind == MigrationOpKind.DELETE:
        if isinstance(op.old_object, Table):
            return f"DROP TABLE {op.old_object.name}"
        elif isinstance(op.old_object, Column):
            return f"ALTER TABLE {op.old_object.table.name} DROP COLUMN {op.old_object.name}"
        elif isinstance(op.old_object, Index):
            return f"DROP INDEX {op.old_object.name}"
        elif isinstance(op.old_object, Constraint):
            return f"ALTER TABLE {op.old_object.table.name} DROP CONSTRAINT {op.old_object.name}"

    raise RuntimeError(f"unexpected migration op: {op!r}")


#
# Introspection
#


async def introspect_tables_from_pg(
    cur: psycopg.AsyncCursor,
    *,
    include_columns: bool = True,
    include_constraints: bool = True,
    include_indexes: bool = True,
    table_prefix: str = "bench_",
) -> list[Table]:
    start = asyncio.get_running_loop().time()
    logger.info(
        "introspect",
        cur=cur,
        include_columns=include_columns,
        include_constraints=include_constraints,
        include_indexes=include_indexes,
    )

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
    tables_query = sql.SQL(tables_query).format(sql.Literal(table_prefix + "%"))
    tables_rows = await pg_select_raw(cur, tables_query)
    tables_names: list[str] = [str(row["table_name"]) for row in tables_rows]

    # columns
    if include_columns:
        # TODO @Performance: improve introspect tables performance
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
   string_agg(ccu.table_name, ',') AS foreign_table_names,
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
            column_type = COLUMN_TYPE_BY_POSTGRES_TYPE[postgres_type]
            is_foreign_key_to = (
                row["foreign_table_names"]
                if "FOREIGN KEY" in (row["constraint_types"] or "")
                else None
            )
            cascade_action = CascadeAction(row["delete_rules"]) if row.get("delete_rules") else None
            column = Column(
                name=row["column_name"],
                type=column_type,
                is_primary_key="PRIMARY KEY" in (row["constraint_types"] or ""),
                is_foreign_key_to=is_foreign_key_to,
                on_delete=cascade_action,
                is_unique="UNIQUE" in (row["constraint_types"] or ""),
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
        constraints_rows = await pg_select_raw(cur, constraints_query)
        constraints_by_table: dict[str, list[Constraint]] = defaultdict(list)
        for row in constraints_rows:
            columns = row["column_names"].split(", ") if row["column_names"] else []
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
            if constraint.type == ConstraintType.UNIQUE and len(constraint.columns) == 1:
                continue  # skip simple unique constraints, already handled by columns
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
        indexes_rows = await pg_select_raw(cur, indexes_query)
        for row in indexes_rows:
            definition = row["index_definition"]
            columns_str = definition.split("(")[1].split(")")[0]
            columns = [col.strip() for col in columns_str.split(",")]
            if not columns:
                columns = None
            # definition like 'CREATE INDEX index_name ON table_name USING index_type (columns) [WHERE condition]'
            index_type = re.search(r"USING (\w+)", definition).group(1)
            condition = (
                re.search(r"WHERE (.+)", definition).group(1) if "WHERE" in definition else None
            )
            if condition:
                condition = _strip_condition(condition)
            table_name = row["table_name"]
            index_name = row["index_name"][len(table_name) + 1 :]
            index = Index(
                inner_name=index_name,
                type=IndexType(index_type.upper()),
                columns=tuple(columns),
                condition=condition,
            )
            # ignore simple primary/foreign key index
            if (
                index.type == IndexType.BTREE
                and len(index.columns) == 1
                and (index.columns[0].endswith("_id") or index.columns[0] == "id")
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
    logger.info("introspect.done", cur=cur, duration=duration, tables=tables)

    return tables
