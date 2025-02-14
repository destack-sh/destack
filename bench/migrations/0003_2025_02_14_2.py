# This migration was automatically generated on 2025.02.14. Edit as needed.
import psycopg

ID = 3
VERSION = "2025.02.14.2"
HAS_GLOBAL = False
HAS_REGIONAL = False
HAS_LOCAL = True


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
    pass


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_trigger
    await cur.execute(
        """
        ALTER TABLE bench_trigger    
        ALTER COLUMN "scope_id" DROP NOT NULL,
        ALTER COLUMN "scope_ck" DROP NOT NULL,
        ALTER COLUMN "scope_type" DROP NOT NULL,
        ALTER COLUMN "scope_bench_id" DROP NOT NULL
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
