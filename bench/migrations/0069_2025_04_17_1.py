# This migration was automatically generated on 2025.04.17. Edit as needed.
import psycopg

ID = 69
VERSION = "2025.04.17.1"
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
        DROP COLUMN "main_handle_bench_id"
    """
    )

    # bench_organization
    await cur.execute(
        """
        ALTER TABLE "bench_organization"    
        DROP COLUMN "main_bench_id",
        DROP COLUMN "main_handle_bench_id",
        DROP COLUMN "main_handle_id"
    """
    )

    # bench_user
    await cur.execute(
        """
        ALTER TABLE "bench_user"    
        DROP COLUMN "main_bench_id",
        DROP COLUMN "main_cursor_bench_id",
        DROP COLUMN "main_cursor_id",
        DROP COLUMN "main_handle_bench_id"
    """
    )

    # bench_client
    await cur.execute(
        """
        ALTER TABLE "bench_client"    
        DROP COLUMN "main_cursor_bench_id",
        DROP COLUMN "main_cursor_id"
    """
    )

    # bench_bench
    await cur.execute(
        """
        ALTER TABLE "bench_bench"    
        ADD COLUMN "handle_bench_id" uuid
    """
    )

    # bench_user
    await cur.execute(
        """
        ALTER TABLE "bench_user"    
        ADD COLUMN "handle_bench_id" uuid,
        ADD COLUMN "cursor_id" uuid,
        ADD COLUMN "cursor_bench_id" uuid
    """
    )

    # bench_organization
    await cur.execute(
        """
        ALTER TABLE "bench_organization"    
        ADD COLUMN "handle_bench_id" uuid
    """
    )

    # bench_client
    await cur.execute(
        """
        ALTER TABLE "bench_client"    
        ADD COLUMN "cursor_id" uuid,
        ADD COLUMN "cursor_bench_id" uuid
    """
    )

    # bench_user
    await cur.execute(
        """
        ALTER TABLE "bench_user"    
        ADD COLUMN "bench_id" uuid REFERENCES bench_bench ON DELETE SET NULL
    """
    )

    # bench_organization
    await cur.execute(
        """
        ALTER TABLE "bench_organization"    
        ADD COLUMN "bench_id" uuid REFERENCES bench_bench ON DELETE SET NULL,
        ADD COLUMN "handle_id" uuid REFERENCES bench_handle ON DELETE SET NULL
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    # bench_space
    await cur.execute(
        """
        ALTER TABLE "bench_space"    
        DROP COLUMN "channel_bench_id",
        DROP COLUMN "channel_ck",
        DROP COLUMN "channel_id",
        DROP COLUMN "run_base_id",
        DROP COLUMN "run_bench_id",
        DROP COLUMN "run_id"
    """
    )

    # bench_channel
    await cur.execute(
        """
        ALTER TABLE "bench_channel"    
        DROP COLUMN "main_page_id",
        DROP COLUMN "main_plan_ck",
        DROP COLUMN "main_plan_id"
    """
    )

    # bench_page
    await cur.execute(
        """
        ALTER TABLE "bench_page"    
        DROP COLUMN "main_thread_bench_id",
        DROP COLUMN "main_thread_ck",
        DROP COLUMN "main_thread_id"
    """
    )

    # bench_application
    await cur.execute('DROP TABLE "bench_application"')

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        DROP COLUMN "main_page_id",
        DROP COLUMN "main_plan_ck",
        DROP COLUMN "main_plan_id"
    """
    )

    # bench_agent
    await cur.execute(
        """
        ALTER TABLE "bench_agent"    
        DROP COLUMN "main_cursor_id",
        DROP COLUMN "main_page_id",
        DROP COLUMN "main_plan_ck",
        DROP COLUMN "main_plan_id"
    """
    )

    # bench_page
    await cur.execute(
        """
        ALTER TABLE "bench_page"    
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid,
        ADD COLUMN "thread_bench_id" uuid
    """
    )

    # bench_channel
    await cur.execute(
        """
        ALTER TABLE "bench_channel"    
        ADD COLUMN "page_id" uuid,
        ADD COLUMN "plan_id" uuid,
        ADD COLUMN "plan_ck" uuid
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        ADD COLUMN "page_id" uuid,
        ADD COLUMN "plan_id" uuid,
        ADD COLUMN "plan_ck" uuid
    """
    )

    # bench_agent
    await cur.execute(
        """
        ALTER TABLE "bench_agent"    
        ADD COLUMN "page_id" uuid,
        ADD COLUMN "plan_id" uuid,
        ADD COLUMN "plan_ck" uuid,
        ADD COLUMN "cursor_id" uuid
    """
    )

    # bench_space
    await cur.execute(
        """
        ALTER TABLE "bench_space"    
        ADD COLUMN "container_id" uuid,
        ADD COLUMN "container_ck" uuid,
        ADD COLUMN "container_type" smallint,
        ADD COLUMN "container_bench_id" uuid,
        ADD COLUMN "container_base_id" uuid,
        ADD COLUMN "page_id" uuid,
        ADD COLUMN "page_bench_id" uuid,
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid,
        ADD COLUMN "thread_bench_id" uuid
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
