# This migration was automatically generated on 2025.04.09. Edit as needed.
import psycopg

ID = 48
VERSION = "2025.04.09.0"
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
        DROP COLUMN "paused_at",
        DROP COLUMN "resumed_at",
        DROP COLUMN "stopped_at"
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        DROP COLUMN "paused_at",
        DROP COLUMN "resumed_at",
        DROP COLUMN "stopped_at"
    """
    )

    # bench_channel
    await cur.execute(
        """
        ALTER TABLE "bench_channel"    
        DROP COLUMN "paused_at",
        DROP COLUMN "resumed_at",
        DROP COLUMN "stopped_at"
    """
    )

    # bench_agent
    await cur.execute(
        """
        ALTER TABLE "bench_agent"    
        DROP COLUMN "paused_at",
        DROP COLUMN "resumed_at",
        DROP COLUMN "stopped_at"
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        DROP COLUMN "paused_at",
        DROP COLUMN "resumed_at",
        DROP COLUMN "stopped_at"
    """
    )

    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        DROP COLUMN "paused_at",
        DROP COLUMN "resumed_at",
        DROP COLUMN "stopped_at"
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        DROP COLUMN "paused_at",
        DROP COLUMN "resumed_at",
        DROP COLUMN "stopped_at"
    """
    )

    # bench_channel
    await cur.execute(
        """
        ALTER TABLE "bench_channel"    
        ADD COLUMN "active_at" timestamp,
        ADD COLUMN "requested_stop_at" timestamp,
        ADD COLUMN "requested_pause_at" timestamp,
        ADD COLUMN "requested_resume_at" timestamp
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        ADD COLUMN "active_at" timestamp,
        ADD COLUMN "requested_stop_at" timestamp,
        ADD COLUMN "requested_pause_at" timestamp,
        ADD COLUMN "requested_resume_at" timestamp
    """
    )

    # bench_agent
    await cur.execute(
        """
        ALTER TABLE "bench_agent"    
        ADD COLUMN "active_at" timestamp,
        ADD COLUMN "requested_stop_at" timestamp,
        ADD COLUMN "requested_pause_at" timestamp,
        ADD COLUMN "requested_resume_at" timestamp
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        ADD COLUMN "active_at" timestamp,
        ADD COLUMN "requested_stop_at" timestamp,
        ADD COLUMN "requested_pause_at" timestamp,
        ADD COLUMN "requested_resume_at" timestamp
    """
    )

    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        ADD COLUMN "active_at" timestamp,
        ADD COLUMN "requested_stop_at" timestamp,
        ADD COLUMN "requested_pause_at" timestamp,
        ADD COLUMN "requested_resume_at" timestamp
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        ADD COLUMN "active_at" timestamp,
        ADD COLUMN "requested_stop_at" timestamp,
        ADD COLUMN "requested_pause_at" timestamp,
        ADD COLUMN "requested_resume_at" timestamp
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        ADD COLUMN "active_at" timestamp,
        ADD COLUMN "requested_stop_at" timestamp,
        ADD COLUMN "requested_pause_at" timestamp,
        ADD COLUMN "requested_resume_at" timestamp
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
