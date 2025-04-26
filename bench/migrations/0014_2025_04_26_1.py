# This migration was automatically generated on 2025.04.26. Edit as needed.
import psycopg

ID = 14
VERSION = "2025.04.26.1"
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
    # bench_link
    await cur.execute(
        """
        ALTER TABLE "bench_link"    
        DROP COLUMN "activated_at",
        DROP COLUMN "active_at",
        DROP COLUMN "deactivated_at",
        DROP COLUMN "decommissioned_at",
        DROP COLUMN "failed_at",
        DROP COLUMN "failed_attempts",
        DROP COLUMN "reset_at",
        DROP COLUMN "scaler_bench_id",
        DROP COLUMN "scaler_ck",
        DROP COLUMN "scaler_id",
        DROP COLUMN "status",
        DROP COLUMN "suspended_at"
    """
    )

    # bench_file
    await cur.execute(
        """
        ALTER TABLE "bench_file"    
        DROP COLUMN "activated_at",
        DROP COLUMN "active_at",
        DROP COLUMN "deactivated_at",
        DROP COLUMN "decommissioned_at",
        DROP COLUMN "failed_at",
        DROP COLUMN "failed_attempts",
        DROP COLUMN "reset_at",
        DROP COLUMN "scaler_bench_id",
        DROP COLUMN "scaler_ck",
        DROP COLUMN "scaler_id",
        DROP COLUMN "status",
        DROP COLUMN "suspended_at"
    """
    )

    # bench_stream
    await cur.execute(
        """
        ALTER TABLE "bench_stream"    
        DROP COLUMN "activated_at",
        DROP COLUMN "active_at",
        DROP COLUMN "deactivated_at",
        DROP COLUMN "decommissioned_at",
        DROP COLUMN "failed_at",
        DROP COLUMN "failed_attempts",
        DROP COLUMN "reset_at",
        DROP COLUMN "scaler_bench_id",
        DROP COLUMN "scaler_ck",
        DROP COLUMN "scaler_id",
        DROP COLUMN "status",
        DROP COLUMN "suspended_at"
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
