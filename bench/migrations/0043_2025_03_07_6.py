# This migration was automatically generated on 2025.03.07. Edit as needed.
import psycopg

ID = 43
VERSION = "2025.03.07.6"
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
    # bench_page
    await cur.execute(
        """
        ALTER TABLE "bench_page"    
        ADD COLUMN "parent_base_id" uuid
    """
    )

    # bench_trigger
    await cur.execute(
        """
        ALTER TABLE "bench_trigger"    
        ADD COLUMN "parent_base_id" uuid,
        ADD COLUMN "run_root_base_id" uuid,
        ADD COLUMN "run_base_id" uuid
    """
    )

    # bench_space
    await cur.execute(
        """
        ALTER TABLE "bench_space"    
        ADD COLUMN "run_base_id" uuid
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        ADD COLUMN "run_root_base_id" uuid,
        ADD COLUMN "run_base_id" uuid
    """
    )

    # bench_message
    await cur.execute(
        """
        ALTER TABLE "bench_message"    
        ADD COLUMN "run_root_base_id" uuid,
        ADD COLUMN "run_base_id" uuid,
        ADD COLUMN "created_run_base_id" uuid
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        ADD COLUMN "parent_base_id" uuid,
        ADD COLUMN "root_base_id" uuid,
        ADD COLUMN "incoming_base_id" uuid[]
    """
    )

    # bench_run_span
    await cur.execute(
        """
        ALTER TABLE "bench_run_span"    
        ADD COLUMN "parent_base_id" uuid,
        ADD COLUMN "root_base_id" uuid
    """
    )

    # bench_interruption
    await cur.execute(
        """
        ALTER TABLE "bench_interruption"    
        ADD COLUMN "parent_base_id" uuid,
        ADD COLUMN "root_base_id" uuid
    """
    )

    # bench_log
    await cur.execute(
        """
        ALTER TABLE "bench_log"    
        ADD COLUMN "parent_base_id" uuid
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        ADD COLUMN "parent_base_id" uuid,
        ADD COLUMN "implemented_by_base_id" uuid
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        ADD COLUMN "parent_base_id" uuid,
        ADD COLUMN "implemented_by_base_id" uuid
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
