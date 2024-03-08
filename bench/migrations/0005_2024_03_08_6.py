# This migration was automatically generated on 2024.03.08. Edit as needed.
import psycopg

ID = 5
VERSION = "2024.03.08.6"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_view
    await cur.execute(
        """
        ALTER TABLE bench_view    
        ALTER COLUMN is_visible DROP NOT NULL,
        ALTER COLUMN is_disabled DROP NOT NULL,
        ALTER COLUMN is_loading DROP NOT NULL,
        ALTER COLUMN is_input DROP NOT NULL,
        ALTER COLUMN is_secret DROP NOT NULL
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
