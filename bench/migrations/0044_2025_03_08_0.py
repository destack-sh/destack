# This migration was automatically generated on 2025.03.08. Edit as needed.
import psycopg

ID = 44
VERSION = "2025.03.08.0"
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
    # bench_action
    await cur.execute(
        """
        ALTER TABLE "bench_action"    
        RENAME COLUMN "run_options" TO "options"
    """
    )

    # bench_flow
    await cur.execute(
        """
        ALTER TABLE "bench_flow"    
        RENAME COLUMN "run_options" TO "options"
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        ADD COLUMN "package_id" uuid NOT NULL
    """
    )

    # bench_message
    await cur.execute(
        """
        ALTER TABLE "bench_message"    
        ADD COLUMN "package_id" uuid NOT NULL
    """
    )

    # bench_notification
    await cur.execute(
        """
        ALTER TABLE "bench_notification"    
        ADD COLUMN "package_id" uuid NOT NULL
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        ADD COLUMN "order_key" varchar NOT NULL DEFAULT \'a0\'::character varying,
        ADD COLUMN "icon" jsonb,
        ADD COLUMN "text" jsonb,
        ADD COLUMN "definition_id" uuid,
        ADD COLUMN "thread_id" uuid,
        ADD COLUMN "tags_id" uuid[]
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
