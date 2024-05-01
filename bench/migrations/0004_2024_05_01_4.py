# This migration was automatically generated on 2024.05.01. Edit as needed.
import psycopg

ID = 4
VERSION = "2024.05.01.4"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_field
    await cur.execute(
        """
        ALTER TABLE bench_field    
        ADD COLUMN zone smallint NOT NULL DEFAULT 1,
        ALTER COLUMN kind DROP NOT NULL,
    ALTER COLUMN kind DROP DEFAULT
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
        ADD COLUMN kind smallint NOT NULL
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
