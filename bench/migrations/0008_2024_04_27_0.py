# This migration was automatically generated on 2024.04.27. Edit as needed.
import psycopg

ID = 8
VERSION = "2024.04.27.0"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_environment
    await cur.execute(
        """
        ALTER TABLE bench_environment    
        ALTER COLUMN search_store_id DROP NOT NULL
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
