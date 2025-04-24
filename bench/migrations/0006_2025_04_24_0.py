# This migration was automatically generated on 2025.04.24. Edit as needed.
import psycopg

ID = 6
VERSION = "2025.04.24.0"
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
    # bench_computer
    await cur.execute(
        """
        ALTER TABLE "bench_computer"    
        ADD COLUMN "image_id" varchar,
        ADD COLUMN "is_headless" boolean NOT NULL DEFAULT false
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
