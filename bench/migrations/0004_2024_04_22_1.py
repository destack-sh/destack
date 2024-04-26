# This migration was automatically generated on 2024.04.22. Edit as needed.
import psycopg

ID = 4
VERSION = "2024.04.22.1"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_block
    await cur.execute(
        """
        ALTER TABLE bench_block    
        ADD COLUMN is_template boolean NOT NULL DEFAULT false
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
