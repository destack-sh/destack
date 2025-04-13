# This migration was automatically generated on 2025.04.13. Edit as needed.
import psycopg

ID = 57
VERSION = "2025.04.13.1"
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
        DROP COLUMN "options"
    """
    )

    # bench_action
    await cur.execute(
        """
        ALTER TABLE "bench_action"    
        DROP COLUMN "options"
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        DROP COLUMN "options"
    """
    )

    # bench_interruption
    await cur.execute(
        """
        ALTER TABLE "bench_interruption"    
        DROP COLUMN "breakpoint_site"
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        DROP COLUMN "implemented_by_base_id",
        DROP COLUMN "implemented_by_id"
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        DROP COLUMN "implemented_by_base_id",
        DROP COLUMN "implemented_by_id"
    """
    )

    # bench_agent
    await cur.execute(
        """
        ALTER TABLE "bench_agent"    
        DROP COLUMN "implemented_by_base_id",
        DROP COLUMN "implemented_by_id"
    """
    )

    # bench_transition
    await cur.execute(
        """
        ALTER TABLE "bench_transition"    
        DROP COLUMN "options"
    """
    )

    # bench_flow
    await cur.execute(
        """
        ALTER TABLE "bench_flow"    
        ADD COLUMN "max_attempts" integer,
        ADD COLUMN "retry_interval" interval,
        ADD COLUMN "backoff" real,
        ADD COLUMN "max_retry_interval" interval,
        ADD COLUMN "model_developer" smallint,
        ADD COLUMN "model_provider" smallint,
        ADD COLUMN "model_family" smallint
    """
    )

    # bench_action
    await cur.execute(
        """
        ALTER TABLE "bench_action"    
        ADD COLUMN "max_attempts" integer,
        ADD COLUMN "retry_interval" interval,
        ADD COLUMN "backoff" real,
        ADD COLUMN "max_retry_interval" interval,
        ADD COLUMN "model_developer" smallint,
        ADD COLUMN "model_provider" smallint,
        ADD COLUMN "model_family" smallint
    """
    )

    # bench_transition
    await cur.execute(
        """
        ALTER TABLE "bench_transition"    
        ADD COLUMN "max_attempts" integer,
        ADD COLUMN "retry_interval" interval,
        ADD COLUMN "backoff" real,
        ADD COLUMN "max_retry_interval" interval,
        ADD COLUMN "model_developer" smallint,
        ADD COLUMN "model_provider" smallint,
        ADD COLUMN "model_family" smallint
    """
    )

    # bench_agent
    await cur.execute(
        """
        ALTER TABLE "bench_agent"    
        ADD COLUMN "max_attempts" integer,
        ADD COLUMN "retry_interval" interval,
        ADD COLUMN "backoff" real,
        ADD COLUMN "max_retry_interval" interval,
        ADD COLUMN "model_developer" smallint,
        ADD COLUMN "model_provider" smallint,
        ADD COLUMN "model_family" smallint
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
