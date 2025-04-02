# This migration was automatically generated on 2025.04.02. Edit as needed.
import psycopg

ID = 40
VERSION = "2025.04.02.1"
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
        DROP COLUMN "attempt"
    """
    )

    # bench_agent
    await cur.execute(
        """
        ALTER TABLE "bench_agent"    
        ADD COLUMN "scheduled_at" timestamp,
        ADD COLUMN "stopped_at" timestamp,
        ADD COLUMN "paused_at" timestamp,
        ADD COLUMN "resumed_at" timestamp,
        ADD COLUMN "error" jsonb,
        ADD COLUMN "session_id" uuid,
        ADD COLUMN "client_id" uuid,
        ADD COLUMN "computer_id" uuid,
        ADD COLUMN "user_id" uuid,
        ADD COLUMN "agent_id" uuid,
        ADD COLUMN "agent_ck" uuid,
        ADD COLUMN "agent_bench_id" uuid
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        ADD COLUMN "scheduled_at" timestamp,
        ADD COLUMN "stopped_at" timestamp,
        ADD COLUMN "interrupted_at" timestamp,
        ADD COLUMN "paused_at" timestamp,
        ADD COLUMN "resumed_at" timestamp,
        ADD COLUMN "interruption_id" uuid
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        ADD COLUMN "scheduled_at" timestamp,
        ADD COLUMN "paused_at" timestamp,
        ADD COLUMN "resumed_at" timestamp,
        ADD COLUMN "error" jsonb
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
