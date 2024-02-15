# This migration was automatically generated on 2024.02.15. Edit as needed.
import psycopg

ID = 5
VERSION = "2024.02.15.3"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_notification
    await cur.execute(
        """
        ALTER TABLE bench_notification    
        DROP COLUMN parent_user_id
    """
    )
    await cur.execute("DROP INDEX bench_notification_bench_idx_deleted_at")
    await cur.execute("DROP INDEX bench_notification_bench_idx_archived_at")

    # bench_user
    await cur.execute(
        """
        ALTER TABLE bench_user    
        ADD COLUMN status smallint NOT NULL
    """
    )

    # bench_organization
    await cur.execute(
        """
        ALTER TABLE bench_organization    
        ADD COLUMN status smallint NOT NULL
    """
    )

    # bench_notification
    await cur.execute(
        """
        ALTER TABLE bench_notification    
        ADD COLUMN ck uuid NOT NULL,
        ADD COLUMN kind smallint NOT NULL,
        ADD COLUMN sender_block_ck uuid,
        ADD COLUMN title varchar,
        ADD COLUMN text jsonb,
        ADD COLUMN value_packed jsonb,
        ADD COLUMN secret_value_packed bytea,
        ADD COLUMN parent_package_id uuid REFERENCES bench_package ON DELETE CASCADE,
        ADD COLUMN package_id uuid NOT NULL REFERENCES bench_package ON DELETE CASCADE,
        ADD COLUMN sender_bench_id uuid REFERENCES bench_bench ON DELETE SET NULL
    """
    )
    await cur.execute(
        "CREATE INDEX bench_notification_bench_idx_package_deleted_at ON bench_notification USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_notification_bench_idx_package_archived_at ON bench_notification USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_notification    
        DROP CONSTRAINT IF EXISTS bench_notification_bench_check_one_parent, ADD CONSTRAINT bench_notification_bench_check_one_parent CHECK ((parent_package_id IS NOT NULL))
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
