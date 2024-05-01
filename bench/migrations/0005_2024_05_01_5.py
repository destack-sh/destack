# This migration was automatically generated on 2024.05.01. Edit as needed.
import psycopg

ID = 5
VERSION = "2024.05.01.5"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_field
    await cur.execute(
        """
        ALTER TABLE bench_field    
        DROP COLUMN base_field_kind,
        ADD COLUMN base_field_zone smallint
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
