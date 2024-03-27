# This migration was automatically generated on 2024.03.27. Edit as needed.
import psycopg

ID = 5
VERSION = "2024.03.27.4"
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
    # bench_log
    await cur.execute(
        """
        ALTER TABLE bench_log    
        DROP COLUMN value_dynamic,
        ADD COLUMN value_packed jsonb
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
