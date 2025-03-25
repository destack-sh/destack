# This migration was automatically generated on 2025.03.25. Edit as needed.
import psycopg

ID = 25
VERSION = "2025.03.25.2"
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
    # bench_claim
    await cur.execute(
        """
        ALTER TABLE "bench_claim"    
        DROP COLUMN "closed_at",
        DROP COLUMN "granted_at",
        ADD COLUMN "paused_at" timestamp,
        ADD COLUMN "resumed_at" timestamp,
        ADD COLUMN "terminated_at" timestamp,
        ALTER COLUMN "status" SET DEFAULT 1
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
