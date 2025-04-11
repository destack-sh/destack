# This migration was automatically generated on 2025.04.11. Edit as needed.
import psycopg

ID = 51
VERSION = "2025.04.11.0"
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
    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        DROP COLUMN "agent_bench_id",
        DROP COLUMN "manual_plan_ck",
        DROP COLUMN "manual_plan_id",
        DROP COLUMN "manual_task_ck",
        DROP COLUMN "manual_task_id",
        DROP COLUMN "run_plan_id",
        DROP COLUMN "run_task_id"
    """
    )

    # bench_session
    await cur.execute(
        """
        ALTER TABLE "bench_session"    
        DROP COLUMN "agent_bench_id",
        DROP COLUMN "agent_ck",
        DROP COLUMN "agent_id"
    """
    )

    # bench_channel
    await cur.execute(
        """
        ALTER TABLE "bench_channel"    
        DROP COLUMN "agent_bench_id",
        DROP COLUMN "agent_ck",
        DROP COLUMN "agent_id"
    """
    )

    # bench_interruption
    await cur.execute(
        """
        ALTER TABLE "bench_interruption"    
        DROP COLUMN "agent_bench_id",
        DROP COLUMN "agent_ck",
        DROP COLUMN "agent_id"
    """
    )

    # bench_log
    await cur.execute(
        """
        ALTER TABLE "bench_log"    
        DROP COLUMN "agent_bench_id",
        DROP COLUMN "agent_ck",
        DROP COLUMN "agent_id"
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        DROP COLUMN "agent_bench_id",
        DROP COLUMN "agent_ck",
        DROP COLUMN "agent_id",
        DROP COLUMN "type"
    """
    )

    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        DROP COLUMN "agent_bench_id",
        DROP COLUMN "manual_plan_ck",
        DROP COLUMN "manual_plan_id",
        DROP COLUMN "manual_task_ck",
        DROP COLUMN "manual_task_id",
        DROP COLUMN "run_plan_id",
        DROP COLUMN "run_task_id"
    """
    )

    # bench_claim
    await cur.execute(
        """
        ALTER TABLE "bench_claim"    
        DROP COLUMN "agent_bench_id",
        DROP COLUMN "agent_ck",
        DROP COLUMN "agent_id"
    """
    )

    # bench_agent
    await cur.execute(
        """
        ALTER TABLE "bench_agent"    
        DROP COLUMN "agent_bench_id",
        DROP COLUMN "agent_ck",
        DROP COLUMN "agent_id",
        DROP COLUMN "main_flow_bench_id",
        DROP COLUMN "main_flow_id"
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        DROP COLUMN "agent_bench_id",
        DROP COLUMN "agent_ck",
        DROP COLUMN "agent_id",
        DROP COLUMN "type"
    """
    )

    # bench_cursor
    await cur.execute(
        """
        ALTER TABLE "bench_cursor"    
        DROP COLUMN "agent_bench_id",
        DROP COLUMN "agent_ck",
        DROP COLUMN "agent_id"
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        DROP COLUMN "agent_bench_id",
        DROP COLUMN "agent_ck",
        DROP COLUMN "agent_id"
    """
    )

    # bench_agent
    await cur.execute(
        """
        ALTER TABLE "bench_agent"    
        ADD COLUMN "computed_values" jsonb[]
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        ADD COLUMN "plan_id" uuid,
        ADD COLUMN "plan_ck" uuid,
        ADD COLUMN "task_id" uuid,
        ADD COLUMN "task_ck" uuid
    """
    )

    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        ADD COLUMN "plan_id" uuid,
        ADD COLUMN "plan_ck" uuid,
        ADD COLUMN "task_id" uuid,
        ADD COLUMN "task_ck" uuid
    """
    )

    # bench_cursor
    await cur.execute(
        """
        ALTER TABLE "bench_cursor"    
        ADD COLUMN "title" jsonb,
        ADD COLUMN "started_at" timestamp,
        ADD COLUMN "seen_at" timestamp,
        ADD COLUMN "terminated_at" timestamp
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
