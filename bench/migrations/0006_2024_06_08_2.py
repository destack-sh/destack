# This migration was automatically generated on 2024.06.08. Edit as needed.
import psycopg

ID = 6
VERSION = "2024.06.08.2"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_server
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "current_status" smallint')

    # bench_store
    await cur.execute('ALTER TABLE "bench_store" ADD COLUMN "current_status" smallint')

    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "current_status" smallint')

    # bench_drive
    await cur.execute('ALTER TABLE "bench_drive" ADD COLUMN "current_status" smallint')

    # bench_blob
    await cur.execute('ALTER TABLE "bench_blob" ADD COLUMN "current_status" smallint')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "current_status" smallint')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
