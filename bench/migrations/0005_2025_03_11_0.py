# This migration was automatically generated on 2025.03.11. Edit as needed.
import psycopg

ID = 5
VERSION = "2025.03.11.0"
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
    # bench_flow
    await cur.execute(
        """
        ALTER TABLE "bench_flow"    
        ADD COLUMN "type" smallint NOT NULL DEFAULT 10
        """
    )

    # Remove default after adding the column
    await cur.execute(
        """
        ALTER TABLE "bench_flow"
        ALTER COLUMN "type" DROP DEFAULT
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
