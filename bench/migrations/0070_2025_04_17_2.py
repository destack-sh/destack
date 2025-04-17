# This migration was automatically generated on 2025.04.17. Edit as needed.
import psycopg

ID = 70
VERSION = "2025.04.17.2"
HAS_GLOBAL = True
HAS_REGIONAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_bench
    await cur.execute(
        """
        ALTER TABLE "bench_bench"    
        DROP COLUMN "handle_bench_id"
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    # bench_space
    await cur.execute(
        """
        ALTER TABLE "bench_space"    
        ADD COLUMN "run_id" uuid,
        ADD COLUMN "run_bench_id" uuid,
        ADD COLUMN "run_base_id" uuid
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
