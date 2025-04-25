# This migration was automatically generated on 2025.04.25. Edit as needed.
import psycopg

ID = 12
VERSION = "2025.04.25.4"
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
        DROP COLUMN "interruption_bench_id",
        DROP COLUMN "interruption_id",
        DROP COLUMN "run_bench_id",
        DROP COLUMN "run_id",
        DROP COLUMN "runnable_bench_id",
        DROP COLUMN "runnable_ck",
        DROP COLUMN "runnable_id",
        DROP COLUMN "runnable_type",
        DROP COLUMN "value_packed"
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
