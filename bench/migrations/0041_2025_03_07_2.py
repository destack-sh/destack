# This migration was automatically generated on 2025.03.07. Edit as needed.
import psycopg

ID = 41
VERSION = "2025.03.07.2"
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
    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "trigger_run_base_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "trigger_run_id"')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "channel_id" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "channel_ck" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "kit_id" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "kit_ck" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "task_ck" uuid')

    # bench_interruption
    await cur.execute('ALTER TABLE "bench_interruption" ADD COLUMN "task_ck" uuid')

    # bench_task
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "template_ck" uuid')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
