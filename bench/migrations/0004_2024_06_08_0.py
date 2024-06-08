# This migration was automatically generated on 2024.06.08. Edit as needed.
import psycopg

ID = 4
VERSION = "2024.06.08.0"
HAS_GLOBAL = False
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass


async def downgrade_global(cur: psycopg.AsyncCursor):
    pass


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_notification
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "ck" uuid NOT NULL')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "ck" uuid NOT NULL')


async def downgrade_local(cur: psycopg.AsyncCursor):
    # bench_message
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "ck"')

    # bench_notification
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "ck"')
