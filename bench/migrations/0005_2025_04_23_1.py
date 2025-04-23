# This migration was automatically generated on 2025.04.23. Edit as needed.
import psycopg

ID = 5
VERSION = "2025.04.23.1"
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
        DROP COLUMN "default_agent_bench_id",
        DROP COLUMN "default_agent_ck",
        DROP COLUMN "default_agent_id"
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        ALTER COLUMN "thread_id" DROP NOT NULL,
        ALTER COLUMN "thread_ck" DROP NOT NULL
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
