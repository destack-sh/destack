# This migration was automatically generated on 2024.05.16. Edit as needed.
import psycopg

ID = 3
VERSION = "2024.05.16.0"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_store
    await cur.execute('ALTER TABLE "bench_store" DROP COLUMN "engine"')
    await cur.execute('ALTER TABLE "bench_store" DROP COLUMN "kind"')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
