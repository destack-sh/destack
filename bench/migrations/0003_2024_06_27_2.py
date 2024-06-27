# This migration was automatically generated on 2024.06.27. Edit as needed.
import psycopg

ID = 3
VERSION = "2024.06.27.2"
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
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "category" smallint')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "nodes_total" integer')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
