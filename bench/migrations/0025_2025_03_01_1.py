# This migration was automatically generated on 2025.03.01. Edit as needed.
import psycopg

ID = 25
VERSION = "2025.03.01.1"
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
    # bench_action
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "plans"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "plan_step"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "title"')

    # bench_plan
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "block_bench_id"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "block_ck"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "block_id"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "execution"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "icon"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "name"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "on_error"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "on_terminate"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "order_key"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "package_id"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "tags_ck"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "tags_id"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "template_at"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "template_id"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "text"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "thread_id"')
    await cur.execute(
        'ALTER TABLE "bench_plan" ADD COLUMN "termination_mode" smallint NOT NULL DEFAULT 20'
    )
    await cur.execute(
        'ALTER TABLE "bench_plan" ADD COLUMN "failure_mode" smallint NOT NULL DEFAULT 20'
    )
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "session_id" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "run_id" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "run_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "client_id" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "machine_id" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "user_id" uuid')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "task_id" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "task_ck" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "name" varchar')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
