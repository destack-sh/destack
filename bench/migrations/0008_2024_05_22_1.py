# This migration was automatically generated on 2024.05.22. Edit as needed.
import psycopg

ID = 8
VERSION = "2024.05.22.1"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_server
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "current_profile" smallint')
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "version" varchar')
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "current_version" varchar')

    # bench_store
    await cur.execute('ALTER TABLE "bench_store" ADD COLUMN "current_version" varchar')

    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "current_profile" smallint')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
