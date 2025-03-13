# This migration was automatically generated on 2025.03.13. Edit as needed.
import psycopg

ID = 9
VERSION = "2025.03.13.0"
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
    # bench_database
    await cur.execute(
        """
        ALTER TABLE "bench_database"    
        DROP COLUMN "template_ck"
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        DROP COLUMN "template_ck"
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        DROP COLUMN "template_ck"
    """
    )

    # bench_dependency
    await cur.execute(
        """
        ALTER TABLE "bench_dependency"    
        ADD COLUMN "dependency_type" smallint NOT NULL
    """
    )

    # bench_field
    await cur.execute(
        """
        ALTER TABLE "bench_field"    
        ADD COLUMN "oneof_ck" uuid
    """
    )

    # bench_action
    await cur.execute(
        """
        ALTER TABLE "bench_action"    
        ADD COLUMN "ck" uuid NOT NULL,
        ADD COLUMN "parent_ck" uuid,
        ADD COLUMN "tool_ck" uuid
    """
    )

    # bench_link
    await cur.execute(
        """
        ALTER TABLE "bench_link"    
        ADD COLUMN "parent_ck" uuid
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        ADD COLUMN "action_ck" uuid
    """
    )

    # bench_run_span
    await cur.execute(
        """
        ALTER TABLE "bench_run_span"    
        ADD COLUMN "action_ck" uuid
    """
    )

    # bench_interruption
    await cur.execute(
        """
        ALTER TABLE "bench_interruption"    
        ADD COLUMN "action_ck" uuid
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        ADD COLUMN "target_ck" uuid
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
