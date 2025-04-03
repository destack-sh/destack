# This migration was automatically generated on 2025.04.03. Edit as needed.
import psycopg

ID = 43
VERSION = "2025.04.03.2"
HAS_GLOBAL = False
HAS_REGIONAL = True
HAS_LOCAL = False


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
    # bench_link
    await cur.execute(
        """
        ALTER TABLE "bench_link"    
        DROP COLUMN "is_manual",
        DROP COLUMN "trigger"
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        DROP COLUMN "incoming_base_id",
        DROP COLUMN "incoming_id",
        DROP COLUMN "plan_ck",
        DROP COLUMN "plan_id",
        DROP COLUMN "task_ck",
        DROP COLUMN "task_id",
        DROP COLUMN "trigger_key"
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        DROP COLUMN "on_failure"
    """
    )

    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        DROP COLUMN "plan_ck",
        DROP COLUMN "plan_id",
        DROP COLUMN "task_ck",
        DROP COLUMN "task_id",
        DROP COLUMN "trigger_key"
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        ADD COLUMN "manual_plan_id" uuid,
        ADD COLUMN "manual_plan_ck" uuid,
        ADD COLUMN "manual_task_id" uuid,
        ADD COLUMN "manual_task_ck" uuid,
        ADD COLUMN "run_plan_id" uuid,
        ADD COLUMN "run_task_id" uuid
    """
    )

    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        ADD COLUMN "manual_plan_id" uuid,
        ADD COLUMN "manual_plan_ck" uuid,
        ADD COLUMN "manual_task_id" uuid,
        ADD COLUMN "manual_task_ck" uuid,
        ADD COLUMN "run_plan_id" uuid,
        ADD COLUMN "run_task_id" uuid
    """
    )


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
