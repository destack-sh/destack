# This migration was automatically generated on 2024.05.20. Edit as needed.
import psycopg

ID = 4
VERSION = "2024.05.20.1"
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
    # bench_notification
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "block_id" uuid')
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "block_ck" uuid')
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "block_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "step_id" uuid')
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "step_ck" uuid')
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "step_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "session_id" uuid')
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "session_ck" uuid')
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "run_id" uuid')
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "run_ck" uuid')
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "run_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "client_id" uuid')
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "server_id" uuid')
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "user_id" uuid')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
