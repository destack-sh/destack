# This migration was automatically generated on 2025.04.25. Edit as needed.
import psycopg

ID = 10
VERSION = "2025.04.25.2"
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
    # bench_flow
    await cur.execute(
        """
        ALTER TABLE "bench_flow"    
        DROP COLUMN "max_retry_interval"
    """
    )

    # bench_action
    await cur.execute(
        """
        ALTER TABLE "bench_action"    
        DROP COLUMN "max_retry_interval"
    """
    )

    # bench_transition
    await cur.execute(
        """
        ALTER TABLE "bench_transition"    
        DROP COLUMN "max_retry_interval"
    """
    )

    # bench_agent
    await cur.execute(
        """
        ALTER TABLE "bench_agent"    
        DROP COLUMN "max_retry_interval"
    """
    )

    # bench_flow
    await cur.execute(
        """
        ALTER TABLE "bench_flow"    
        ADD COLUMN "model_id" varchar
    """
    )

    # bench_action
    await cur.execute(
        """
        ALTER TABLE "bench_action"    
        ADD COLUMN "model_id" varchar
    """
    )

    # bench_transition
    await cur.execute(
        """
        ALTER TABLE "bench_transition"    
        ADD COLUMN "model_id" varchar
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        ADD COLUMN "model_developer" smallint,
        ADD COLUMN "model_provider" smallint,
        ADD COLUMN "model_id" varchar
    """
    )

    # bench_message
    await cur.execute(
        """
        ALTER TABLE "bench_message"    
        ADD COLUMN "model_developer" smallint,
        ADD COLUMN "model_provider" smallint,
        ADD COLUMN "model_id" varchar
    """
    )

    # bench_agent
    await cur.execute(
        """
        ALTER TABLE "bench_agent"    
        ADD COLUMN "model_id" varchar
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        ADD COLUMN "model_developer" smallint,
        ADD COLUMN "model_provider" smallint,
        ADD COLUMN "model_id" varchar
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
