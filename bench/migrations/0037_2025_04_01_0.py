# This migration was automatically generated on 2025.04.01. Edit as needed.
import psycopg

ID = 37
VERSION = "2025.04.01.0"
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
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_file
    await cur.execute(
        """
        ALTER TABLE "bench_file"    
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_stream
    await cur.execute(
        """
        ALTER TABLE "bench_stream"    
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_computer
    await cur.execute(
        """
        ALTER TABLE "bench_computer"    
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_flow
    await cur.execute(
        """
        ALTER TABLE "bench_flow"    
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_choice
    await cur.execute(
        """
        ALTER TABLE "bench_choice"    
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_class
    await cur.execute(
        """
        ALTER TABLE "bench_class"    
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_tag
    await cur.execute(
        """
        ALTER TABLE "bench_tag"    
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_view
    await cur.execute(
        """
        ALTER TABLE "bench_view"    
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_role
    await cur.execute(
        """
        ALTER TABLE "bench_role"    
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_kit
    await cur.execute(
        """
        ALTER TABLE "bench_kit"    
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_database
    await cur.execute(
        """
        ALTER TABLE "bench_database"    
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_store
    await cur.execute(
        """
        ALTER TABLE "bench_store"    
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_team
    await cur.execute(
        """
        ALTER TABLE "bench_team"    
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_channel
    await cur.execute(
        """
        ALTER TABLE "bench_channel"    
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_secret
    await cur.execute(
        """
        ALTER TABLE "bench_secret"    
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_page
    await cur.execute(
        """
        ALTER TABLE "bench_page"    
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        DROP COLUMN "error",
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_application
    await cur.execute(
        """
        ALTER TABLE "bench_application"    
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id"
    """
    )

    # bench_agent
    await cur.execute(
        """
        ALTER TABLE "bench_agent"    
        DROP COLUMN "implemented_by_bench_id",
        DROP COLUMN "tags_id",
        DROP COLUMN "thread_ck",
        DROP COLUMN "thread_id",
        ADD COLUMN "status" smallint NOT NULL DEFAULT 1,
        ADD COLUMN "duration" interval,
        ADD COLUMN "started_at" timestamp,
        ADD COLUMN "terminated_at" timestamp,
        ADD COLUMN "interrupted_at" timestamp,
        ADD COLUMN "interruption_id" uuid
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        ADD COLUMN "interrupted_at" timestamp
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
