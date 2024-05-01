# This migration was automatically generated on 2024.05.01. Edit as needed.
import psycopg

ID = 2
VERSION = "2024.05.01.2"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_notice
    await cur.execute(
        """
        ALTER TABLE bench_notice    
        ADD COLUMN origin_id uuid,
        ADD COLUMN origin_ck uuid,
        ADD COLUMN origin_type smallint,
        ADD COLUMN origin_bench_id uuid,
        ADD COLUMN origin_base_ck uuid,
        ADD COLUMN origin_base_bench_id uuid
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
