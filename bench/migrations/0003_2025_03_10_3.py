# This migration was automatically generated on 2025.03.10. Edit as needed.
import psycopg

ID = 3
VERSION = "2025.03.10.3"
HAS_GLOBAL = True
HAS_REGIONAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_client
    await cur.execute(
        """
        ALTER TABLE "bench_client"    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    # bench_page
    await cur.execute(
        """
        ALTER TABLE "bench_page"    
        DROP COLUMN "name"
    """
    )

    # bench_role
    await cur.execute(
        """
        ALTER TABLE "bench_role"    
        DROP COLUMN "name"
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        DROP COLUMN "name"
    """
    )

    # bench_interruption
    await cur.execute(
        """
        ALTER TABLE "bench_interruption"    
        DROP COLUMN "title"
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        DROP COLUMN "name"
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        DROP COLUMN "name"
    """
    )

    # bench_package
    await cur.execute(
        """
        ALTER TABLE "bench_package"    
        ADD COLUMN "main_flow_id" uuid,
        ADD COLUMN "main_flow_bench_id" uuid
    """
    )

    # bench_page
    await cur.execute(
        """
        ALTER TABLE "bench_page"    
        ADD COLUMN "title" jsonb
    """
    )

    # bench_block
    await cur.execute(
        """
        ALTER TABLE "bench_block"    
        ADD COLUMN "name" varchar
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        ADD COLUMN "title" jsonb
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        ADD COLUMN "title" jsonb
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        ADD COLUMN "title" jsonb
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        DROP COLUMN "title"
    """
    )
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        ADD COLUMN "title" jsonb
    """
    )

    # bench_message
    await cur.execute(
        """
        ALTER TABLE "bench_message"    
        DROP COLUMN "title"
    """
    )
    await cur.execute(
        """
        ALTER TABLE "bench_message"    
        ADD COLUMN "title" jsonb
    """
    )

    # bench_notification
    await cur.execute(
        """
        ALTER TABLE "bench_notification"    
        DROP COLUMN "title"
    """
    )
    await cur.execute(
        """
        ALTER TABLE "bench_notification"    
        ADD COLUMN "title" jsonb
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
