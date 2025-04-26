# This migration was automatically generated on 2025.04.26. Edit as needed.
import psycopg

ID = 16
VERSION = "2025.04.26.3"
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
        DROP COLUMN "activated_at",
        DROP COLUMN "deactivated_at",
        DROP COLUMN "decommissioned_at",
        DROP COLUMN "reset_at",
        DROP COLUMN "suspended_at"
    """
    )

    # bench_computer
    await cur.execute(
        """
        ALTER TABLE "bench_computer"    
        DROP COLUMN "activated_at",
        DROP COLUMN "deactivated_at",
        DROP COLUMN "decommissioned_at",
        DROP COLUMN "reset_at",
        DROP COLUMN "suspended_at"
    """
    )

    # bench_store
    await cur.execute(
        """
        ALTER TABLE "bench_store"    
        DROP COLUMN "activated_at",
        DROP COLUMN "deactivated_at",
        DROP COLUMN "decommissioned_at",
        DROP COLUMN "reset_at",
        DROP COLUMN "suspended_at"
    """
    )

    # bench_scaler
    await cur.execute(
        """
        ALTER TABLE "bench_scaler"    
        ADD COLUMN "requested_activate_at" timestamp,
        ADD COLUMN "requested_deactivate_at" timestamp,
        ADD COLUMN "requested_reset_at" timestamp,
        ADD COLUMN "requested_suspend_at" timestamp,
        ADD COLUMN "requested_decommission_at" timestamp
    """
    )

    # bench_store
    await cur.execute(
        """
        ALTER TABLE "bench_store"    
        ADD COLUMN "requested_activate_at" timestamp,
        ADD COLUMN "requested_deactivate_at" timestamp,
        ADD COLUMN "requested_reset_at" timestamp,
        ADD COLUMN "requested_suspend_at" timestamp,
        ADD COLUMN "requested_decommission_at" timestamp
    """
    )

    # bench_computer
    await cur.execute(
        """
        ALTER TABLE "bench_computer"    
        ADD COLUMN "requested_activate_at" timestamp,
        ADD COLUMN "requested_deactivate_at" timestamp,
        ADD COLUMN "requested_reset_at" timestamp,
        ADD COLUMN "requested_suspend_at" timestamp,
        ADD COLUMN "requested_decommission_at" timestamp
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
