# This migration was automatically generated on 2025.04.08. Edit as needed.
import psycopg

ID = 47
VERSION = "2025.04.08.0"
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
    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        ADD COLUMN "scheduled_at" timestamp,
        ADD COLUMN "stopped_at" timestamp,
        ADD COLUMN "paused_at" timestamp,
        ADD COLUMN "resumed_at" timestamp,
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
