# This migration was automatically generated on 2024.05.09. Edit as needed.
import psycopg

ID = 5
VERSION = "2024.05.09.0"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_space
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "bar_position" smallint DEFAULT 1')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
