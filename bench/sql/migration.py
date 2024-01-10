from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime
import os
from pathlib import Path
import re
from typing import Any, Optional, Callable, Awaitable

import psycopg
from psycopg import sql

from bench.sql.core import (
    Object,
    Table,
    Column,
    Index,
    Constraint,
    MIGRATION_TABLE,
    PostgresColumnType,
    COLUMN_TYPE_BY_POSTGRES_TYPE,
    ConstraintType,
    IndexType,
    CascadeAction,
)
from bench.sql.engine import SqlUndefinedObject, pg_select

MIGRATIONS_PATH = "bench/sql/migrations"
MIGRATIONS_TEMPLATE_PATH = "bench/sql/migrations/0000_template.py"


#
# Managing migrations
#


@dataclass
class Migration:
    id: int
    commit: str
    version: str
    has_global: bool
    has_local: bool
    applied_at: Optional[datetime]
    path: Optional[str] = None  # not stored
    file: Optional["MigrationFile"] = None  # not stored


MigratorFunc = Callable[[psycopg.AsyncConnection], Awaitable[None]]


@dataclass
class MigrationFile:
    path: str
    module: Any
    upgrade_global: MigratorFunc
    downgrade_global: MigratorFunc
    upgrade_local: MigratorFunc
    downgrade_local: MigratorFunc


def pack_migration_row(migration: Migration) -> dict[str, Any]:
    return {
        "id": migration.id,
        "commit": migration.commit,
        "version": migration.version,
        "has_global": migration.has_global,
        "has_local": migration.has_local,
        "applied_at": migration.applied_at,
    }


def unpack_migration_row(row: dict[str, Any]) -> Migration:
    return Migration(
        id=row["id"],
        commit=row["commit"],
        version=row["version"],
        has_global=row["has_global"],
        has_local=row["has_local"],
        applied_at=row["applied_at"],
    )


async def read_migrations_from_pg(cur: psycopg.AsyncCursor, min_id: int = None) -> list[Migration]:
    """Reads the 'bench_migration' table (if it exists) and returns the corresponding Migration."""
    try:
        if min_id is not None:
            where = sql.SQL("WHERE id >= {}").format(sql.Literal(min_id))
        else:
            where = None
        migrations_rows = await pg_select(cur, MIGRATION_TABLE, where=where)
        migrations = [unpack_migration_row(row) for row in migrations_rows]
        return migrations
    except SqlUndefinedObject:
        return []


def read_migrations_from_fs() -> list[Migration]:
    """Reads the migrations from local filesystem. Actually loads each migration file."""
    migrations: list[Migration] = []
    for migration_path in sorted(os.listdir(MIGRATIONS_PATH)):
        if not migration_path.endswith(".py"):
            continue

        # parse the file
        migration_source = Path(migration_path).read_text()
        migration_metadata_str = migration_source.split("# <Metadata>")[1].split("# </Metadata>")[0]
        migration_metadata: dict[str, str] = {
            match[0]: match[1] for match in re.findall(r"(\w+) = \"(.*)\"", migration_metadata_str)
        }
        migration = Migration(
            id=int(migration_metadata["ID"]),
            commit=migration_metadata["COMMIT"],
            version=migration_metadata["VERSION"],
            has_global=migration_metadata["HAS_GLOBAL"] == "True",
            has_local=migration_metadata["HAS_LOCAL"] == "True",
            applied_at=None,
            path=migration_path,
        )
        migrations.append(migration)
    return migrations


def load_migration_from_path(migration: Migration) -> MigrationFile:
    assert migration.path is not None, "migration path not set"
    migration_module = __import__(migration.path)
    file = MigrationFile(
        path=migration.path,
        module=migration_module,
        upgrade_global=migration_module.upgrade_global,
        downgrade_global=migration_module.downgrade_global,
        upgrade_local=migration_module.upgrade_local,
        downgrade_local=migration_module.downgrade_local,
    )
    return file


#
# Introspection
#


async def introspect_tables_from_pg(cur: psycopg.AsyncCursor) -> list[Table]:
    # tables
    tables_query = """
    SELECT
        table_name
    FROM
        information_schema.tables
    WHERE
        table_schema = 'public'
        AND table_name LIKE 'bench_%';
    """
    tables_rows = await cur.execute(tables_query)
    tables_names: tuple[str] = tuple(str(row["table_name"]) for row in tables_rows)

    # columns
    columns_query = """
    SELECT 
        col.table_name, 
        col.column_name, 
        col.data_type, 
        col.is_nullable, 
        col.column_default,
        tc.constraint_type,
        kcu.column_name AS foreign_column_name,
        ccu.table_name AS foreign_table_name
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
    WHERE 
        col.table_schema = 'public' AND col.table_name IN %s;
    """
    columns_rows = await cur.execute(columns_query, (tables_names,))
    columns_by_table: dict[str, list[Column]] = defaultdict(list)
    for row in columns_rows:
        column = Column(
            name=row["column_name"],
            type=COLUMN_TYPE_BY_POSTGRES_TYPE[PostgresColumnType(str(row["data_type"]))],
            is_primary_key=row["constraint_type"] == "PRIMARY KEY",
            is_foreign_key_to=row["foreign_table_name"]
            if row["constraint_type"] == "FOREIGN KEY"
            else None,
            on_delete=CascadeAction(row["delete_rule"]) if row["delete_rule"] else None,
            is_unique=row["constraint_type"] == "UNIQUE",
            is_nullable=row["is_nullable"] == "YES",
            # nocheckin: handle Column.encrypted
            default=row["column_default"],
        )
        columns_by_table[row["table_name"]].append(column)

    # constraints
    constraints_query = """
    SELECT 
        tc.table_name,
        tc.constraint_type,
        kcu.column_name,
        ccu.table_name AS foreign_table_name,
        ccu.column_name AS foreign_column_name,
        rc.update_rule,
        rc.delete_rule,
        chk.check_clause AS condition
    FROM 
        information_schema.table_constraints AS tc
    LEFT JOIN 
        information_schema.key_column_usage AS kcu 
        ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema
    LEFT JOIN 
        information_schema.constraint_column_usage AS ccu 
        ON ccu.constraint_name = tc.constraint_name AND ccu.table_schema = tc.table_schema
    LEFT JOIN 
        information_schema.referential_constraints AS rc 
        ON tc.constraint_name = rc.constraint_name AND tc.table_schema = rc.constraint_schema
    LEFT JOIN 
        information_schema.check_constraints AS chk 
        ON tc.constraint_name = chk.constraint_name AND tc.table_schema = chk.constraint_schema
    WHERE 
        tc.table_schema = 'public' 
        AND tc.table_name IN %s
        AND tc.constraint_type NOT IN ('PRIMARY KEY', 'FOREIGN KEY');
    """
    constraints_rows = await cur.execute(constraints_query, (tables_names,))
    constraints_by_table: dict[str, list[Constraint]] = defaultdict(list)
    for row in constraints_rows:
        constraint = Constraint(
            inner_name=row["constraint_name"],
            type=ConstraintType(row["constraint_type"]),
            columns=[row["column_name"]] if row.get("column_name") else None,
            condition=row.get("condition"),
        )
        if constraint.type == ConstraintType.UNIQUE and len(constraint.columns) == 1:
            continue  # skip scalar unique constraints, already handled by columns
        constraints_by_table[row["table_name"]].append(constraint)

    # indexes
    indexes_by_table: dict[str, list[Index]] = defaultdict(list)
    indexes_query = """
        SELECT 
            idx.tablename AS table_name,
            idx.indexname AS index_name,
            idx.indexdef AS index_definition
        FROM 
            pg_indexes idx
        WHERE 
            idx.schemaname = 'public' AND idx.tablename IN %s;
        """
    indexes_rows = await cur.execute(indexes_query, (tables_names,))
    for row in indexes_rows:
        definition = row["index_definition"]
        columns_str = definition.split("(")[1].split(")")[0]
        columns = [col.strip() for col in columns_str.split(",")]
        index_type = IndexType(definition.split(" ")[0])
        if "WHERE" in definition:
            condition = definition.split("WHERE")[1].strip()
        else:
            condition = None
        index = Index(
            inner_name=row["index_name"],
            type=index_type,
            columns=columns,
            condition=condition,
        )
        indexes_by_table[row["table_name"]].append(index)

    # assemble the tables
    tables: list[Table] = []
    for table_name in tables_names:
        table = Table(
            name=table_name,
            columns=tuple(columns_by_table[table_name]),
            indexes=indexes_by_table.get(table_name, []),
            constraints=constraints_by_table.get(table_name, []),
        )
        tables.append(table)

    return tables


#
# Generating migrations
#


def create_object_flat_sql(object: Object) -> sql.Composable:
    """Gets the SQL to create the given object (without any nested objects)."""
    if isinstance(object, Table):
        return sql.SQL("CREATE TABLE {} ()").format(sql.Identifier(object.name))
    elif isinstance(object, Column):
        return sql.SQL("ALTER TABLE {} ADD COLUMN {}").format(
            sql.Identifier(object.table.name), sql.SQL(object.sql())
        )
    elif isinstance(object, Index):
        return sql.SQL("CREATE INDEX {}").format(sql.SQL(object.sql()))
    elif isinstance(object, Constraint):
        return sql.SQL("ALTER TABLE {} ADD CONSTRAINT {}").format(
            sql.Identifier(object.table.name),
            sql.SQL(object.sql()),
        )
    else:
        raise RuntimeError(f"unexpected object: {object}")


def delete_object_sql(object: Object) -> sql.Composable:
    """Gets the SQL to delete the given object (necessarily includes nested objects)."""
    if isinstance(object, Table):
        return sql.SQL("DROP TABLE {}").format(sql.Identifier(object.name))
    elif isinstance(object, Column):
        return sql.SQL("ALTER TABLE {} DROP COLUMN {}").format(
            sql.Identifier(object.table.name), sql.Identifier(object.name)
        )
    elif isinstance(object, Index):
        return sql.SQL("DROP INDEX {}").format(sql.Identifier(object.name))
    elif isinstance(object, Constraint):
        return sql.SQL("ALTER TABLE {} DROP CONSTRAINT {}").format(
            sql.Identifier(object.table.name),
            sql.Identifier(object.name),
        )
    else:
        raise RuntimeError(f"unexpected object: {object}")
