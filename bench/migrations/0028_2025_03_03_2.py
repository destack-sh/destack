# This migration was automatically generated on 2025.03.03. Edit as needed.
import psycopg

ID = 28
VERSION = "2025.03.03.2"
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
    # bench_session
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "run_base_ck"')
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "run_id"')

    # bench_run_span
    await cur.execute('ALTER TABLE "bench_run_span" DROP COLUMN "run_base_ck"')
    await cur.execute('ALTER TABLE "bench_run_span" DROP COLUMN "run_id"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "run_base_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "run_id"')

    # bench_interruption
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "run_base_ck"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "run_id"')

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "run_base_ck"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "run_id"')

    # bench_task
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "run_base_ck"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "run_id"')

    # bench_plan
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "run_base_ck"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "run_id"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
