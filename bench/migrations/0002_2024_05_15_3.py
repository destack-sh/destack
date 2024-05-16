# This migration was automatically generated on 2024.05.16. Edit as needed.
import psycopg

ID = 2
VERSION = "2024.05.15.3"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_environment
    await cur.execute('ALTER TABLE "bench_environment" DROP COLUMN "analytics_store_id"')
    await cur.execute('ALTER TABLE "bench_environment" DROP COLUMN "cache_id"')
    await cur.execute('ALTER TABLE "bench_environment" DROP COLUMN "search_store_id"')

    # bench_bench
    await cur.execute('ALTER TABLE "bench_bench" DROP COLUMN "published_branch_id"')

    # bench_cache
    await cur.execute('DROP TABLE "bench_cache"')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
