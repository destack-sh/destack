# This migration was automatically generated on 2024.02.13. Edit as needed.
import psycopg

ID = 2
VERSION = "2024.02.13.0"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_skip
    await cur.execute(
        """
        ALTER TABLE bench_skip    
        ADD COLUMN reference_dependency_ck uuid NOT NULL,
        ADD COLUMN reference_upgrade_ck uuid NOT NULL
    """
    )

    # bench_dependency
    await cur.execute(
        """
    CREATE TABLE bench_dependency (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        scopes_block_ck uuid[] NOT NULL
    )
    """
    )

    # bench_link
    await cur.execute(
        """
        ALTER TABLE bench_link    
        ADD COLUMN reference_dependency_ck uuid,
        ADD COLUMN reference_upgrade_ck uuid
    """
    )

    # bench_upgrade
    await cur.execute(
        """
    CREATE TABLE bench_upgrade (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        name varchar,
        text jsonb
    )
    """
    )

    # bench_dependency
    await cur.execute(
        """
        ALTER TABLE bench_dependency    
        ADD COLUMN parent_package_id uuid REFERENCES bench_package ON DELETE CASCADE,
        ADD COLUMN package_id uuid NOT NULL REFERENCES bench_package ON DELETE CASCADE,
        ADD COLUMN dependency_package_id uuid NOT NULL REFERENCES bench_package ON DELETE SET NULL
    """
    )
    await cur.execute(
        "CREATE INDEX bench_dependency_bench_idx_package_deleted_at ON bench_dependency USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_dependency_bench_idx_package_archived_at ON bench_dependency USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_dependency    
        ADD CONSTRAINT bench_dependency_bench_check_one_parent CHECK ((parent_package_id IS NOT NULL))
    """
    )

    # bench_upgrade
    await cur.execute(
        """
        ALTER TABLE bench_upgrade    
        ADD COLUMN parent_package_id uuid REFERENCES bench_package ON DELETE CASCADE,
        ADD COLUMN package_id uuid NOT NULL REFERENCES bench_package ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_upgrade_bench_idx_package_deleted_at ON bench_upgrade USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_upgrade_bench_idx_package_archived_at ON bench_upgrade USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_upgrade    
        ADD CONSTRAINT bench_upgrade_bench_check_one_parent CHECK ((parent_package_id IS NOT NULL))
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
