# This migration was automatically generated on 2024.05.07. Edit as needed.
import psycopg

ID = 2
VERSION = "2024.05.07.1"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_block
    await cur.execute(
        """
        ALTER TABLE bench_block    
        ALTER COLUMN visibility DROP DEFAULT
    """
    )


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
        ALTER COLUMN status SET DEFAULT 1
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
