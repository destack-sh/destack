# This migration was automatically generated on 2025.03.19. Edit as needed.
import psycopg

ID = 13
VERSION = "2025.03.19.3"
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
    # bench_package
    await cur.execute(
        """
        ALTER TABLE "bench_package"    
        DROP COLUMN "main_channel_id",
        DROP COLUMN "main_flow_bench_id",
        DROP COLUMN "main_flow_id"
    """
    )

    # bench_trigger
    await cur.execute(
        """
        ALTER TABLE "bench_trigger"    
        DROP COLUMN "scope_bench_id",
        DROP COLUMN "scope_ck",
        DROP COLUMN "scope_id",
        DROP COLUMN "scope_type"
    """
    )

    # bench_message
    await cur.execute(
        """
        ALTER TABLE "bench_message"    
        DROP COLUMN "created_interruption_bench_id",
        DROP COLUMN "created_interruption_id",
        DROP COLUMN "created_run_base_id",
        DROP COLUMN "created_run_bench_id",
        DROP COLUMN "created_run_id",
        DROP COLUMN "created_thread_id",
        DROP COLUMN "run_base_id",
        DROP COLUMN "run_root_base_id",
        DROP COLUMN "run_root_id",
        DROP COLUMN "selection"
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        DROP COLUMN "page_id"
    """
    )

    # bench_interruption
    await cur.execute(
        """
        ALTER TABLE "bench_interruption"    
        DROP COLUMN "cancel_trigger_bench_id",
        DROP COLUMN "cancel_trigger_id",
        DROP COLUMN "complete_trigger_bench_id",
        DROP COLUMN "complete_trigger_id",
        DROP COLUMN "page_bench_id",
        DROP COLUMN "page_id"
    """
    )

    # bench_channel
    await cur.execute(
        """
        ALTER TABLE "bench_channel"    
        DROP COLUMN "type"
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        DROP COLUMN "run_base_id",
        DROP COLUMN "run_id",
        DROP COLUMN "run_root_base_id",
        DROP COLUMN "run_root_id",
        DROP COLUMN "type"
    """
    )

    # bench_identity
    await cur.execute(
        """
        ALTER TABLE "bench_identity"    
        DROP COLUMN "run_base_id",
        DROP COLUMN "run_bench_id",
        DROP COLUMN "run_id"
    """
    )

    # bench_package
    await cur.execute(
        """
        ALTER TABLE "bench_package"    
        ADD COLUMN "default_channel_id" uuid,
        ADD COLUMN "default_flow_id" uuid,
        ADD COLUMN "default_flow_bench_id" uuid,
        ADD COLUMN "default_identity_id" uuid
    """
    )

    # bench_channel
    await cur.execute(
        """
        ALTER TABLE "bench_channel"    
        ADD COLUMN "status" smallint NOT NULL DEFAULT 10,
        ADD COLUMN "closed_at" timestamp,
        ADD COLUMN "main_page_id" uuid,
        ADD COLUMN "main_plan_id" uuid,
        ADD COLUMN "main_plan_ck" uuid
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        ADD COLUMN "main_page_id" uuid,
        ADD COLUMN "main_plan_id" uuid,
        ADD COLUMN "main_plan_ck" uuid
    """
    )

    # bench_message
    await cur.execute(
        """
        ALTER TABLE "bench_message"    
        ADD COLUMN "interruption_id" uuid,
        ADD COLUMN "interruption_bench_id" uuid
    """
    )

    # bench_identity
    await cur.execute(
        """
        ALTER TABLE "bench_identity"    
        ADD COLUMN "implemented_by_id" uuid,
        ADD COLUMN "implemented_by_bench_id" uuid,
        ADD COLUMN "implemented_by_base_id" uuid
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        ADD COLUMN "nodes_id" uuid[],
        ADD COLUMN "nodes_ck" uuid[],
        ADD COLUMN "nodes_type" smallint[],
        ADD COLUMN "nodes_bench_id" uuid[],
        ADD COLUMN "nodes_base_id" uuid[]
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
