# This migration was automatically generated on 2025.02.22. Edit as needed.
import psycopg

ID = 13
VERSION = "2025.02.22.0"
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
    # bench_identity
    await cur.execute('DROP TABLE "bench_identity"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "identity_bench_id"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "identity_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "identity_id"')

    # bench_session
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "identity_bench_id"')
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "identity_ck"')
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "identity_id"')

    # bench_run_span
    await cur.execute('ALTER TABLE "bench_run_span" DROP COLUMN "identity_bench_id"')
    await cur.execute('ALTER TABLE "bench_run_span" DROP COLUMN "identity_ck"')
    await cur.execute('ALTER TABLE "bench_run_span" DROP COLUMN "identity_id"')

    # bench_interruption
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "identity_bench_id"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "identity_ck"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "identity_id"')

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "identity_bench_id"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "identity_ck"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "identity_id"')

    # bench_plan
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "identity_bench_id"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "identity_ck"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "identity_id"')

    # bench_flow
    await cur.execute('ALTER TABLE "bench_flow" DROP COLUMN "identity_bench_id"')
    await cur.execute('ALTER TABLE "bench_flow" DROP COLUMN "identity_ck"')
    await cur.execute('ALTER TABLE "bench_flow" DROP COLUMN "identity_id"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
