# This migration was automatically generated on 2025.03.10. Edit as needed.
import psycopg

ID = 2
VERSION = "2025.03.10.1"
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
    # bench_machine
    await cur.execute(
        """
        ALTER TABLE "bench_machine"    
        DROP COLUMN "occupancy"
    """
    )

    # bench_browser
    await cur.execute(
        """
        ALTER TABLE "bench_browser"    
        DROP COLUMN "occupancy"
    """
    )

    # bench_file
    await cur.execute(
        """
        ALTER TABLE "bench_file"    
        DROP COLUMN "occupancy"
    """
    )

    # bench_stream
    await cur.execute(
        """
        ALTER TABLE "bench_stream"    
        DROP COLUMN "occupancy"
    """
    )

    # bench_secret
    await cur.execute(
        """
        ALTER TABLE "bench_secret"    
        DROP COLUMN "occupancy"
    """
    )

    # bench_scaler
    await cur.execute(
        """
        ALTER TABLE "bench_scaler"    
        ADD COLUMN "owned_by_id" uuid,
        ADD COLUMN "owned_by_type" smallint,
        ADD COLUMN "scaler_id" uuid,
        ADD COLUMN "scaler_bench_id" uuid
    """
    )

    # bench_store
    await cur.execute(
        """
        ALTER TABLE "bench_store"    
        ADD COLUMN "owned_by_id" uuid,
        ADD COLUMN "owned_by_type" smallint,
        ADD COLUMN "scaler_id" uuid,
        ADD COLUMN "scaler_bench_id" uuid
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
