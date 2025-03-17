# This migration was automatically generated on 2025.03.17. Edit as needed.
import psycopg

ID = 7
VERSION = "2025.03.17.4"
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
    # bench_action
    await cur.execute(
        """
        ALTER TABLE "bench_action"    
        DROP COLUMN "selection"
    """
    )

    # bench_flow
    await cur.execute(
        """
        ALTER TABLE "bench_flow"    
        DROP COLUMN "selection"
    """
    )

    # bench_kit
    await cur.execute(
        """
        ALTER TABLE "bench_kit"    
        DROP COLUMN "selection"
    """
    )

    # bench_claim
    await cur.execute(
        """
        ALTER TABLE "bench_claim"    
        DROP COLUMN "resource_bench_id",
        DROP COLUMN "resource_id",
        DROP COLUMN "resource_selection",
        DROP COLUMN "resource_type",
        ADD COLUMN "target_id" uuid,
        ADD COLUMN "target_ck" uuid,
        ADD COLUMN "target_type" smallint,
        ADD COLUMN "target_bench_id" uuid
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
