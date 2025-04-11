# This migration was automatically generated on 2025.04.11. Edit as needed.
import psycopg

ID = 52
VERSION = "2025.04.11.1"
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
        ADD COLUMN "main_thread_id" uuid,
        ADD COLUMN "main_thread_ck" uuid,
        ADD COLUMN "main_thread_bench_id" uuid
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
