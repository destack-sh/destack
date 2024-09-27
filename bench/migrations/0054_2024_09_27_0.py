# This migration was automatically generated on 2024.09.27. Edit as needed.
import psycopg

ID = 54
VERSION = "2024.09.27.0"
HAS_GLOBAL = False
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "pipe_id" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "pipe_ck" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "pipe_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "view_id" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "view_ck" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "view_bench_id" uuid')

    # bench_signal
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "pipe_id" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "pipe_ck" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "pipe_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "view_id" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "view_ck" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "view_bench_id" uuid')

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "pipe_id" uuid')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "pipe_ck" uuid')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "pipe_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "view_id" uuid')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "view_ck" uuid')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "view_bench_id" uuid')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
