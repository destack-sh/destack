# This migration was automatically generated on 2025.04.29. Edit as needed.
import psycopg

ID = 21
VERSION = "2025.04.29.0"
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
    # bench_message
    await cur.execute(
        """
        ALTER TABLE "bench_message"    
        DROP COLUMN "failed_at",
        DROP COLUMN "read_at",
        DROP COLUMN "received_at",
        DROP COLUMN "sent_at",
        DROP COLUMN "status"
    """
    )

    # bench_agent
    await cur.execute(
        """
        ALTER TABLE "bench_agent"    
        DROP COLUMN "plan_ck",
        DROP COLUMN "plan_id"
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
