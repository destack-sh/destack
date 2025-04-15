# This migration was automatically generated on 2025.04.15. Edit as needed.
import psycopg

ID = 61
VERSION = "2025.04.15.1"
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
        ADD COLUMN "title" jsonb,
        ADD COLUMN "image_id" uuid,
        ADD COLUMN "image_ck" uuid,
        ADD COLUMN "image_bench_id" uuid,
        ADD COLUMN "image_url" varchar,
        ADD COLUMN "text" jsonb
    """
    )

    # bench_cursor
    await cur.execute(
        """
        ALTER TABLE "bench_cursor"    
        ADD COLUMN "filter" jsonb,
        ADD COLUMN "sort" jsonb[],
        ADD COLUMN "url" varchar
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
