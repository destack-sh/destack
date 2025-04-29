# This migration was automatically generated on 2025.04.29. Edit as needed.
import psycopg

ID = 22
VERSION = "2025.04.29.1"
HAS_GLOBAL = True
HAS_REGIONAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_bench
    await cur.execute(
        """
        ALTER TABLE "bench_bench"    
        DROP COLUMN "text"
    """
    )

    # bench_user
    await cur.execute(
        """
        ALTER TABLE "bench_user"    
        DROP COLUMN "text"
    """
    )

    # bench_organization
    await cur.execute(
        """
        ALTER TABLE "bench_organization"    
        DROP COLUMN "text"
    """
    )

    # bench_bench
    await cur.execute(
        """
        ALTER TABLE "bench_bench"    
        ADD COLUMN "line" jsonb
    """
    )

    # bench_user
    await cur.execute(
        """
        ALTER TABLE "bench_user"    
        ADD COLUMN "line" jsonb
    """
    )

    # bench_organization
    await cur.execute(
        """
        ALTER TABLE "bench_organization"    
        ADD COLUMN "line" jsonb
    """
    )


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
        DROP COLUMN "text"
    """
    )

    # bench_tag
    await cur.execute('DROP TABLE "bench_tag"')

    # bench_transition
    await cur.execute(
        """
        ALTER TABLE "bench_transition"    
        DROP COLUMN "text"
    """
    )

    # bench_kit
    await cur.execute(
        """
        ALTER TABLE "bench_kit"    
        DROP COLUMN "parent_type",
        DROP COLUMN "text"
    """
    )

    # bench_trigger
    await cur.execute(
        """
        ALTER TABLE "bench_trigger"    
        DROP COLUMN "text"
    """
    )

    # bench_option
    await cur.execute(
        """
        ALTER TABLE "bench_option"    
        DROP COLUMN "text"
    """
    )

    # bench_field
    await cur.execute(
        """
        ALTER TABLE "bench_field"    
        DROP COLUMN "text"
    """
    )

    # bench_channel
    await cur.execute(
        """
        ALTER TABLE "bench_channel"    
        DROP COLUMN "page_id",
        DROP COLUMN "plan_ck",
        DROP COLUMN "plan_id"
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        DROP COLUMN "text"
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        DROP COLUMN "text"
    """
    )

    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        DROP COLUMN "text"
    """
    )

    # bench_interruption
    await cur.execute(
        """
        ALTER TABLE "bench_interruption"    
        DROP COLUMN "text"
    """
    )

    # bench_agent
    await cur.execute(
        """
        ALTER TABLE "bench_agent"    
        DROP COLUMN "parent_type",
        DROP COLUMN "text"
    """
    )

    # bench_space
    await cur.execute(
        """
        ALTER TABLE "bench_space"    
        DROP COLUMN "text"
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        ADD COLUMN "assigned_to_id" uuid,
        ADD COLUMN "assigned_to_ck" uuid,
        ADD COLUMN "assigned_to_type" smallint,
        ADD COLUMN "assigned_to_bench_id" uuid
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
