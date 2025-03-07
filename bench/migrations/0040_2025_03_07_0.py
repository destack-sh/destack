# This migration was automatically generated on 2025.03.07. Edit as needed.
import psycopg

ID = 40
VERSION = "2025.03.07.0"
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
    # bench_interruption
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "message_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "message_bench_id"')

    # bench_task
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "node_bench_id"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "node_ck"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "node_id"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "node_type"')

    # bench_interruption
    await cur.execute('ALTER TABLE "bench_interruption" ADD COLUMN "task_id" uuid')

    # bench_task
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "target_id" uuid')
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "target_ck" uuid')
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "target_type" smallint')
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "target_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "interruption_id" uuid')
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "interruption_base_ck" uuid')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
