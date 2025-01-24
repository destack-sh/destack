# This migration was automatically generated on 2025.01.24. Edit as needed.
import psycopg

ID = 7
VERSION = "2025.01.24.0"
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
    # bench_pipe
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "condition"')
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "constraint"')

    # bench_interruption
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "attempt_no"')

    # bench_run_plan
    await cur.execute('ALTER TABLE "bench_run_plan" DROP COLUMN "duration"')
    await cur.execute('ALTER TABLE "bench_run_plan" DROP COLUMN "runs_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_run_plan" DROP COLUMN "runs_base_ck"')
    await cur.execute('ALTER TABLE "bench_run_plan" DROP COLUMN "runs_bench_id"')
    await cur.execute('ALTER TABLE "bench_run_plan" DROP COLUMN "runs_id"')
    await cur.execute('ALTER TABLE "bench_run_plan" DROP COLUMN "started_at"')
    await cur.execute('ALTER TABLE "bench_run_plan" DROP COLUMN "terminated_at"')

    # bench_pipe
    await cur.execute('ALTER TABLE "bench_pipe" ADD COLUMN "trigger" smallint NOT NULL DEFAULT 1')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "title" varchar')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "text" jsonb')

    # bench_run_plan
    await cur.execute('ALTER TABLE "bench_run_plan" ADD COLUMN "started_by_id" uuid')
    await cur.execute('ALTER TABLE "bench_run_plan" ADD COLUMN "started_by_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_run_plan" ADD COLUMN "started_by_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_run_plan" ADD COLUMN "started_by_base_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_run_plan" ADD COLUMN "terminated_by_id" uuid')
    await cur.execute('ALTER TABLE "bench_run_plan" ADD COLUMN "terminated_by_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_run_plan" ADD COLUMN "terminated_by_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_run_plan" ADD COLUMN "terminated_by_base_bench_id" uuid')

    # bench_interruption
    await cur.execute('ALTER TABLE "bench_interruption" ADD COLUMN "attempt" integer')
    await cur.execute(
        """
        ALTER TABLE bench_interruption    
        ALTER COLUMN "status" SET DEFAULT 10
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
