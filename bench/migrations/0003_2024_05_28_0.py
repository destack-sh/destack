# This migration was automatically generated on 2024.05.28. Edit as needed.
import psycopg

ID = 3
VERSION = "2024.05.28.0"
HAS_GLOBAL = False
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_run
    await cur.execute(
        """
        ALTER TABLE bench_run    
        ALTER COLUMN root_base_ck DROP NOT NULL
    """
    )

    # bench_message
    await cur.execute(
        """
        ALTER TABLE bench_message    
        ALTER COLUMN origin_base_ck DROP NOT NULL,
        ALTER COLUMN origin_base_bench_id DROP NOT NULL
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
