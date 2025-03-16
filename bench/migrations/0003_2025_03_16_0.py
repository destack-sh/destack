# This migration was automatically generated on 2025.03.16. Edit as needed.
import psycopg

ID = 3
VERSION = "2025.03.16.0"
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
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_store
    await cur.execute(
        """
        ALTER TABLE "bench_store"    
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_computer
    await cur.execute(
        """
        ALTER TABLE "bench_computer"    
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_browser
    await cur.execute(
        """
        ALTER TABLE "bench_browser"    
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_file
    await cur.execute(
        """
        ALTER TABLE "bench_file"    
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_stream
    await cur.execute(
        """
        ALTER TABLE "bench_stream"    
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_secret
    await cur.execute(
        """
        ALTER TABLE "bench_secret"    
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_page
    await cur.execute(
        """
        ALTER TABLE "bench_page"    
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_choice
    await cur.execute(
        """
        ALTER TABLE "bench_choice"    
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_class
    await cur.execute(
        """
        ALTER TABLE "bench_class"    
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_tag
    await cur.execute(
        """
        ALTER TABLE "bench_tag"    
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_flow
    await cur.execute(
        """
        ALTER TABLE "bench_flow"    
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_kit
    await cur.execute(
        """
        ALTER TABLE "bench_kit"    
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_view
    await cur.execute(
        """
        ALTER TABLE "bench_view"    
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_database
    await cur.execute(
        """
        ALTER TABLE "bench_database"    
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_role
    await cur.execute(
        """
        ALTER TABLE "bench_role"    
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_channel
    await cur.execute(
        """
        ALTER TABLE "bench_channel"    
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "thread_ck" uuid
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        ADD COLUMN "thread_id" uuid,
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
