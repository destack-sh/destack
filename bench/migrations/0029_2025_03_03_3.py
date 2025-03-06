# This migration was automatically generated on 2025.03.03. Edit as needed.
import psycopg

ID = 29
VERSION = "2025.03.03.3"
HAS_GLOBAL = False
HAS_REGIONAL = False
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    pass


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_plan
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "package_id" uuid NOT NULL')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "package_ck" uuid NOT NULL')

    # bench_task
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "package_id" uuid NOT NULL')
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "package_ck" uuid NOT NULL')

    # bench_session
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "package_id" uuid NOT NULL')
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "package_ck" uuid NOT NULL')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "package_id" uuid NOT NULL')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "package_ck" uuid NOT NULL')

    # bench_run_span
    await cur.execute('ALTER TABLE "bench_run_span" ADD COLUMN "package_id" uuid NOT NULL')
    await cur.execute('ALTER TABLE "bench_run_span" ADD COLUMN "package_ck" uuid NOT NULL')

    # bench_interruption
    await cur.execute('ALTER TABLE "bench_interruption" ADD COLUMN "package_id" uuid NOT NULL')
    await cur.execute('ALTER TABLE "bench_interruption" ADD COLUMN "package_ck" uuid NOT NULL')

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "package_id" uuid NOT NULL')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "package_ck" uuid NOT NULL')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
