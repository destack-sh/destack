# This migration was automatically generated on 2025.03.31. Edit as needed.
import psycopg

ID = 34
VERSION = "2025.03.31.3"
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
    # bench_identity
    await cur.execute(
        """
        ALTER TABLE "bench_identity"    
        DROP COLUMN "default_flow_bench_id",
        DROP COLUMN "default_flow_id",
        ADD COLUMN "main_flow_id" uuid,
        ADD COLUMN "main_flow_bench_id" uuid
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
