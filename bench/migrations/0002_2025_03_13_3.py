# This migration was automatically generated on 2025.03.13. Edit as needed.
import psycopg

ID = 2
VERSION = "2025.03.13.3"
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
    # bench_scaler
    await cur.execute(
        """
        ALTER TABLE "bench_scaler"    
        DROP COLUMN "thread_id"
    """
    )

    # bench_store
    await cur.execute(
        """
        ALTER TABLE "bench_store"    
        DROP COLUMN "thread_id"
    """
    )

    # bench_computer
    await cur.execute(
        """
        ALTER TABLE "bench_computer"    
        DROP COLUMN "thread_id"
    """
    )

    # bench_browser
    await cur.execute(
        """
        ALTER TABLE "bench_browser"    
        DROP COLUMN "thread_id"
    """
    )

    # bench_file
    await cur.execute(
        """
        ALTER TABLE "bench_file"    
        DROP COLUMN "thread_id"
    """
    )

    # bench_stream
    await cur.execute(
        """
        ALTER TABLE "bench_stream"    
        DROP COLUMN "thread_id"
    """
    )

    # bench_secret
    await cur.execute(
        """
        ALTER TABLE "bench_secret"    
        DROP COLUMN "thread_id"
    """
    )

    # bench_page
    await cur.execute(
        """
        ALTER TABLE "bench_page"    
        DROP COLUMN "thread_id"
    """
    )

    # bench_choice
    await cur.execute(
        """
        ALTER TABLE "bench_choice"    
        DROP COLUMN "thread_id"
    """
    )

    # bench_class
    await cur.execute(
        """
        ALTER TABLE "bench_class"    
        DROP COLUMN "thread_id"
    """
    )

    # bench_tag
    await cur.execute(
        """
        ALTER TABLE "bench_tag"    
        DROP COLUMN "thread_id"
    """
    )

    # bench_flow
    await cur.execute(
        """
        ALTER TABLE "bench_flow"    
        DROP COLUMN "thread_id"
    """
    )

    # bench_kit
    await cur.execute(
        """
        ALTER TABLE "bench_kit"    
        DROP COLUMN "thread_id"
    """
    )

    # bench_view
    await cur.execute(
        """
        ALTER TABLE "bench_view"    
        DROP COLUMN "thread_id"
    """
    )

    # bench_database
    await cur.execute(
        """
        ALTER TABLE "bench_database"    
        DROP COLUMN "thread_id"
    """
    )

    # bench_channel
    await cur.execute(
        """
        ALTER TABLE "bench_channel"    
        DROP COLUMN "thread_id"
    """
    )

    # bench_role
    await cur.execute(
        """
        ALTER TABLE "bench_role"    
        DROP COLUMN "thread_id"
    """
    )

    # bench_space
    await cur.execute(
        """
        ALTER TABLE "bench_space"    
        DROP COLUMN "thread_bench_id",
        DROP COLUMN "thread_id"
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        DROP COLUMN "thread_id"
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        DROP COLUMN "thread_id"
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        ADD COLUMN "ck" uuid NOT NULL,
        ADD COLUMN "parent_ck" uuid,
        ADD COLUMN "template_id" uuid,
        ADD COLUMN "template_bench_id" uuid,
        ADD COLUMN "template_at" timestamp,
        ADD COLUMN "order_key" varchar,
        ADD COLUMN "icon" jsonb,
        ADD COLUMN "definition_id" uuid,
        ADD COLUMN "tags_id" uuid[],
        ADD COLUMN "scope_ck" uuid
    """
    )

    # bench_message
    await cur.execute(
        """
        ALTER TABLE "bench_message"    
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_notification
    await cur.execute(
        """
        ALTER TABLE "bench_notification"    
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        ADD COLUMN "thread_ck" uuid
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
