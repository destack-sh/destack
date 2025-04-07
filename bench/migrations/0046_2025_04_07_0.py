# This migration was automatically generated on 2025.04.07. Edit as needed.
import psycopg

ID = 46
VERSION = "2025.04.07.0"
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
    # bench_channel
    await cur.execute(
        """
        ALTER TABLE "bench_channel"    
        DROP COLUMN "closed_at"
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        DROP COLUMN "closed_at"
    """
    )

    # bench_channel
    await cur.execute(
        """
        ALTER TABLE "bench_channel"    
        ADD COLUMN "duration" interval,
        ADD COLUMN "scheduled_at" timestamp,
        ADD COLUMN "started_at" timestamp,
        ADD COLUMN "stopped_at" timestamp,
        ADD COLUMN "interrupted_at" timestamp,
        ADD COLUMN "paused_at" timestamp,
        ADD COLUMN "resumed_at" timestamp,
        ADD COLUMN "terminated_at" timestamp,
        ADD COLUMN "error" jsonb,
        ADD COLUMN "interruption_id" uuid,
        ADD COLUMN "session_id" uuid,
        ADD COLUMN "client_id" uuid,
        ADD COLUMN "computer_id" uuid,
        ADD COLUMN "computer_ck" uuid,
        ADD COLUMN "user_id" uuid,
        ADD COLUMN "agent_id" uuid,
        ADD COLUMN "agent_ck" uuid,
        ADD COLUMN "agent_bench_id" uuid
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        ADD COLUMN "duration" interval,
        ADD COLUMN "scheduled_at" timestamp,
        ADD COLUMN "started_at" timestamp,
        ADD COLUMN "stopped_at" timestamp,
        ADD COLUMN "interrupted_at" timestamp,
        ADD COLUMN "paused_at" timestamp,
        ADD COLUMN "resumed_at" timestamp,
        ADD COLUMN "terminated_at" timestamp,
        ADD COLUMN "error" jsonb,
        ADD COLUMN "interruption_id" uuid
    """
    )

    # bench_channel
    await cur.execute(
        """
        ALTER TABLE "bench_channel"    
        ALTER COLUMN "status" SET DEFAULT 1
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        ALTER COLUMN "status" SET DEFAULT 1
    """
    )

    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        ALTER COLUMN "status" SET DEFAULT 4
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
