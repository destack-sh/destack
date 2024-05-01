# This migration was automatically generated on 2024.05.01. Edit as needed.
import psycopg

ID = 3
VERSION = "2024.05.01.3"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_notice
    await cur.execute(
        """
        ALTER TABLE bench_notice    
        DROP COLUMN message,
        ADD COLUMN title varchar,
        ADD COLUMN text jsonb
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_log
    await cur.execute(
        """
        ALTER TABLE bench_log    
        DROP COLUMN message
    """
    )

    # bench_notification
    await cur.execute(
        """
        ALTER TABLE bench_notification    
        DROP COLUMN sender_bench_id,
        DROP COLUMN sender_ck,
        DROP COLUMN sender_id
    """
    )

    # bench_signal
    await cur.execute(
        """
        ALTER TABLE bench_signal    
        DROP COLUMN sender_bench_id,
        DROP COLUMN sender_ck,
        DROP COLUMN sender_id
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE bench_run    
        ADD COLUMN step_id uuid,
        ADD COLUMN step_ck uuid,
        ADD COLUMN step_bench_id uuid,
        ADD COLUMN code jsonb,
        ADD COLUMN text jsonb,
        ADD COLUMN paused_at timestamp
    """
    )

    # bench_signal
    await cur.execute(
        """
        ALTER TABLE bench_signal    
        ADD COLUMN origin_id uuid,
        ADD COLUMN origin_ck uuid,
        ADD COLUMN origin_bench_id uuid
    """
    )

    # bench_log
    await cur.execute(
        """
        ALTER TABLE bench_log    
        ADD COLUMN title varchar,
        ADD COLUMN step_id uuid,
        ADD COLUMN step_ck uuid,
        ADD COLUMN step_bench_id uuid
    """
    )

    # bench_notification
    await cur.execute(
        """
        ALTER TABLE bench_notification    
        ADD COLUMN origin_id uuid,
        ADD COLUMN origin_ck uuid,
        ADD COLUMN origin_bench_id uuid
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE bench_run    
        ALTER COLUMN duration DROP NOT NULL,
    ALTER COLUMN duration DROP DEFAULT
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
