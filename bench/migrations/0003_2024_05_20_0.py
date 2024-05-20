# This migration was automatically generated on 2024.05.20. Edit as needed.
import psycopg

ID = 3
VERSION = "2024.05.20.0"
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
    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "run_id" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "run_ck" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "run_base_ck" uuid')

    # bench_signal
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "block_id" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "block_ck" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "block_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "step_id" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "step_ck" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "step_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "session_id" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "session_ck" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "run_id" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "run_ck" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "run_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "client_id" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "server_id" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "user_id" uuid')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "block_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "block_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "block_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "step_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "step_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "step_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "session_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "session_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "run_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "run_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "run_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "client_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "server_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "user_id" uuid')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
