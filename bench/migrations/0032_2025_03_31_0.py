# This migration was automatically generated on 2025.03.31. Edit as needed.
import psycopg

ID = 32
VERSION = "2025.03.31.0"
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
    # bench_page
    await cur.execute(
        """
        ALTER TABLE "bench_page"    
        ADD COLUMN "owned_by_id" uuid,
        ADD COLUMN "owned_by_type" smallint,
        ADD COLUMN "claimed_by_id" uuid,
        ADD COLUMN "claimed_by_ck" uuid
    """
    )

    # bench_flow
    await cur.execute(
        """
        ALTER TABLE "bench_flow"    
        ADD COLUMN "owned_by_id" uuid,
        ADD COLUMN "owned_by_type" smallint
    """
    )

    # bench_kit
    await cur.execute(
        """
        ALTER TABLE "bench_kit"    
        ADD COLUMN "owned_by_id" uuid,
        ADD COLUMN "owned_by_type" smallint
    """
    )

    # bench_view
    await cur.execute(
        """
        ALTER TABLE "bench_view"    
        ADD COLUMN "owned_by_id" uuid,
        ADD COLUMN "owned_by_type" smallint,
        ADD COLUMN "claimed_by_id" uuid,
        ADD COLUMN "claimed_by_ck" uuid
    """
    )

    # bench_database
    await cur.execute(
        """
        ALTER TABLE "bench_database"    
        ADD COLUMN "owned_by_id" uuid,
        ADD COLUMN "owned_by_type" smallint,
        ADD COLUMN "claimed_by_id" uuid,
        ADD COLUMN "claimed_by_ck" uuid
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        ADD COLUMN "claimed_by_id" uuid,
        ADD COLUMN "claimed_by_ck" uuid
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        ADD COLUMN "claimed_by_id" uuid,
        ADD COLUMN "claimed_by_ck" uuid
    """
    )

    # bench_claim
    await cur.execute(
        """
        ALTER TABLE "bench_claim"    
        ADD COLUMN "target_base_id" uuid,
        ADD COLUMN "target_template_base_id" uuid
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
