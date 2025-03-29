# This migration was automatically generated on 2025.03.29. Edit as needed.
import psycopg

ID = 29
VERSION = "2025.03.29.1"
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
        ADD COLUMN "claimed_by_id" uuid,
        ADD COLUMN "claimed_by_ck" uuid
    """
    )

    # bench_store
    await cur.execute(
        """
        ALTER TABLE "bench_store"    
        ADD COLUMN "claimed_by_id" uuid,
        ADD COLUMN "claimed_by_ck" uuid
    """
    )

    # bench_computer
    await cur.execute(
        """
        ALTER TABLE "bench_computer"    
        ADD COLUMN "claimed_by_id" uuid,
        ADD COLUMN "claimed_by_ck" uuid
    """
    )

    # bench_application
    await cur.execute(
        """
        ALTER TABLE "bench_application"    
        ADD COLUMN "claimed_by_id" uuid,
        ADD COLUMN "claimed_by_ck" uuid
    """
    )

    # bench_file
    await cur.execute(
        """
        ALTER TABLE "bench_file"    
        ADD COLUMN "claimed_by_id" uuid,
        ADD COLUMN "claimed_by_ck" uuid
    """
    )

    # bench_stream
    await cur.execute(
        """
        ALTER TABLE "bench_stream"    
        ADD COLUMN "claimed_by_id" uuid,
        ADD COLUMN "claimed_by_ck" uuid
    """
    )

    # bench_secret
    await cur.execute(
        """
        ALTER TABLE "bench_secret"    
        ADD COLUMN "claimed_by_id" uuid,
        ADD COLUMN "claimed_by_ck" uuid
    """
    )

    # bench_flow
    await cur.execute(
        """
        ALTER TABLE "bench_flow"    
        ADD COLUMN "claimed_by_id" uuid,
        ADD COLUMN "claimed_by_ck" uuid
    """
    )

    # bench_action
    await cur.execute(
        """
        ALTER TABLE "bench_action"    
        ADD COLUMN "claimed_by_id" uuid,
        ADD COLUMN "claimed_by_ck" uuid
    """
    )

    # bench_kit
    await cur.execute(
        """
        ALTER TABLE "bench_kit"    
        ADD COLUMN "claimed_by_id" uuid,
        ADD COLUMN "claimed_by_ck" uuid
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
