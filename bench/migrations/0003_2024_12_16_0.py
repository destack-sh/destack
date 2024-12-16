# This migration was automatically generated on 2024.12.16. Edit as needed.
import psycopg

ID = 3
VERSION = "2024.12.16.0"
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
    # bench_message
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "mode"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "package_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "parent_base_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "parent_type"')

    # bench_session
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "package_id"')
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "parent_ck"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "interrupt_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "package_id"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "parent_ck"')

    # bench_interrupt
    await cur.execute('ALTER TABLE "bench_interrupt" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_interrupt" DROP COLUMN "package_id"')

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "package_id"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "parent_ck"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
