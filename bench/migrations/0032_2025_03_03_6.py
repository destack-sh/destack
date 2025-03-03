# This migration was automatically generated on 2025.03.03. Edit as needed.
import psycopg

ID = 32
VERSION = "2025.03.03.6"
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
    # bench_thread
    await cur.execute('ALTER TABLE "bench_thread" DROP COLUMN "owned_by_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_thread" DROP COLUMN "owned_by_base_ck"')

    # bench_space
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "owned_by_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "owned_by_base_ck"')

    # bench_package
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "owned_by_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "owned_by_base_ck"')

    # bench_plan
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "owned_by_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "owned_by_base_ck"')

    # bench_task
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "owned_by_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "owned_by_base_ck"')

    # bench_plan
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "implemented_by_id" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "implemented_by_base_ck" uuid')

    # bench_task
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "thread_id" uuid')
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "tags_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "tags_ck" uuid[]')
    await cur.execute(
        'ALTER TABLE "bench_task" ADD COLUMN "is_manual" boolean NOT NULL DEFAULT false'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
