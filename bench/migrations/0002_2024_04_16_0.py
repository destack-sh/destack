# This migration was automatically generated on 2024.04.16. Edit as needed.
import psycopg

ID = 2
VERSION = "2024.04.16.0"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_space
    await cur.execute(
        """
        ALTER TABLE bench_space    
        ADD COLUMN base_id uuid,
        ADD COLUMN base_ck uuid,
        ADD COLUMN base_type smallint,
        ADD COLUMN base_bench_id uuid,
        ADD COLUMN base_base_ck uuid,
        ADD COLUMN base_base_bench_id uuid
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
