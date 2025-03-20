# This migration was automatically generated on 2025.03.20. Edit as needed.
import psycopg

ID = 17
VERSION = "2025.03.20.5"
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
        ALTER COLUMN "thread_id" SET NOT NULL,
        ALTER COLUMN "thread_ck" SET NOT NULL
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
